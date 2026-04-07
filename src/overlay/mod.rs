#[cfg(target_os = "linux")]
mod wayland;
#[cfg(target_os = "windows")]
mod windows;

use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::events::FlatEvent;
use crate::gsi::GameState;
use crate::patches::RecurringTiming;

pub fn run(
    config: Config,
    shared_state: Arc<Mutex<GameState>>,
    events: Vec<FlatEvent>,
    recurring: Vec<RecurringTiming>,
) {
    #[cfg(target_os = "linux")]
    wayland::run(config, shared_state, events, recurring);

    #[cfg(target_os = "windows")]
    windows::run(config, shared_state, events, recurring);

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    compile_error!("Unsupported platform — only Linux (Wayland) and Windows are supported");
}
