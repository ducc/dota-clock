use gtk4::gdk::Display;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, CssProvider, Image, Label, Orientation, Window};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::events::FlatEvent;
use crate::gsi::GameState;
use crate::icons;
use crate::patches::RecurringTiming;

static ICON_SIZE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(40);

const CSS: &str = "
window {
    background-color: transparent;
}
.bar {
    padding: 4px;
}
.event-widget {
    padding: 4px 8px;
    border-radius: 8px;
    background-color: rgba(10, 10, 20, 0.65);
    margin: 0 2px;
    transition: opacity 200ms;
}
.event-widget.urgent {
    background-color: rgba(180, 30, 30, 0.7);
}
.event-widget.warning {
    background-color: rgba(140, 100, 10, 0.65);
}
.event-widget.passed {
    opacity: 0.3;
}
.countdown {
    font-family: 'Courier New', monospace;
    font-size: 15px;
    font-weight: bold;
    color: #88cc44;
}
.countdown.urgent { color: #ff4444; }
.countdown.warning { color: #ffaa00; }
.countdown.soon { color: #88cc44; }
.countdown.passed { color: #666666; }
.countdown.dimmed { color: #888888; }
.event-name {
    font-size: 10px;
    color: #aaaaaa;
}
";

struct EventWidget {
    container: GtkBox,
    image: Image,
    countdown_label: Label,
    name_label: Label,
}

impl EventWidget {
    fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 2);
        container.add_css_class("event-widget");
        container.set_can_target(false);

        let image = Image::new();
        image.set_pixel_size(ICON_SIZE.load(std::sync::atomic::Ordering::Relaxed));
        image.set_can_target(false);
        container.append(&image);

        let countdown_label = Label::new(Some(""));
        countdown_label.add_css_class("countdown");
        countdown_label.set_can_target(false);
        container.append(&countdown_label);

        let name_label = Label::new(Some(""));
        name_label.add_css_class("event-name");
        name_label.set_can_target(false);
        container.append(&name_label);

        Self {
            container,
            image,
            countdown_label,
            name_label,
        }
    }

    fn update(&self, icon_data: &'static [u8], countdown: &str, name: &str, css_class: &str) {
        let bytes = glib::Bytes::from_static(icon_data);
        let texture = gtk4::gdk::Texture::from_bytes(&bytes).unwrap();
        self.image.set_paintable(Some(&texture));
        self.countdown_label.set_text(countdown);
        self.name_label.set_text(name);

        for cls in &["urgent", "warning", "soon", "passed", "dimmed"] {
            self.countdown_label.remove_css_class(cls);
            self.container.remove_css_class(cls);
        }
        self.countdown_label.add_css_class(css_class);
        self.container.add_css_class(css_class);
    }

    fn show(&self) {
        self.container.set_visible(true);
    }
    fn hide(&self) {
        self.container.set_visible(false);
    }
}

fn format_time(sec: i64) -> String {
    let neg = sec < 0;
    let abs = sec.unsigned_abs();
    format!(
        "{}{:}:{:02}",
        if neg { "-" } else { "" },
        abs / 60,
        abs % 60
    )
}

pub fn run(
    config: Config,
    shared_state: Arc<Mutex<GameState>>,
    events: Vec<FlatEvent>,
    recurring: Vec<RecurringTiming>,
) {
    ICON_SIZE.store(config.icon_size, std::sync::atomic::Ordering::Relaxed);

    let t0 = std::time::Instant::now();
    gtk4::init().unwrap();
    eprintln!("[TIMING] gtk4::init took {:?}", t0.elapsed());

    let css = CssProvider::new();
    css.load_from_string(CSS);
    gtk4::style_context_add_provider_for_display(
        &Display::default().unwrap(),
        &css,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let window = Window::builder().title("Dota Clock").build();

    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_exclusive_zone(0);

    match config.anchor.as_str() {
        "bottom-left" => {
            window.set_anchor(Edge::Bottom, true);
            window.set_anchor(Edge::Left, true);
        }
        "top-right" => {
            window.set_anchor(Edge::Top, true);
            window.set_anchor(Edge::Right, true);
        }
        "top-left" => {
            window.set_anchor(Edge::Top, true);
            window.set_anchor(Edge::Left, true);
        }
        _ => {
            window.set_anchor(Edge::Bottom, true);
            window.set_anchor(Edge::Right, true);
        }
    }

    window.set_margin(Edge::Bottom, config.margin_bottom);
    window.set_margin(Edge::Right, config.margin_right);
    window.set_margin(Edge::Top, config.margin_top);
    window.set_margin(Edge::Left, config.margin_left);
    window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
    window.set_can_target(false);

    window.connect_realize(|win| {
        if let Some(surface) = win.surface() {
            let empty_region = gtk4::cairo::Region::create();
            surface.set_input_region(&empty_region);
        }
    });

    let hbox = GtkBox::new(Orientation::Horizontal, 0);
    hbox.add_css_class("bar");
    hbox.set_valign(Align::Center);
    hbox.set_can_target(false);

    let mut recurring_widgets: Vec<EventWidget> = Vec::new();
    for _ in &recurring {
        let w = EventWidget::new();
        w.hide();
        hbox.append(&w.container);
        recurring_widgets.push(w);
    }

    let max_visible = config.max_icons;
    let mut event_widgets: Vec<EventWidget> = Vec::new();
    for _ in 0..max_visible {
        let w = EventWidget::new();
        hbox.append(&w.container);
        w.hide();
        event_widgets.push(w);
    }

    window.set_child(Some(&hbox));
    window.present();

    let events = Arc::new(events);
    let shared = shared_state;
    let last_display_clock = std::cell::Cell::new(i64::MIN);
    let anchor_clock = std::cell::Cell::new(0i64);
    let anchor_local_ms = std::cell::Cell::new(0u64);

    glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
        let gs = shared.lock().unwrap().clone();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        if gs.received_at_ms > 0 {
            let elapsed_ms = now_ms.saturating_sub(anchor_local_ms.get());
            let projected = if anchor_local_ms.get() > 0 {
                anchor_clock.get() + (elapsed_ms / 1000) as i64
            } else {
                i64::MIN
            };
            let drift = (gs.clock_time - projected).abs();
            if drift > 2 || anchor_local_ms.get() == 0 || gs.paused {
                anchor_clock.set(gs.clock_time);
                anchor_local_ms.set(gs.received_at_ms.saturating_sub(gs.subsecond_ms));
            }
        }

        let clock = if anchor_local_ms.get() > 0 && !gs.paused {
            let elapsed_ms = now_ms.saturating_sub(anchor_local_ms.get());
            anchor_clock.get() + (elapsed_ms / 1000) as i64
        } else {
            gs.clock_time
        };

        hbox.set_visible(gs.in_game());
        if !gs.in_game() {
            return glib::ControlFlow::Continue;
        }

        if clock == last_display_clock.get() {
            return glib::ControlFlow::Continue;
        }
        last_display_clock.set(clock);

        let sec_in_min = ((clock % 60) + 60) % 60;
        for (i, timing) in recurring.iter().enumerate() {
            let widget = &recurring_widgets[i];
            if clock < 0 {
                widget.hide();
                continue;
            }

            // Find nearest target within this minute
            let mut best_diff = i64::MAX;
            for &target in &timing.targets {
                let diff = target - sec_in_min;
                // Consider wrap-around to next minute
                let candidates = [diff, diff + 60];
                for d in candidates {
                    if d.abs() < best_diff.abs() {
                        best_diff = d;
                    }
                }
            }

            if best_diff >= 0 && best_diff <= timing.warn_window {
                let (text, css) = if best_diff == 0 {
                    ("NOW!".to_string(), "urgent")
                } else if best_diff <= timing.active_window {
                    (format!("{}s", best_diff), "urgent")
                } else if best_diff <= timing.warn_window / 2 {
                    (format!("{}s", best_diff), "warning")
                } else {
                    (format!("{}s", best_diff), "soon")
                };
                widget.update(icons::bytes(timing.icon_file), &text, timing.name, css);
                widget.show();
            } else if best_diff < 0 && best_diff.abs() <= timing.active_window {
                widget.update(
                    icons::bytes(timing.icon_file),
                    "NOW!",
                    timing.name,
                    "urgent",
                );
                widget.show();
            } else {
                widget.hide();
            }
        }

        let mut visible_events: Vec<&FlatEvent> = Vec::new();
        for ev in events.iter() {
            if ev.time < clock - 3 {
                continue;
            }
            if visible_events.len() >= max_visible {
                break;
            }
            if ev.time > clock + 300 {
                break;
            }
            visible_events.push(ev);
        }

        for (i, widget) in event_widgets.iter().enumerate() {
            if let Some(ev) = visible_events.get(i) {
                let diff = ev.time - clock;
                let (countdown_text, css_class) = if diff < 0 {
                    ("PAST".to_string(), "passed")
                } else if diff == 0 {
                    ("NOW!".to_string(), "urgent")
                } else if diff <= 10 {
                    (format!("{}s", diff), "urgent")
                } else if diff <= 30 {
                    (format!("{}s", diff), "warning")
                } else if diff <= 60 {
                    (format!("{}s", diff), "soon")
                } else {
                    (format_time(ev.time), "dimmed")
                };

                widget.update(icons::bytes(ev.icon_file), &countdown_text, ev.name, css_class);
                widget.show();
            } else {
                widget.hide();
            }
        }

        glib::ControlFlow::Continue
    });

    glib::MainLoop::new(None, false).run();
}
