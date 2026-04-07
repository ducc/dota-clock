use crate::events::FlatEvent;
use crate::gsi::GameState;
use crate::patches::RecurringTiming;

#[derive(Clone, Debug)]
pub enum Urgency {
    Urgent,
    Warning,
    Soon,
    Passed,
    Dimmed,
}

#[derive(Clone, Debug)]
pub struct DisplayItem {
    pub icon_file: &'static str,
    pub name: &'static str,
    pub text: String,
    pub urgency: Urgency,
}

pub struct DisplayFrame {
    pub visible: bool,
    pub recurring: Vec<Option<DisplayItem>>,
    pub events: Vec<DisplayItem>,
}

pub struct ClockState {
    anchor_clock: i64,
    anchor_local_ms: u64,
    last_clock: i64,
}

impl ClockState {
    pub fn new() -> Self {
        Self {
            anchor_clock: 0,
            anchor_local_ms: 0,
            last_clock: i64::MIN,
        }
    }

    pub fn tick(
        &mut self,
        gs: &GameState,
        events: &[FlatEvent],
        recurring: &[RecurringTiming],
        max_icons: usize,
    ) -> Option<DisplayFrame> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        if gs.received_at_ms > 0 {
            let elapsed_ms = now_ms.saturating_sub(self.anchor_local_ms);
            let projected = if self.anchor_local_ms > 0 {
                self.anchor_clock + (elapsed_ms / 1000) as i64
            } else {
                i64::MIN
            };
            let drift = (gs.clock_time - projected).abs();
            if drift > 2 || self.anchor_local_ms == 0 || gs.paused {
                self.anchor_clock = gs.clock_time;
                self.anchor_local_ms = gs.received_at_ms.saturating_sub(gs.subsecond_ms);
            }
        }

        let clock = if self.anchor_local_ms > 0 && !gs.paused {
            let elapsed_ms = now_ms.saturating_sub(self.anchor_local_ms);
            self.anchor_clock + (elapsed_ms / 1000) as i64
        } else {
            gs.clock_time
        };

        if !gs.in_game() {
            return Some(DisplayFrame {
                visible: false,
                recurring: Vec::new(),
                events: Vec::new(),
            });
        }

        if clock == self.last_clock {
            return None;
        }
        self.last_clock = clock;

        let recurring_items = compute_recurring(clock, recurring);
        let event_items = compute_events(clock, events, max_icons);

        Some(DisplayFrame {
            visible: true,
            recurring: recurring_items,
            events: event_items,
        })
    }
}

fn compute_recurring(clock: i64, timings: &[RecurringTiming]) -> Vec<Option<DisplayItem>> {
    let sec_in_min = ((clock % 60) + 60) % 60;

    timings
        .iter()
        .map(|timing| {
            if clock < 0 {
                return None;
            }

            let mut best_diff = i64::MAX;
            for &target in &timing.targets {
                let diff = target - sec_in_min;
                for d in [diff, diff + 60] {
                    if d.abs() < best_diff.abs() {
                        best_diff = d;
                    }
                }
            }

            if best_diff >= 0 && best_diff <= timing.warn_window {
                let (text, urgency) = if best_diff == 0 {
                    ("NOW!".to_string(), Urgency::Urgent)
                } else if best_diff <= timing.active_window {
                    (format!("{}s", best_diff), Urgency::Urgent)
                } else if best_diff <= timing.warn_window / 2 {
                    (format!("{}s", best_diff), Urgency::Warning)
                } else {
                    (format!("{}s", best_diff), Urgency::Soon)
                };
                Some(DisplayItem {
                    icon_file: timing.icon_file,
                    name: timing.name,
                    text,
                    urgency,
                })
            } else if best_diff < 0 && best_diff.abs() <= timing.active_window {
                Some(DisplayItem {
                    icon_file: timing.icon_file,
                    name: timing.name,
                    text: "NOW!".to_string(),
                    urgency: Urgency::Urgent,
                })
            } else {
                None
            }
        })
        .collect()
}

fn compute_events(clock: i64, events: &[FlatEvent], max_icons: usize) -> Vec<DisplayItem> {
    let mut out = Vec::new();
    for ev in events {
        if ev.time < clock - 3 {
            continue;
        }
        if out.len() >= max_icons {
            break;
        }
        if ev.time > clock + 300 {
            break;
        }

        let diff = ev.time - clock;
        let (text, urgency) = if diff < 0 {
            ("PAST".to_string(), Urgency::Passed)
        } else if diff == 0 {
            ("NOW!".to_string(), Urgency::Urgent)
        } else if diff <= 10 {
            (format!("{}s", diff), Urgency::Urgent)
        } else if diff <= 30 {
            (format!("{}s", diff), Urgency::Warning)
        } else if diff <= 60 {
            (format!("{}s", diff), Urgency::Soon)
        } else {
            (format_time(ev.time), Urgency::Dimmed)
        };

        out.push(DisplayItem {
            icon_file: ev.icon_file,
            name: ev.name,
            text,
            urgency,
        });
    }
    out
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
