use gtk4::gdk::Display;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, CssProvider, Image, Label, Orientation, Window};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::sync::{Arc, Mutex};

use crate::clock::{ClockState, DisplayFrame, Urgency};
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

    fn apply(&self, icon_file: &str, countdown: &str, name: &str, urgency: &Urgency) {
        let bytes = glib::Bytes::from_static(icons::bytes(icon_file));
        let texture = gtk4::gdk::Texture::from_bytes(&bytes).unwrap();
        self.image.set_paintable(Some(&texture));
        self.countdown_label.set_text(countdown);
        self.name_label.set_text(name);

        let css_class = urgency_css(urgency);
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

fn urgency_css(u: &Urgency) -> &'static str {
    match u {
        Urgency::Urgent => "urgent",
        Urgency::Warning => "warning",
        Urgency::Soon => "soon",
        Urgency::Passed => "passed",
        Urgency::Dimmed => "dimmed",
    }
}

fn render_frame(
    frame: &DisplayFrame,
    hbox: &GtkBox,
    recurring_widgets: &[EventWidget],
    event_widgets: &[EventWidget],
) {
    hbox.set_visible(frame.visible);
    if !frame.visible {
        return;
    }

    for (i, widget) in recurring_widgets.iter().enumerate() {
        if let Some(Some(item)) = frame.recurring.get(i) {
            widget.apply(item.icon_file, &item.text, item.name, &item.urgency);
            widget.show();
        } else {
            widget.hide();
        }
    }

    for (i, widget) in event_widgets.iter().enumerate() {
        if let Some(item) = frame.events.get(i) {
            widget.apply(item.icon_file, &item.text, item.name, &item.urgency);
            widget.show();
        } else {
            widget.hide();
        }
    }
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

    let orientation = if config.vertical {
        Orientation::Vertical
    } else {
        Orientation::Horizontal
    };
    let hbox = GtkBox::new(orientation, 0);
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
    let mut clock_state = ClockState::new();

    glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
        let gs = shared_state.lock().unwrap().clone();

        if let Some(frame) = clock_state.tick(&gs, &events, &recurring, max_visible) {
            render_frame(&frame, &hbox, &recurring_widgets, &event_widgets);
        }

        glib::ControlFlow::Continue
    });

    glib::MainLoop::new(None, false).run();
}
