use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_anchor")]
    pub anchor: String,
    #[serde(default = "default_margin_bottom")]
    pub margin_bottom: i32,
    #[serde(default = "default_margin_right")]
    pub margin_right: i32,
    #[serde(default)]
    pub margin_top: i32,
    #[serde(default)]
    pub margin_left: i32,
    #[serde(default = "default_icon_size")]
    pub icon_size: i32,
    #[serde(default = "default_max_icons")]
    pub max_icons: usize,
}

fn default_anchor() -> String {
    "bottom-right".to_string()
}
fn default_margin_bottom() -> i32 {
    10
}
fn default_margin_right() -> i32 {
    470
}
fn default_icon_size() -> i32 {
    40
}
fn default_max_icons() -> usize {
    10
}

impl Default for Config {
    fn default() -> Self {
        Self {
            anchor: default_anchor(),
            margin_bottom: default_margin_bottom(),
            margin_right: default_margin_right(),
            margin_top: 0,
            margin_left: 0,
            icon_size: default_icon_size(),
            max_icons: default_max_icons(),
        }
    }
}

pub fn load() -> Config {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("~/.config"))
        .join("dota-clock");

    let config_path = config_dir.join("config.toml");

    if config_path.exists() {
        match std::fs::read_to_string(&config_path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => {
                    println!("Loaded config from {}", config_path.display());
                    return config;
                }
                Err(e) => eprintln!("Config parse error: {e}, using defaults"),
            },
            Err(e) => eprintln!("Config read error: {e}, using defaults"),
        }
    } else {
        let _ = std::fs::create_dir_all(&config_dir);
        let default = Config::default();
        let toml_str = toml::to_string_pretty(&default).unwrap();
        if std::fs::write(&config_path, &toml_str).is_ok() {
            println!("Created default config at {}", config_path.display());
        }
    }

    Config::default()
}
