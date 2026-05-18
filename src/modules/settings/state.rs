use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const STATE_FILE_PATH: &str = "~/.local/state/seashell.toml";

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum BatteryProtectionMode {
    Off,
    On,
    StationaryMode,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum BatteryProtectionLastState {
    Charging,
    Charged,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct BatteryProtectionState {
    pub mode: BatteryProtectionMode,
    pub last_state: BatteryProtectionLastState,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct AudioState {
    pub volume: u32,
    pub microphone_volume: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct State {
    pub battery_protection: BatteryProtectionState,
    pub audio_state: Option<AudioState>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            battery_protection: BatteryProtectionState {
                mode: BatteryProtectionMode::On,
                last_state: BatteryProtectionLastState::Charged,
            },
            audio_state: None,
        }
    }
}

impl State {
    pub fn load() -> Self {
        let file = Self::file();
        if file.exists()
            && let Ok(content) = std::fs::read_to_string(file)
            && let Ok(state) = toml::from_str(&content)
        {
            state
        } else {
            Self::default()
        }
    }

    pub fn save(&self) {
        let file = Self::file();
        if let Some(parent) = file.parent() {
            if std::fs::create_dir_all(parent).is_ok()
                && let Ok(content) = toml::to_string(self)
            {
                let _ = std::fs::write(file, content);
            }
        }
    }

    fn file() -> PathBuf {
        PathBuf::from(shellexpand::tilde(STATE_FILE_PATH).to_string())
    }
}
