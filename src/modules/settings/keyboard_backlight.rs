use crate::{
    components::{icons::StaticIcon, quick_setting_button},
    services::device::KeyboardBacklight,
};
use iced::Element;

#[derive(Debug, Clone)]
pub enum Message {
    NextKeyboardBacklightLevel,
}

pub struct KeyboardBacklightSettings {}

impl KeyboardBacklightSettings {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, message: Message, keyboard_backlight: KeyboardBacklight) -> KeyboardBacklight {
        match message {
            Message::NextKeyboardBacklightLevel => match keyboard_backlight {
                KeyboardBacklight::Off => KeyboardBacklight::Low,
                KeyboardBacklight::Low => KeyboardBacklight::Medium,
                KeyboardBacklight::Medium => KeyboardBacklight::High,
                KeyboardBacklight::High => KeyboardBacklight::Off,
            },
        }
    }

    pub fn quick_setting_button<'a>(
        &'a self,
        keyboard_backlight: KeyboardBacklight,
    ) -> Option<(Element<'a, Message>, Option<Element<'a, Message>>)> {
        let active = match keyboard_backlight {
            KeyboardBacklight::Off => false,
            _ => true,
        };
        Some((
            quick_setting_button(
                StaticIcon::Keyboard,
                "Keyboard Backlight".to_string(),
                Some(keyboard_backlight.to_string()),
                active,
                Message::NextKeyboardBacklightLevel,
                None,
                None,
            ),
            None,
        ))
    }
}
