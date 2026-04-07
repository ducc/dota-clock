use axum::{Router, http::StatusCode, routing::post};
use std::sync::{Arc, Mutex};

use crate::gsi::{GameState, GsiPayload};

pub fn spawn(shared_state: Arc<Mutex<GameState>>) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let app = Router::new().route(
                "/gsi",
                post(move |body: String| {
                    let shared = shared_state.clone();
                    async move {
                        let payload: GsiPayload = match serde_json::from_str(&body) {
                            Ok(p) => p,
                            Err(e) => {
                                eprintln!("[GSI] Parse error: {e}");
                                return StatusCode::BAD_REQUEST;
                            }
                        };

                        if let Some(map) = payload.map {
                            println!(
                                "[GSI] clock={} game_time={:.3} state={} daytime={}",
                                map.clock_time, map.game_time, map.game_state, map.daytime
                            );

                            let gs = GameState::from_payload(map, payload.previously.as_ref());

                            if let Ok(mut s) = shared.lock() {
                                *s = gs;
                            }
                        }

                        StatusCode::OK
                    }
                }),
            );

            println!("GSI endpoint: http://localhost:3000/gsi");

            let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
}
