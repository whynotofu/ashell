use anyhow::{Context, Error, Result, anyhow};
use iced::{
    Subscription,
    futures::{channel::mpsc, sink::SinkExt},
};
use log::warn;
use std::{fmt, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixStream, unix::OwnedWriteHalf},
    sync::Mutex,
};

pub struct DeviceService {
    battery_charge: Option<Percentage>,
    battery_status: Option<BatteryStatus>,
    battery_protection: Option<BatteryProtection>,
    display_brightness: Option<Percentage>,
    keyboard_backlight: Option<KeyboardBacklight>,
    platform_profile: Option<PlatformProfile>,
    writer: Option<Arc<Mutex<OwnedWriteHalf>>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Synced(Arc<Mutex<OwnedWriteHalf>>),
    Reset,
    BatteryCharge(Percentage),
    BatteryStatus(BatteryStatus),
    BatteryProtection(BatteryProtection),
    DisplayBrightness(Percentage),
    KeyboardBacklight(KeyboardBacklight),
    PlatformProfile(PlatformProfile),
}

impl DeviceService {
    pub fn new() -> Self {
        Self {
            battery_charge: None,
            battery_status: None,
            battery_protection: None,
            display_brightness: None,
            keyboard_backlight: None,
            platform_profile: None,
            writer: None,
        }
    }

    pub fn get_battery_info(&self) -> Option<(Percentage, BatteryStatus)> {
        if let (Some(charge), Some(status)) = (self.battery_charge, self.battery_status) {
            Some((charge, status))
        } else {
            None
        }
    }

    pub fn get_battery_protection(&self) -> Option<BatteryProtection> {
        self.battery_protection
    }

    pub fn set_battery_protection(&mut self, protection: BatteryProtection) {
        self.set_call(1, protection.to_u8());
    }

    pub fn get_display_brightness(&self) -> Option<Percentage> {
        self.display_brightness
    }

    pub fn set_display_brightness(&mut self, brightness: Percentage) {
        self.set_call(2, brightness.to_u8());
    }

    pub fn get_keyboard_backlight(&self) -> Option<KeyboardBacklight> {
        self.keyboard_backlight
    }

    pub fn set_keyboard_backlight(&mut self, backlight: KeyboardBacklight) {
        self.set_call(3, backlight.to_u8())
    }

    pub fn get_platform_profile(&self) -> Option<PlatformProfile> {
        self.platform_profile
    }

    pub fn set_platform_profile(&mut self, profile: PlatformProfile) {
        self.set_call(4, profile.to_u8());
    }

    pub fn set_call(&self, code: u8, value: u8) {
        if let Some(writer) = self.writer.clone() {
            tokio::spawn(async move {
                if let Err(e) = writer.lock().await.write_all(&[code, value]).await {
                    warn!("DeviceService.set_call: {}", e);
                }
            });
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Synced(writer) => self.writer = Some(writer),
            Message::Reset => {
                self.battery_charge = None;
                self.battery_status = None;
                self.battery_protection = None;
                self.display_brightness = None;
                self.keyboard_backlight = None;
                self.platform_profile = None;
                self.writer = None;
            }
            Message::BatteryCharge(charge) => self.battery_charge = Some(charge),
            Message::BatteryStatus(status) => self.battery_status = Some(status),
            Message::BatteryProtection(protection) => {
                self.battery_protection = Some(protection);
            }
            Message::DisplayBrightness(brightness) => {
                self.display_brightness = Some(brightness);
            }
            Message::KeyboardBacklight(backlight) => {
                self.keyboard_backlight = Some(backlight);
            }
            Message::PlatformProfile(profile) => {
                self.platform_profile = Some(profile);
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::run(|| {
            iced::stream::channel(100, |mut output: mpsc::Sender<Message>| async move {
                let intervals = [2u16, 5, 10, 15, 30, 60, 600];
                let mut atempt = 0;

                loop {
                    atempt += 1;

                    if let Err(e) = Self::connect(&mut output, &mut atempt).await {
                        warn!("DeviceService: {}", e);
                    }
                    output.send(Message::Reset).await.expect("Failed to reset!");

                    if atempt <= intervals.len() {
                        tokio::time::sleep(Duration::from_secs(intervals[atempt - 1] as u64)).await
                    } else {
                        break;
                    }
                }
            })
        })
    }

    async fn connect(output: &mut mpsc::Sender<Message>, atempt: &mut usize) -> Result<()> {
        let Ok(stream) = UnixStream::connect("/tmp/device.sock").await else {
            return Ok(());
        };

        let (mut reader, mut writer) = stream.into_split();
        let mut message = [0u8; 2];
        let mut synced = false;

        writer.write_all(&[255, 255]).await.context("Service down.")?;

        let writer = Arc::new(Mutex::new(writer));

        loop {
            reader.read_exact(&mut message).await.context("Service down.")?;
            let (code, value) = (message[0], message[1]);

            if code == 255 {
                if value == 1 && !synced {
                    output.send(Message::Synced(Arc::clone(&writer))).await?;
                    synced = true;
                    *atempt = 1;
                } else {
                    return Err(anyhow!("Protocol error."));
                }
            } else {
                match Message::try_from(&message) {
                    Ok(message) => {
                        if !synced {
                            output.feed(message).await?;
                        } else {
                            output.send(message).await?;
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
        }
    }
}

impl TryFrom<&[u8; 2]> for Message {
    type Error = Error;
    fn try_from(bytes: &[u8; 2]) -> Result<Self, Self::Error> {
        let (code, value) = (bytes[0], bytes[1]);

        match code {
            1 => Percentage::try_from(value).map(Message::BatteryCharge),
            2 => BatteryStatus::try_from(value).map(Message::BatteryStatus),
            3 => BatteryProtection::try_from(value).map(Message::BatteryProtection),
            4 => Percentage::try_from(value).map(Message::DisplayBrightness),
            5 => KeyboardBacklight::try_from(value).map(Message::KeyboardBacklight),
            6 => PlatformProfile::try_from(value).map(Message::PlatformProfile),
            _ => Err(anyhow!("Unknown message code.")),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BatteryStatus {
    Charged,
    Charging,
    Discharging,
    BatteryProtection,
}

impl TryFrom<u8> for BatteryStatus {
    type Error = Error;
    fn try_from(code: u8) -> Result<Self, Self::Error> {
        match code {
            1 => Ok(BatteryStatus::Charged),
            2 => Ok(BatteryStatus::Charging),
            3 => Ok(BatteryStatus::Discharging),
            4 => Ok(BatteryStatus::BatteryProtection),
            _ => Err(anyhow!("Unknown battery status code.")),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BatteryProtection {
    Off,
    On,
    Stationary,
}

impl BatteryProtection {
    pub fn to_str(&self) -> &'static str {
        match self {
            BatteryProtection::Off => "Off",
            BatteryProtection::On => "On",
            BatteryProtection::Stationary => "Stationary",
        }
    }
    fn to_u8(&self) -> u8 {
        match self {
            BatteryProtection::Off => 0,
            BatteryProtection::On => 1,
            BatteryProtection::Stationary => 2,
        }
    }
}

impl TryFrom<u8> for BatteryProtection {
    type Error = Error;
    fn try_from(code: u8) -> Result<Self, Self::Error> {
        match code {
            0 => Ok(BatteryProtection::Off),
            1 => Ok(BatteryProtection::On),
            2 => Ok(BatteryProtection::Stationary),
            _ => Err(anyhow!("Unknow battery protection code.")),
        }
    }
}

impl fmt::Display for BatteryProtection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Percentage {
    value: u8,
}

impl Percentage {
    pub fn new(value: u8) -> Self {
        Self { value }
    }
    pub fn add(self, value: u8) -> Self {
        Self {
            value: self.value.saturating_add(value).min(100),
        }
    }
    pub fn sub(self, value: u8) -> Self {
        Self {
            value: self.value.saturating_sub(value),
        }
    }
    pub fn to_u8(&self) -> u8 {
        self.value
    }
}

impl TryFrom<u8> for Percentage {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if (0u8..100).contains(&value) {
            Ok(Self { value })
        } else {
            Err(anyhow!("Invalid percentage value."))
        }
    }
}

impl fmt::Display for Percentage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum KeyboardBacklight {
    Off,
    Low,
    Medium,
    Max,
}

impl KeyboardBacklight {
    pub fn to_str(&self) -> &'static str {
        match self {
            KeyboardBacklight::Off => "Off",
            KeyboardBacklight::Low => "Low",
            KeyboardBacklight::Medium => "Medium",
            KeyboardBacklight::Max => "Max",
        }
    }
    fn to_u8(&self) -> u8 {
        match self {
            KeyboardBacklight::Off => 0,
            KeyboardBacklight::Low => 1,
            KeyboardBacklight::Medium => 2,
            KeyboardBacklight::Max => 3,
        }
    }
}

impl TryFrom<u8> for KeyboardBacklight {
    type Error = Error;
    fn try_from(code: u8) -> Result<Self, Self::Error> {
        match code {
            0 => Ok(KeyboardBacklight::Off),
            1 => Ok(KeyboardBacklight::Low),
            2 => Ok(KeyboardBacklight::Medium),
            3 => Ok(KeyboardBacklight::Max),
            _ => Err(anyhow!("Unknow keyboard backlight code.")),
        }
    }
}

impl fmt::Display for KeyboardBacklight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PlatformProfile {
    LowPower,
    Balanced,
    Performance,
}

impl PlatformProfile {
    pub fn to_str(&self) -> &'static str {
        match self {
            PlatformProfile::LowPower => "Low Power",
            PlatformProfile::Balanced => "Balanced",
            PlatformProfile::Performance => "Performance",
        }
    }
    fn to_u8(&self) -> u8 {
        match self {
            PlatformProfile::LowPower => 1,
            PlatformProfile::Balanced => 2,
            PlatformProfile::Performance => 3,
        }
    }
}

impl TryFrom<u8> for PlatformProfile {
    type Error = Error;
    fn try_from(code: u8) -> Result<Self, Self::Error> {
        match code {
            1 => Ok(PlatformProfile::LowPower),
            2 => Ok(PlatformProfile::Balanced),
            3 => Ok(PlatformProfile::Performance),
            _ => Err(anyhow!("Unknown platform profile code.")),
        }
    }
}

impl fmt::Display for PlatformProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_str())
    }
}
