use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameState {
    pub clock_time: i64,
    pub game_time: f64,
    pub daytime: bool,
    pub paused: bool,
    pub game_state: String,
    pub roshan_state: String,
    pub roshan_state_end_seconds: i64,
    #[serde(skip)]
    pub received_at_ms: u64,
    #[serde(skip)]
    pub subsecond_ms: u64,
    #[serde(skip)]
    pub clock_rate: f64,
}

#[derive(Debug, Deserialize)]
pub struct GsiPayload {
    pub map: Option<GsiMap>,
    pub previously: Option<GsiPreviously>,
}

#[derive(Debug, Deserialize)]
pub struct GsiPreviously {
    pub map: Option<GsiPrevMap>,
}

#[derive(Debug, Deserialize)]
pub struct GsiPrevMap {
    #[serde(default)]
    pub clock_time: Option<i64>,
    #[serde(default)]
    pub game_time: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct GsiMap {
    #[serde(default)]
    pub clock_time: i64,
    #[serde(default)]
    pub game_time: f64,
    #[serde(default)]
    pub daytime: bool,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub game_state: String,
    #[serde(default)]
    pub roshan_state: String,
    #[serde(default)]
    pub roshan_state_end_seconds: i64,
}

impl GameState {
    pub fn from_payload(map: GsiMap, previously: Option<&GsiPreviously>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let frac = map.game_time - map.game_time.floor();
        let subsecond_ms = (frac * 1000.0) as u64;

        let clock_rate = if let Some(prev) = previously {
            if let Some(prev_map) = &prev.map {
                let prev_gt = prev_map.game_time.unwrap_or(map.game_time);
                let prev_ct = prev_map.clock_time.unwrap_or(map.clock_time);
                let dt_game = map.game_time - prev_gt;
                let dt_clock = (map.clock_time - prev_ct) as f64;
                if dt_game > 0.1 {
                    dt_clock / dt_game
                } else {
                    1.0
                }
            } else {
                1.0
            }
        } else {
            1.0
        };

        Self {
            clock_time: map.clock_time,
            game_time: map.game_time,
            daytime: map.daytime,
            paused: map.paused,
            game_state: map.game_state,
            roshan_state: map.roshan_state,
            roshan_state_end_seconds: map.roshan_state_end_seconds,
            received_at_ms: now,
            subsecond_ms,
            clock_rate,
        }
    }

    pub fn in_game(&self) -> bool {
        matches!(
            self.game_state.as_str(),
            "DOTA_GAMERULES_STATE_PRE_GAME"
                | "DOTA_GAMERULES_STATE_GAME_IN_PROGRESS"
                | "DOTA_GAMERULES_STATE_STRATEGY_TIME"
                | "DOTA_GAMERULES_STATE_HERO_SELECTION"
        )
    }
}
