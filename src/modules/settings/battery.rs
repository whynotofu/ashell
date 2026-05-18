use crate::{
    components::{format_indicator, icons::StaticIcon, quick_setting_button},
    config::SettingsFormat,
    modules::settings::state::{
        BatteryProtectionLastState, BatteryProtectionMode, BatteryProtectionState,
    },
};
use iced::{Element, widget::text};

#[derive(Debug, Clone)]
pub enum Message {
    NextBatteryProtectionMode,
}

pub enum Action {
    None,
}

pub struct BatterySettings {
    charge: u8,
    pub state: BatteryProtectionState,
}

impl BatterySettings {
    pub fn new(state: BatteryProtectionState) -> Self {
        Self { charge: 0, state }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::NextBatteryProtectionMode => {
                self.state.mode = match self.state.mode {
                    BatteryProtectionMode::Off => BatteryProtectionMode::On,
                    BatteryProtectionMode::On => BatteryProtectionMode::StationaryMode,
                    BatteryProtectionMode::StationaryMode => BatteryProtectionMode::Off,
                };
                Action::None
            }
        }
    }

    pub fn battery_indicator<'a>(&self) -> Option<Element<'a, Message>> {
        Some(
            format_indicator(
                SettingsFormat::IconAndPercentage,
                StaticIcon::BatteryCharging,
                text("50%").into(),
                crate::utils::IndicatorState::Normal,
            )
            .into(),
        )
    }

    pub fn quick_setting_button<'a>(
        &'a self,
    ) -> Option<(Element<'a, Message>, Option<Element<'a, Message>>)> {
        let (mode, active) = match self.state.mode {
            BatteryProtectionMode::Off => ("Off", false),
            BatteryProtectionMode::On => ("On", true),
            BatteryProtectionMode::StationaryMode => ("Stationary Mode", true),
        };
        Some((
            quick_setting_button(
                StaticIcon::BatteryCharging,
                "Battery Protection".to_string(),
                Some(mode.to_string()),
                active,
                Message::NextBatteryProtectionMode,
                None,
                None,
            ),
            None,
        ))
    }

    fn thresholds(mode: BatteryProtectionMode) -> (u32, u32) {
        match mode {
            BatteryProtectionMode::Off => (95, 100),
            BatteryProtectionMode::On => (75, 80),
            BatteryProtectionMode::StationaryMode => (40, 60),
        }
    }
}
