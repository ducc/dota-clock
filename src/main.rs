mod clock;
mod config;
mod events;
mod gsi;
mod icons;
mod overlay;
mod patches;
mod server;

use std::sync::{Arc, Mutex};

fn main() {
    #[cfg(target_os = "linux")]
    unsafe {
        std::env::set_var("GSETTINGS_BACKEND", "memory");
        std::env::set_var("GTK_USE_PORTAL", "0");
        std::env::set_var("GTK_A11Y", "none");
    }

    let shared_state = Arc::new(Mutex::new(gsi::GameState::default()));
    let config = config::load();

    let patch = patches::latest();
    println!("Using patch {}", patch.version());
    let events = events::generate(patch.events());
    let recurring = patch.recurring_timings();

    server::spawn(shared_state.clone());
    overlay::run(config, shared_state, events, recurring);
}
