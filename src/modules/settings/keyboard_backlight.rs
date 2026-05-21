use crate::components::{icons::StaticIcon, quick_setting_button};
use iced::Element;

#[derive(Debug, Clone)]
pub enum Message {
    NextKeyboardBacklightLevel,
}

pub enum Action {
    None,
}

#[derive(Debug, Clone)]
pub enum KeyboardBacklightLevel {
    Off,
    Dim,
    Medium,
    Bright,
}

pub struct KeyboardBacklightSettings {
    level: KeyboardBacklightLevel,
}

impl KeyboardBacklightSettings {
    pub fn new() -> Self {
        Self {
            level: KeyboardBacklightLevel::Off,
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::NextKeyboardBacklightLevel => {
                self.level = match self.level {
                    KeyboardBacklightLevel::Off => KeyboardBacklightLevel::Dim,
                    KeyboardBacklightLevel::Dim => KeyboardBacklightLevel::Medium,
                    KeyboardBacklightLevel::Medium => KeyboardBacklightLevel::Bright,
                    KeyboardBacklightLevel::Bright => KeyboardBacklightLevel::Off,
                };
                Action::None
            }
        }
    }

    pub fn quick_setting_button<'a>(
        &'a self,
    ) -> Option<(Element<'a, Message>, Option<Element<'a, Message>>)> {
        let (level, active) = match self.level {
            KeyboardBacklightLevel::Off => ("Off", false),
            KeyboardBacklightLevel::Dim => ("Dim", true),
            KeyboardBacklightLevel::Medium => ("Medium", true),
            KeyboardBacklightLevel::Bright => ("Bright", true),
        };
        Some((
            quick_setting_button(
                StaticIcon::Keyboard,
                "Keyboard Backlight".to_string(),
                Some(level.to_string()),
                active,
                Message::NextKeyboardBacklightLevel,
                None,
                None,
            ),
            None,
        ))
    }

    fn level(mode: KeyboardBacklightLevel) -> u32 {
        match mode {
            KeyboardBacklightLevel::Off => 0,
            KeyboardBacklightLevel::Dim => 1,
            KeyboardBacklightLevel::Medium => 2,
            KeyboardBacklightLevel::Bright => 3,
        }
    }
}
