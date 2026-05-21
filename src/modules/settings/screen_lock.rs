use crate::components::{icons::StaticIcon, quick_setting_button};
use iced::Element;

#[derive(Debug, Clone)]
pub enum Message {
    NextScreenLock,
}

pub enum Action {
    None,
}

pub struct ScreenLockSettings {
    auto_lock: bool,
}

impl ScreenLockSettings {
    pub fn new() -> Self {
        Self { auto_lock: true }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::NextScreenLock => {
                self.auto_lock = !self.auto_lock;
                Action::None
            }
        }
    }

    pub fn quick_setting_button<'a>(
        &'a self,
    ) -> Option<(Element<'a, Message>, Option<Element<'a, Message>>)> {
        let (level, active) = match self.auto_lock {
            true => ("30 minutes", true),
            false => ("Off", false),
        };
        Some((
            quick_setting_button(
                StaticIcon::Lock,
                "Screen Autolock".to_string(),
                Some(level.to_string()),
                active,
                Message::NextScreenLock,
                None,
                None,
            ),
            None,
        ))
    }
}
