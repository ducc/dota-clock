mod v7_41;

use crate::events::EventDef;

/// Recurring per-minute timing (e.g. pull at :15/:45, stack at :53)
#[derive(Clone)]
pub struct RecurringTiming {
    pub name: &'static str,
    pub icon_file: &'static str,
    /// Seconds within each minute to trigger (e.g. [15, 45] for pulls)
    pub targets: Vec<i64>,
    /// Show countdown this many seconds before each target
    pub warn_window: i64,
    /// How many seconds the "NOW" window lasts after the target
    pub active_window: i64,
}

pub trait Patch {
    fn version(&self) -> &'static str;
    fn events(&self) -> Vec<EventDef>;
    fn recurring_timings(&self) -> Vec<RecurringTiming>;
}

pub fn latest() -> Box<dyn Patch> {
    Box::new(v7_41::Patch741)
}
