use crate::{
    components::{
        MenuSize, brightness_slider_control, format_indicator,
        icons::{StaticIcon, icon, icon_button},
        password_dialog, quick_setting_button, sub_menu_wrapper,
    },
    config::{SettingsFormat, SettingsIndicator, SettingsModuleConfig},
    modules::settings::{
        audio::{AudioSettings, AudioSettingsConfig},
        battery::BatterySettings,
        bluetooth::{BluetoothSettings, BluetoothSettingsConfig},
        keyboard_backlight::KeyboardBacklightSettings,
        network::{NetworkSettings, NetworkSettingsConfig},
        power::{PowerSettings, PowerSettingsConfig},
        screen_lock::ScreenLockSettings,
        state::State,
    },
    osd,
    services::{
        device::{self, BatteryProtection, DeviceService, KeyboardBacklight, Percentage, PlatformProfile},
        idle_inhibitor::IdleInhibitorManager,
    },
    theme::use_theme,
    utils::IndicatorState,
};
use iced::{
    Element, Length, Subscription, SurfaceId, Task, Theme,
    mouse::ScrollDelta,
    widget::{Column, Row, Space, container, row, space, text},
};

pub(crate) mod audio;
mod battery;
mod bluetooth;
//pub(crate) mod brightness;
mod keyboard_backlight;
pub(crate) mod network;
mod power;
mod screen_lock;
mod state;

pub struct Settings {
    lock_cmd: Option<String>,
    power: PowerSettings,
    audio: AudioSettings,
    network: NetworkSettings,
    bluetooth: BluetoothSettings,
    idle_inhibitor: Option<IdleInhibitorManager>,
    keyboard_backlight: KeyboardBacklightSettings,
    screen_lock: ScreenLockSettings,
    sub_menu: Option<SubMenu>,
    network_dialog: Option<NetworkDialogState>,
    network_dialog_show_password: bool,
    battery: BatterySettings,
    indicators: Vec<SettingsIndicator>,
    device: DeviceService,
}

#[derive(Debug, Clone)]
enum NetworkDialogKind {
    Password,
    OpenNetworkWarning,
}

#[derive(Debug, Clone)]
struct NetworkDialogState {
    ssid: String,
    password: Option<String>,
    kind: NetworkDialogKind,
}

impl NetworkDialogState {
    fn new_password_dialog(ssid: String) -> Self {
        Self {
            ssid,
            password: Some(String::new()),
            kind: NetworkDialogKind::Password,
        }
    }

    fn new_warning_dialog(ssid: String) -> Self {
        Self {
            ssid,
            password: None,
            kind: NetworkDialogKind::OpenNetworkWarning,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Network(network::Message),
    Bluetooth(bluetooth::Message),
    Audio(audio::Message),
    ToggleInhibitIdle,
    CycleKeyboardBacklight,
    ScreenLock(screen_lock::Message),
    Lock,
    Power(power::Message),
    ToggleSubMenu(SubMenu),
    PasswordDialog(password_dialog::Message),
    MenuOpened,
    ConfigReloaded(SettingsModuleConfig),
    Battery(battery::Message),
    Device(device::Message),
    CycleBatteryProtection,
    CyclePlatformProfile,
    SetDisplayBrightness(u8),
}

pub enum Action {
    None,
    Command(Task<Message>),
    Response(Option<Task<Message>>, Option<osd::OsdMessage>),
    CloseMenu(SurfaceId),
    RequestKeyboard(SurfaceId),
    ReleaseKeyboard(SurfaceId),
    ReleaseKeyboardWithCommand(SurfaceId, Task<Message>),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SubMenu {
    PeripheralMenu,
    Power,
    Sinks,
    Sources,
    Wifi,
    Vpn,
    Bluetooth,
}

impl Settings {
    pub fn volume_adjust(&mut self, up: bool) -> Action {
        match self.audio.volume_adjust(up) {
            audio::Action::Response(task, osd) => match task {
                Some(task) => Action::Response(Some(task.map(Message::Audio)), osd),
                None => Action::Response(None, osd),
            },
            _ => Action::None,
        }
    }

    pub fn toggle_mute(&mut self) -> Action {
        match self.audio.toggle_mute() {
            audio::Action::Response(task, osd) => match task {
                Some(task) => Action::Response(Some(task.map(Message::Audio)), osd),
                None => Action::Response(None, osd),
            },
            _ => Action::None,
        }
    }

    pub fn microphone_adjust(&mut self, up: bool) -> Action {
        match self.audio.microphone_adjust(up) {
            audio::Action::Response(task, osd) => match task {
                Some(task) => Action::Response(Some(task.map(Message::Audio)), osd),
                None => Action::Response(None, osd),
            },
            _ => Action::None,
        }
    }

    pub fn microphone_toggle_mute(&mut self) -> Action {
        match self.audio.microphone_toggle_mute() {
            audio::Action::Response(task, osd) => match task {
                Some(task) => Action::Response(Some(task.map(Message::Audio)), osd),
                None => Action::Response(None, osd),
            },
            _ => Action::None,
        }
    }

    pub fn brightness_adjust(&mut self, up: bool) -> Action {
        match self.device.get_display_brightness() {
            Some(brightness) => {
                let brightness = if up { brightness.add(1) } else { brightness.sub(1) };
                self.device.set_display_brightness(brightness);
                Action::Response(
                    None,
                    Some(osd::OsdMessage::Brightness {
                        value: (brightness.to_u8() as f32) / 100.0,
                    }),
                )
            }
            None => Action::None,
        }
    }

    pub fn toggle_airplane(&mut self) -> Action {
        let action = match self.network.update(network::Message::ToggleAirplaneMode) {
            network::Action::CloseSubMenu(task) => Some(task.map(Message::Network)),
            network::Action::Command(task) => Some(task.map(Message::Network)),
            _ => None,
        };
        let osd = osd::OsdMessage::Airplane {
            active: !self.network.is_airplane_mode().unwrap_or(false),
        };
        Action::Response(action, Some(osd))
    }

    pub fn toggle_idle_inhibitor(&mut self) -> Action {
        if let Some(idle_inhibitor) = &mut self.idle_inhibitor {
            idle_inhibitor.toggle();
            let osd = osd::OsdMessage::IdleInhibitor {
                active: idle_inhibitor.is_inhibited(),
            };
            return Action::Response(None, Some(osd));
        }
        Action::None
    }

    pub fn save_state(&mut self) {
        State {
            battery_protection: self.battery.state,
            audio_state: self.audio.get_audio_state(),
        }
        .save()
    }

    pub fn new(config: SettingsModuleConfig) -> Self {
        let state = State::load();

        Settings {
            lock_cmd: config.lock_cmd,
            power: PowerSettings::new(PowerSettingsConfig::new(
                config.suspend_cmd,
                config.hibernate_cmd,
                config.reboot_cmd,
                config.shutdown_cmd,
                config.logout_cmd,
                config.battery_format,
                config.battery_hide_when_full,
                config.peripheral_indicators,
                config.peripheral_battery_format,
                config.peripheral_expanded_by_default,
            )),
            audio: AudioSettings::new(AudioSettingsConfig::new(config.audio_step), state.audio_state),
            network: NetworkSettings::new(NetworkSettingsConfig::new(config.vpn_more_cmd, config.remove_airplane_btn)),
            bluetooth: BluetoothSettings::new(BluetoothSettingsConfig::new(config.bluetooth_more_cmd)),
            idle_inhibitor: if config.remove_idle_btn {
                None
            } else {
                IdleInhibitorManager::new()
            },
            keyboard_backlight: KeyboardBacklightSettings::new(),
            screen_lock: ScreenLockSettings::new(),
            sub_menu: None,
            network_dialog: None,
            battery: BatterySettings::new(state.battery_protection),
            indicators: config.indicators,
            network_dialog_show_password: false,
            device: DeviceService::new(),
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Power(msg) => match self.power.update(msg) {
                power::Action::None => Action::None,
                power::Action::TogglePeripheralMenu => {
                    if self.sub_menu == Some(SubMenu::PeripheralMenu) {
                        self.sub_menu.take();
                    } else {
                        self.sub_menu.replace(SubMenu::PeripheralMenu);
                    }
                    Action::None
                }
                power::Action::Command(task) => Action::Command(task.map(Message::Power)),
            },
            Message::Battery(msg) => {
                let _ = self.battery.update(msg);
                Action::None
            }
            Message::CyclePlatformProfile => {
                if let Some(profile) = self.device.get_platform_profile() {
                    let profile = match profile {
                        PlatformProfile::LowPower => PlatformProfile::Balanced,
                        PlatformProfile::Balanced => PlatformProfile::Performance,
                        PlatformProfile::Performance => PlatformProfile::LowPower,
                    };
                    self.device.set_platform_profile(profile);
                }
                Action::None
            }
            Message::CycleBatteryProtection => {
                if let Some(protection) = self.device.get_battery_protection() {
                    let protection = match protection {
                        BatteryProtection::Off => BatteryProtection::On,
                        BatteryProtection::On => BatteryProtection::Stationary,
                        BatteryProtection::Stationary => BatteryProtection::Off,
                    };
                    self.device.set_battery_protection(protection);
                }
                Action::None
            }
            Message::CycleKeyboardBacklight => {
                if let Some(backlight) = self.device.get_keyboard_backlight() {
                    let backlight = match backlight {
                        KeyboardBacklight::Off => KeyboardBacklight::Low,
                        KeyboardBacklight::Low => KeyboardBacklight::Medium,
                        KeyboardBacklight::Medium => KeyboardBacklight::High,
                        KeyboardBacklight::High => KeyboardBacklight::Off,
                    };
                    self.device.set_keyboard_backlight(backlight);
                }
                Action::None
            }
            Message::ScreenLock(msg) => {
                let _ = self.screen_lock.update(msg);
                Action::None
            }
            Message::Audio(msg) => match self.audio.update(msg) {
                audio::Action::None => Action::None,
                audio::Action::ToggleSinksMenu => {
                    if self.sub_menu == Some(SubMenu::Sinks) {
                        self.sub_menu.take();
                    } else {
                        self.sub_menu.replace(SubMenu::Sinks);
                    }
                    Action::None
                }
                audio::Action::ToggleSourcesMenu => {
                    if self.sub_menu == Some(SubMenu::Sources) {
                        self.sub_menu.take();
                    } else {
                        self.sub_menu.replace(SubMenu::Sources);
                    }
                    Action::None
                }
                audio::Action::CloseSubMenu => {
                    if self.sub_menu == Some(SubMenu::Sinks) || self.sub_menu == Some(SubMenu::Sources) {
                        self.sub_menu.take();
                    }
                    Action::None
                }
                audio::Action::Response(task, osd) => match task {
                    Some(task) => Action::Response(Some(task.map(Message::Audio)), osd),
                    None => Action::Response(None, osd),
                },
            },
            Message::Network(msg) => match self.network.update(msg) {
                network::Action::None => Action::None,
                network::Action::RequestPasswordForSSID(ssid) => {
                    self.network_dialog = Some(NetworkDialogState::new_password_dialog(ssid));
                    self.network_dialog_show_password = false;
                    Action::None
                }
                network::Action::RequestPassword(id, ssid) => {
                    self.network_dialog = Some(NetworkDialogState::new_password_dialog(ssid));
                    self.network_dialog_show_password = false;
                    Action::RequestKeyboard(id)
                }
                network::Action::ConfirmOpenNetwork(ssid) => {
                    self.network_dialog = Some(NetworkDialogState::new_warning_dialog(ssid));
                    self.network_dialog_show_password = false;
                    Action::None
                }
                network::Action::Command(task) => Action::Command(task.map(Message::Network)),
                network::Action::ToggleWifiMenu => {
                    if self.sub_menu == Some(SubMenu::Wifi) {
                        self.sub_menu.take();
                    } else {
                        self.sub_menu.replace(SubMenu::Wifi);
                    }
                    Action::None
                }
                network::Action::ToggleVpnMenu => {
                    if self.sub_menu == Some(SubMenu::Vpn) {
                        self.sub_menu.take();
                    } else {
                        self.sub_menu.replace(SubMenu::Vpn);
                    }
                    Action::None
                }
                network::Action::CloseSubMenu(task) => {
                    if self.sub_menu == Some(SubMenu::Wifi) || self.sub_menu == Some(SubMenu::Vpn) {
                        self.sub_menu.take();
                    }

                    Action::Command(task.map(Message::Network))
                }
                network::Action::CloseMenu(id) => Action::CloseMenu(id),
            },
            Message::Bluetooth(msg) => match self.bluetooth.update(msg) {
                bluetooth::Action::None => Action::None,
                bluetooth::Action::ToggleBluetoothMenu => {
                    if self.sub_menu == Some(SubMenu::Bluetooth) {
                        self.sub_menu.take();
                    } else {
                        self.sub_menu.replace(SubMenu::Bluetooth);
                    }
                    Action::None
                }
                bluetooth::Action::CloseSubMenu(task) => {
                    if self.sub_menu == Some(SubMenu::Bluetooth) {
                        self.sub_menu.take();
                    }

                    Action::Command(task.map(Message::Bluetooth))
                }
                bluetooth::Action::Command(task) => Action::Command(task.map(Message::Bluetooth)),
                bluetooth::Action::CloseMenu(id) => Action::CloseMenu(id),
            },
            Message::SetDisplayBrightness(brightness) => {
                self.device.set_display_brightness(Percentage::new(brightness));
                Action::Response(
                    None,
                    Some(osd::OsdMessage::Brightness {
                        value: (brightness as f32) / 100.0,
                    }),
                )
            }
            Message::ToggleSubMenu(menu_type) => {
                if self.sub_menu == Some(menu_type) {
                    self.sub_menu.take();

                    Action::None
                } else {
                    self.sub_menu.replace(menu_type);

                    if menu_type == SubMenu::Wifi {
                        match self.network.update(network::Message::WifiMenuOpened) {
                            network::Action::Command(task) => Action::Command(task.map(Message::Network)),
                            _ => Action::None,
                        }
                    } else {
                        Action::None
                    }
                }
            }
            Message::ToggleInhibitIdle => {
                if let Some(idle_inhibitor) = &mut self.idle_inhibitor {
                    idle_inhibitor.toggle();
                }
                Action::None
            }
            Message::Device(message) => match self.device.update(message) {
                device::Action::Event(event) => Action::Response(None, Some(event)),
                _ => Action::None,
            },
            Message::Lock => {
                if let Some(lock_cmd) = &self.lock_cmd {
                    crate::utils::launcher::execute_command(lock_cmd.to_string());
                }
                Action::None
            }
            Message::PasswordDialog(msg) => match msg {
                password_dialog::Message::PasswordChanged(password) => {
                    if let Some(dialog) = &mut self.network_dialog {
                        dialog.password = Some(password);
                    }

                    Action::None
                }
                password_dialog::Message::TogglePasswordVisibility => {
                    self.network_dialog_show_password = !self.network_dialog_show_password;

                    Action::None
                }
                password_dialog::Message::DialogConfirmed(id) => {
                    let action = if let Some(dialog) = self.network_dialog.take() {
                        let message = match dialog.kind {
                            NetworkDialogKind::Password => {
                                network::Message::PasswordDialogConfirmed(dialog.ssid, dialog.password.unwrap_or_default())
                            }
                            NetworkDialogKind::OpenNetworkWarning => network::Message::OpenNetworkDialogConfirmed(dialog.ssid),
                        };

                        match self.network.update(message) {
                            network::Action::Command(task) => {
                                Action::ReleaseKeyboardWithCommand(id, task.map(Message::Network))
                            }
                            _ => Action::ReleaseKeyboard(id),
                        }
                    } else {
                        Action::ReleaseKeyboard(id)
                    };
                    self.network_dialog_show_password = false;
                    action
                }
                password_dialog::Message::DialogCancelled(id) => {
                    self.network_dialog = None;
                    self.network_dialog_show_password = false;

                    Action::ReleaseKeyboard(id)
                }
            },

            Message::MenuOpened => {
                self.sub_menu = if self.power.config.peripheral_expanded_by_default {
                    Some(SubMenu::PeripheralMenu)
                } else {
                    None
                };

                Action::None
            }
            Message::ConfigReloaded(config) => {
                self.lock_cmd = config.lock_cmd;
                self.power.update(power::Message::ConfigReloaded(PowerSettingsConfig::new(
                    config.suspend_cmd,
                    config.hibernate_cmd,
                    config.reboot_cmd,
                    config.shutdown_cmd,
                    config.logout_cmd,
                    config.battery_format,
                    config.battery_hide_when_full,
                    config.peripheral_indicators,
                    config.peripheral_battery_format,
                    config.peripheral_expanded_by_default,
                )));
                self.audio.update(audio::Message::ConfigReloaded(AudioSettingsConfig::new(config.audio_step)));
                self.network.update(network::Message::ConfigReloaded(NetworkSettingsConfig::new(
                    config.vpn_more_cmd,
                    config.remove_airplane_btn,
                )));
                self.bluetooth.update(bluetooth::Message::ConfigReloaded(BluetoothSettingsConfig::new(
                    config.bluetooth_more_cmd,
                )));
                if config.remove_idle_btn {
                    self.idle_inhibitor = None;
                } else if self.idle_inhibitor.is_none() {
                    self.idle_inhibitor = IdleInhibitorManager::new();
                }
                self.indicators = config.indicators;
                Action::None
            }
        }
    }

    pub fn menu_view<'a>(&'a self, id: SurfaceId) -> Element<'a, Message> {
        let space = use_theme(|t| t.space);
        container(if let Some(dialog) = &self.network_dialog {
            password_dialog::view(
                id,
                &dialog.ssid,
                dialog.password.as_deref().unwrap_or(""),
                self.network_dialog_show_password,
                matches!(dialog.kind, NetworkDialogKind::OpenNetworkWarning),
            )
            .map(Message::PasswordDialog)
        } else {
            let right_buttons = Row::with_capacity(2)
                .push(self.lock_cmd.as_ref().map(|_| icon_button(StaticIcon::Lock).on_press(Message::Lock)))
                .push(
                    icon_button(if self.sub_menu == Some(SubMenu::Power) {
                        StaticIcon::Close
                    } else {
                        StaticIcon::Power
                    })
                    .on_press(Message::ToggleSubMenu(SubMenu::Power)),
                )
                .spacing(space.xs);

            let header = Row::with_capacity(3)
                .push(Space::new().width(Length::Fill))
                .push(right_buttons)
                .spacing(space.xs)
                .width(Length::Fill);

            let (sink_slider, source_slider) = self.audio.sliders(self.sub_menu);

            let mut quick_settings = Vec::with_capacity(10);

            quick_settings.push(
                self.network
                    .wifi_quick_setting_button(id, self.sub_menu)
                    .map(|(button, submenu)| (button.map(Message::Network), submenu.map(|e| e.map(Message::Network)))),
            );

            quick_settings.push(
                self.bluetooth
                    .quick_setting_button(id, self.sub_menu)
                    .map(|(button, submenu)| (button.map(Message::Bluetooth), submenu.map(|e| e.map(Message::Bluetooth)))),
            );

            quick_settings.push(
                self.network
                    .vpn_quick_setting_button(id, self.sub_menu)
                    .map(|(button, submenu)| (button.map(Message::Network), submenu.map(|e| e.map(Message::Network)))),
            );

            quick_settings.push(
                self.network.airplane_mode_quick_setting_button().map(|(button, _)| (button.map(Message::Network), None)),
            );

            quick_settings.push(self.idle_inhibitor.as_ref().map(|idle_inhibitor| {
                (
                    quick_setting_button(
                        IdleInhibitorManager::idle_inhibitor_icon(idle_inhibitor.is_inhibited()),
                        "Display Power Saver".to_string(),
                        Some(
                            match !idle_inhibitor.is_inhibited() {
                                true => "5 minutes",
                                false => "Off",
                            }
                            .to_string(),
                        ),
                        !idle_inhibitor.is_inhibited(),
                        Message::ToggleInhibitIdle,
                        None,
                        None,
                    ),
                    None,
                )
            }));

            if let Some(platform_profile) = self.device.get_platform_profile() {
                quick_settings.push(Some((
                    quick_setting_button(
                        StaticIcon::Balanced,
                        "Power Profile".to_string(),
                        Some(platform_profile.to_string()),
                        true,
                        Message::CyclePlatformProfile,
                        None,
                        None,
                    ),
                    None,
                )));
            }

            if let Some(battery_protection) = self.device.get_battery_protection() {
                let active = battery_protection != BatteryProtection::Off;
                quick_settings.push(Some((
                    quick_setting_button(
                        StaticIcon::BatteryCharging,
                        "Battery Protection".to_string(),
                        Some(battery_protection.to_string()),
                        active,
                        Message::CycleBatteryProtection,
                        None,
                        None,
                    ),
                    None,
                )));
            }

            if let Some(keyboard_backlight) = self.device.get_keyboard_backlight() {
                let active = keyboard_backlight != KeyboardBacklight::Off;
                quick_settings.push(Some((
                    quick_setting_button(
                        StaticIcon::Keyboard,
                        "Keyboard Backlight".to_string(),
                        Some(keyboard_backlight.to_string()),
                        active,
                        Message::CycleKeyboardBacklight,
                        None,
                        None,
                    ),
                    None,
                )));
            }

            quick_settings.push(
                self.screen_lock
                    .quick_setting_button()
                    .map(|(button, submenu)| (button.map(Message::ScreenLock), submenu.map(|e| e.map(Message::ScreenLock)))),
            );

            let quick_settings = quick_settings_section(quick_settings.into_iter().flatten().collect::<Vec<_>>());

            let mut content = Column::with_capacity(11)
                .push(header)
                .push(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::PeripheralMenu)
                        .and_then(|_| self.power.peripheral_menu().map(|e| sub_menu_wrapper(e.map(Message::Power)))),
                )
                .push(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Power)
                        .map(|_| sub_menu_wrapper(self.power.menu().map(Message::Power))),
                )
                .push(sink_slider.map(|e| e.map(Message::Audio)))
                .push(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Sinks)
                        .and_then(|_| self.audio.sinks_submenu().map(|submenu| sub_menu_wrapper(submenu.map(Message::Audio)))),
                )
                .push(source_slider.map(|e| e.map(Message::Audio)))
                .push(
                    self.sub_menu.filter(|menu_type| *menu_type == SubMenu::Sources).and_then(|_| {
                        self.audio.sources_submenu().map(|submenu| sub_menu_wrapper(submenu.map(Message::Audio)))
                    }),
                );

            if let Some(brightness) = self.device.get_display_brightness() {
                let slider: Element<Message> =
                    brightness_slider_control(brightness.to_u8(), Message::SetDisplayBrightness).into();

                content = content.push(slider);
            }

            content.push(quick_settings).spacing(space.md).into()
        })
        .width(MenuSize::Medium)
        .into()
    }

    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        let space = use_theme(|t| t.space);
        let mut row = Row::with_capacity(self.indicators.len());

        for indicator in &self.indicators {
            match indicator {
                SettingsIndicator::IdleInhibitor => {
                    if let Some(element) = self.idle_inhibitor.as_ref().filter(|i| i.is_inhibited()).map(|_| {
                        container(icon(IdleInhibitorManager::idle_inhibitor_icon(true))).style(|theme: &Theme| {
                            container::Style {
                                text_color: Some(theme.palette().danger),
                                ..Default::default()
                            }
                        })
                    }) {
                        row = row.push(element);
                    }
                }
                SettingsIndicator::PowerProfile => {
                    if let Some(element) = self.power.power_profile_indicator().map(|e| e.map(Message::Power)) {
                        row = row.push(element);
                    }
                }
                SettingsIndicator::Audio => {
                    if let Some(element) = self.audio.sink_indicator().map(|e| e.map(Message::Audio)) {
                        row = row.push(element);
                    }
                }
                SettingsIndicator::Network => {
                    if let Some(element) = self.network.connection_indicator().map(|e| e.map(Message::Network)) {
                        row = row.push(element);
                    }
                }
                SettingsIndicator::Vpn => {
                    if let Some(element) = self.network.vpn_indicator().map(|e| e.map(Message::Network)) {
                        row = row.push(element);
                    }
                }
                SettingsIndicator::Bluetooth => {
                    if let Some(element) = self.bluetooth.bluetooth_indicator().map(|e| e.map(Message::Bluetooth)) {
                        row = row.push(element);
                    }
                }
                SettingsIndicator::Microphone => {
                    if let Some(element) = self.audio.source_indicator().map(|e| e.map(Message::Audio)) {
                        row = row.push(element);
                    }
                }
                SettingsIndicator::Battery => {
                    if let Some((charge, _status)) = self.device.get_battery_info() {
                        let icon = get_battery_icon(charge.to_u8());
                        let state = if charge.to_u8() < 15 {
                            IndicatorState::Danger
                        } else {
                            IndicatorState::Normal
                        };
                        row = row.push(format_indicator(
                            SettingsFormat::IconAndPercentage,
                            icon,
                            text(format!("{}%", charge)).into(),
                            state,
                        ));
                    }
                }
                SettingsIndicator::PeripheralBattery => {
                    if let Some(element) = self.power.peripheral_indicators().map(|e| e.map(Message::Power)) {
                        row = row.push(element);
                    }
                }
                SettingsIndicator::Brightness => {
                    if let Some(brightness) = self.device.get_display_brightness() {
                        let idicator: Element<Message> = format_indicator(
                            SettingsFormat::Icon,
                            StaticIcon::Brightness,
                            text(format!("{}%", brightness)).into(),
                            IndicatorState::Normal,
                        )
                        .on_scroll(Self::brightness_indicator_on_scroll(brightness.to_u8()))
                        .into();

                        row = row.push(idicator);
                    }
                }
            }
        }

        row.spacing(space.xs).into()
    }

    fn brightness_indicator_on_scroll(current: u8) -> impl Fn(ScrollDelta) -> Message {
        move |delta| {
            let y = match delta {
                ScrollDelta::Lines { y, .. } => y,
                ScrollDelta::Pixels { y, .. } => y,
            };
            let new = if y > 0.0 {
                (current + 1).min(100)
            } else {
                current.saturating_sub(1)
            };
            Message::SetDisplayBrightness(new)
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.power.subscription().map(Message::Power),
            self.audio.subscription().map(Message::Audio),
            self.network.subscription().map(Message::Network),
            self.bluetooth.subscription().map(Message::Bluetooth),
            self.device.subscription().map(Message::Device),
        ])
    }
}

fn quick_settings_section<'a>(buttons: Vec<(Element<'a, Message>, Option<Element<'a, Message>>)>) -> Element<'a, Message> {
    let space = use_theme(|t| t.space);
    // TODO trying to read this function gives me a headache; there's surely
    // a better way to do this, maybe with Iterator::chunks or something?
    // I might be way off though, I still don't fully understand how this works.
    let mut section = Column::with_capacity(buttons.len() * 3).spacing(space.xs);

    let mut before: Option<(Element<'a, Message>, Option<Element<'a, Message>>)> = None;

    for (button, menu) in buttons.into_iter() {
        match before.take() {
            Some((before_button, before_menu)) => {
                section = section.push(row![before_button, button].width(Length::Fill).spacing(space.xs));

                if let Some(menu) = before_menu {
                    section = section.push(sub_menu_wrapper(menu));
                }

                if let Some(menu) = menu {
                    section = section.push(sub_menu_wrapper(menu));
                }
            }
            _ => {
                before = Some((button, menu));
            }
        }
    }

    if let Some((before_button, before_menu)) = before.take() {
        section = section.push(row![before_button, space::horizontal()].width(Length::Fill).spacing(space.xs));

        if let Some(menu) = before_menu {
            section = section.push(sub_menu_wrapper(menu));
        }
    }

    section.into()
}

pub fn get_battery_icon(charge: u8) -> StaticIcon {
    match charge {
        (0..20) => StaticIcon::Battery0,
        (20..40) => StaticIcon::Battery1,
        (40..60) => StaticIcon::Battery2,
        (60..80) => StaticIcon::Battery3,
        (80..) => StaticIcon::Battery4,
    }
}
