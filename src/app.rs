use crate::{
    HEIGHT,
    components::{ButtonUIRef, Centerbox, menu::MenuType},
    config::{self, AppearanceStyle, Config, Modules},
    get_log_spec,
    i18n::{Localizer, init_localizer},
    ipc::IpcCommand,
    modules::{
        self,
        clock::{self, Clock},
        keyboard_layout::KeyboardLayout,
        keyboard_submap::KeyboardSubmap,
        privacy::Privacy,
        settings::{self, Settings},
        system_info::SystemInfo,
        window_title::WindowTitle,
        workspaces::Workspaces,
    },
    osd::{self, Osd},
    outputs::{HasOutput, Outputs},
    services::ReadOnlyService,
    theme::{AshellTheme, backdrop_color, darken_color, init_theme, use_theme},
};
use flexi_logger::LoggerHandle;
use iced::{
    Alignment, Element, Length, OutputEvent, Subscription, SurfaceId, Task, Theme,
    set_exclusive_zone,
    widget::{Row, container, mouse_area},
};
use log::{info, warn};
use std::path::PathBuf;

const OSD_WIDTH: u32 = 250;
const OSD_HEIGHT: u32 = 64;

fn resolve_localizer(config: &Config) -> Localizer {
    Localizer::resolve(config.language.as_deref(), config.region.as_deref())
}

pub struct GeneralConfig {
    outputs: config::Outputs,
    pub modules: Modules,
    pub layer: config::Layer,
}

pub struct App {
    config_path: PathBuf,
    logger: LoggerHandle,
    pub general_config: GeneralConfig,
    pub outputs: Outputs,
    pub workspaces: Workspaces,
    pub window_title: WindowTitle,
    pub system_info: SystemInfo,
    pub keyboard_layout: KeyboardLayout,
    pub keyboard_submap: KeyboardSubmap,
    pub clock: Clock,
    pub privacy: Privacy,
    pub settings: Settings,
    pub osd: Osd,
    pub visible: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    ConfigChanged(Box<Config>),
    ToggleMenu(MenuType, SurfaceId, ButtonUIRef),
    CloseMenu(SurfaceId),
    Workspaces(modules::workspaces::Message),
    WindowTitle(modules::window_title::Message),
    SystemInfo(modules::system_info::Message),
    KeyboardLayout(modules::keyboard_layout::Message),
    KeyboardSubmap(modules::keyboard_submap::Message),
    Clock(modules::clock::Message),
    Privacy(modules::privacy::Message),
    Settings(modules::settings::Message),
    Osd(osd::Message),
    IpcOsdCommand(IpcCommand),
    OutputEvent(OutputEvent),
    ResumeFromSleep,
    None,
    ToggleVisibility,
    SaveState,
}

impl App {
    pub fn new(
        (logger, config, config_path): (LoggerHandle, Config, PathBuf),
    ) -> impl FnOnce() -> (Self, Task<Message>) {
        move || {
            let outputs = Outputs::new(
                config.appearance.style,
                config.layer,
                config.appearance.scale_factor,
            );

            init_theme(AshellTheme::new(&config.appearance));
            init_localizer(resolve_localizer(&config));

            (
                App {
                    config_path,
                    logger,
                    general_config: GeneralConfig {
                        outputs: config.outputs,
                        modules: config.modules,
                        layer: config.layer,
                    },
                    outputs,
                    workspaces: Workspaces::new(config.workspaces),
                    window_title: WindowTitle::new(config.window_title),
                    system_info: SystemInfo::new(),
                    keyboard_layout: KeyboardLayout::new(config.keyboard_layout),
                    keyboard_submap: KeyboardSubmap::default(),
                    clock: Clock::new(config.clock),
                    privacy: Privacy::default(),
                    settings: Settings::new(config.settings),
                    osd: Osd::new(config.osd),
                    visible: true,
                },
                Task::none(),
            )
        }
    }

    fn refresh_config(&mut self, config: Box<Config>) {
        init_theme(AshellTheme::new(&config.appearance));
        init_localizer(resolve_localizer(&config));
        self.general_config = GeneralConfig {
            outputs: config.outputs,
            modules: config.modules,
            layer: config.layer,
        };

        // ignore task, since config change should not generate any
        let _ = self
            .workspaces
            .update(modules::workspaces::Message::ConfigReloaded(
                config.workspaces,
            ))
            .map(Message::Workspaces);

        self.window_title
            .update(modules::window_title::Message::ConfigReloaded(
                config.window_title,
            ));

        self.system_info = SystemInfo::new();

        let _ = self
            .keyboard_layout
            .update(modules::keyboard_layout::Message::ConfigReloaded(
                config.keyboard_layout,
            ))
            .map(Message::KeyboardLayout);

        self.keyboard_submap = KeyboardSubmap::default();
        self.clock
            .update(modules::clock::Message::ConfigReloaded(config.clock));
        self.settings
            .update(modules::settings::Message::ConfigReloaded(config.settings));
        self.osd.update(osd::Message::ConfigReloaded(config.osd));
    }

    pub fn theme(&self) -> Theme {
        use_theme(|t| t.iced_theme.clone())
    }

    pub fn scale_factor(&self) -> f64 {
        use_theme(|t| t.scale_factor)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ConfigChanged(config) => {
                info!("New config: {config:?}");
                let mut tasks = Vec::new();
                info!(
                    "Current outputs: {:?}, new outputs: {:?}",
                    self.general_config.outputs, config.outputs
                );
                let (bar_style, scale_factor) = use_theme(|t| (t.bar_style, t.scale_factor));
                if self.general_config.outputs != config.outputs
                    || bar_style != config.appearance.style
                    || scale_factor != config.appearance.scale_factor
                    || self.general_config.layer != config.layer
                {
                    warn!("Outputs changed, syncing");
                    tasks.push(self.outputs.sync(
                        config.appearance.style,
                        &config.outputs,
                        config.layer,
                        config.appearance.scale_factor,
                    ));
                }

                self.logger.set_new_spec(get_log_spec(&config.log_level));
                self.refresh_config(config);

                Task::batch(tasks)
            }
            Message::ToggleMenu(menu_type, id, button_ui_ref) => {
                let mut cmd = vec![];
                match &menu_type {
                    MenuType::Clock => {
                        self.clock.update(clock::Message::Reset);
                    }
                    MenuType::Settings => {
                        cmd.push(
                            match self.settings.update(modules::settings::Message::MenuOpened) {
                                modules::settings::Action::Command(task) => {
                                    task.map(Message::Settings)
                                }
                                _ => Task::none(),
                            },
                        );
                    }
                    _ => {}
                };
                cmd.push(
                    self.outputs
                        .toggle_menu(id, menu_type, button_ui_ref, false),
                );

                Task::batch(cmd)
            }
            Message::CloseMenu(id) => self.outputs.close_menu(id, None, false),
            Message::Workspaces(msg) => self.workspaces.update(msg).map(Message::Workspaces),
            Message::WindowTitle(msg) => {
                self.window_title.update(msg);
                Task::none()
            }
            Message::SystemInfo(msg) => {
                self.system_info.update(msg);
                Task::none()
            }
            Message::KeyboardLayout(message) => self
                .keyboard_layout
                .update(message)
                .map(Message::KeyboardLayout),
            Message::KeyboardSubmap(message) => {
                self.keyboard_submap.update(message);
                Task::none()
            }
            Message::Clock(message) => match self.clock.update(message) {
                modules::clock::Action::None => Task::none(),
            },
            Message::Privacy(msg) => {
                self.privacy.update(msg);
                Task::none()
            }
            Message::Settings(message) => match self.settings.update(message) {
                modules::settings::Action::None => Task::none(),
                modules::settings::Action::Command(task) => task.map(Message::Settings),
                modules::settings::Action::Response(task, osd) => {
                    let mut tasks = vec![];
                    if let Some(task) = task {
                        tasks.push(task.map(Message::Settings));
                    }
                    if self.osd.config().enabled
                        && !self.outputs.menu_is_open()
                        && let Some(osd) = osd
                        && let osd::Action::Show(timer) = self.osd.update(osd::Message::Show(osd))
                    {
                        tasks.push(timer.map(Message::Osd));
                        tasks.push(self.outputs.show_osd_layer(OSD_WIDTH, OSD_HEIGHT));
                    }
                    Task::batch(tasks)
                }
                modules::settings::Action::CloseMenu(id) => {
                    self.outputs.close_menu(id, None, false)
                }
                modules::settings::Action::RequestKeyboard(id) => self.outputs.request_keyboard(id),
                modules::settings::Action::ReleaseKeyboard(id) => self.outputs.release_keyboard(id),
                modules::settings::Action::ReleaseKeyboardWithCommand(id, task) => {
                    Task::batch(vec![
                        task.map(Message::Settings),
                        self.outputs.release_keyboard(id),
                    ])
                }
            },
            Message::OutputEvent(event) => match event {
                OutputEvent::Added(info) => {
                    info!("Output created: {info:?}");
                    let name = &format!("{} {} {}", info.name, info.make, info.model);

                    if let Some((_, h)) = info.logical_size {
                        self.outputs.set_output_logical_height(info.id, h as u32);
                    }

                    let (bar_style, scale_factor) = use_theme(|t| (t.bar_style, t.scale_factor));
                    self.outputs.add(
                        bar_style,
                        &self.general_config.outputs,
                        self.general_config.layer,
                        name,
                        info.id,
                        scale_factor,
                    )
                }
                OutputEvent::Removed(output_id) => {
                    info!("Output destroyed");
                    let (bar_style, scale_factor) = use_theme(|t| (t.bar_style, t.scale_factor));
                    self.outputs.remove(
                        bar_style,
                        self.general_config.layer,
                        output_id,
                        scale_factor,
                    )
                }
                OutputEvent::InfoChanged(_) => Task::none(),
            },
            Message::ResumeFromSleep => {
                let (bar_style, scale_factor) = use_theme(|t| (t.bar_style, t.scale_factor));
                self.outputs.sync(
                    bar_style,
                    &self.general_config.outputs,
                    self.general_config.layer,
                    scale_factor,
                )
            }
            Message::IpcOsdCommand(cmd) => {
                let mut tasks = vec![];

                // Execute the action via Settings.
                let action = match &cmd {
                    IpcCommand::VolumeUp { .. } => self.settings.volume_adjust(true),
                    IpcCommand::VolumeDown { .. } => self.settings.volume_adjust(false),
                    IpcCommand::VolumeToggleMute { .. } => self.settings.toggle_mute(),
                    IpcCommand::MicrophoneUp { .. } => self.settings.microphone_adjust(true),
                    IpcCommand::MicrophoneDown { .. } => self.settings.microphone_adjust(false),
                    IpcCommand::MicrophoneToggleMute { .. } => {
                        self.settings.microphone_toggle_mute()
                    }
                    IpcCommand::BrightnessUp { .. } => self.settings.brightness_adjust(true),
                    IpcCommand::BrightnessDown { .. } => self.settings.brightness_adjust(false),
                    IpcCommand::ToggleAirplaneMode { .. } => self.settings.toggle_airplane(),
                    IpcCommand::ToggleIdleInhibitor { .. } => self.settings.toggle_idle_inhibitor(),
                    IpcCommand::ToggleVisibility => unreachable!(),
                };
                if let settings::Action::Response(task, osd) = action {
                    if let Some(task) = task {
                        tasks.push(task.map(Message::Settings));
                    }
                    // Show OSD overlay if enabled.
                    if self.osd.config().enabled
                        && !cmd.no_osd()
                        && let Some(osd) = osd
                        && let osd::Action::Show(timer) = self.osd.update(osd::Message::Show(osd))
                    {
                        tasks.push(timer.map(Message::Osd));
                        tasks.push(self.outputs.show_osd_layer(OSD_WIDTH, OSD_HEIGHT));
                    }
                }
                Task::batch(tasks)
            }
            Message::Osd(msg) => match self.osd.update(msg) {
                osd::Action::Hide => self.outputs.hide_osd_layer(),
                _ => Task::none(),
            },
            Message::None => Task::none(),
            Message::ToggleVisibility => {
                self.visible = !self.visible;
                let (bar_style, scale_factor) = use_theme(|t| (t.bar_style, t.scale_factor));
                let height = if self.visible {
                    (crate::HEIGHT
                        - match bar_style {
                            AppearanceStyle::Solid => 8.,
                            AppearanceStyle::Islands => 0.,
                        })
                        * scale_factor
                } else {
                    0.0
                };

                Task::batch(
                    self.outputs
                        .iter()
                        .filter_map(|(_, shell_info, _)| {
                            shell_info
                                .as_ref()
                                .map(|info| set_exclusive_zone(info.id, height as i32))
                        })
                        .collect::<Vec<_>>(),
                )
            }
            Message::SaveState => {
                self.settings.save_state();
                println!("Saving state.");
                Task::Iced(iced_runtime::exit())
            }
        }
    }

    pub fn view(&'_ self, id: SurfaceId) -> Element<'_, Message> {
        match self.outputs.has(id) {
            Some(HasOutput::Main) => {
                if !self.visible {
                    return Row::new().into();
                }

                let [left, center, right] = self.modules_section(id);

                let (space, bar_style, opacity, menu) =
                    use_theme(|t| (t.space, t.bar_style, t.opacity, t.menu));
                let centerbox = Centerbox::new([left, center, right])
                    .spacing(space.xxs)
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .height(if bar_style == AppearanceStyle::Islands {
                        HEIGHT
                    } else {
                        HEIGHT - space.xs as f64
                    } as f32)
                    .padding(if bar_style == AppearanceStyle::Islands {
                        [space.xxs, space.xxs]
                    } else {
                        [0.0, 0.0]
                    });

                let menu_is_open = self.outputs.menu_is_open();
                let status_bar = container(centerbox).style(move |t: &Theme| container::Style {
                    background: match bar_style {
                        AppearanceStyle::Solid => Some({
                            let bg = t.palette().background.scale_alpha(opacity);
                            if menu_is_open {
                                darken_color(bg, menu.backdrop)
                            } else {
                                bg
                            }
                            .into()
                        }),
                        AppearanceStyle::Islands => {
                            if menu_is_open {
                                Some(backdrop_color(menu.backdrop).into())
                            } else {
                                None
                            }
                        }
                    },
                    ..Default::default()
                });

                if self.outputs.menu_is_open() {
                    mouse_area(status_bar)
                        .on_release(Message::CloseMenu(id))
                        .into()
                } else {
                    status_bar.into()
                }
            }
            Some(HasOutput::Menu(Some(open_menu))) => {
                let ui_ref = open_menu.button_ui_ref;
                match &open_menu.menu_type {
                    MenuType::Settings => self.menu_wrapper(
                        id,
                        self.settings.menu_view(id).map(Message::Settings),
                        ui_ref,
                    ),
                    MenuType::SystemInfo => self.menu_wrapper(
                        id,
                        self.system_info.menu_view().map(Message::SystemInfo),
                        ui_ref,
                    ),
                    MenuType::Clock => {
                        self.menu_wrapper(id, self.clock.menu_view().map(Message::Clock), ui_ref)
                    }
                }
            }
            Some(HasOutput::Menu(None)) => Row::new().into(),
            Some(HasOutput::Osd) => self.osd.view().map(Message::Osd),
            None => Row::new().into(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            Subscription::batch(self.modules_subscriptions(&self.general_config.modules.left)),
            Subscription::batch(self.modules_subscriptions(&self.general_config.modules.center)),
            Subscription::batch(self.modules_subscriptions(&self.general_config.modules.right)),
            config::subscription(&self.config_path),
            crate::services::logind::LogindService::subscribe().map(|event| match event {
                crate::services::ServiceEvent::Update(_) => Message::ResumeFromSleep,
                _ => Message::None,
            }),
            iced::output_events().map(Message::OutputEvent),
            Subscription::run(|| {
                use iced::futures::StreamExt;
                signal_hook_tokio::Signals::new([libc::SIGTERM, libc::SIGINT])
                    .expect("Failed to create signal stream")
                    .filter_map(|sig| {
                        iced::futures::future::ready(match sig {
                            libc::SIGTERM | libc::SIGINT => Some(Message::SaveState),
                            _ => None,
                        })
                    })
            }),
            // Always subscribe to audio/brightness services so OSD works
            // even when the Settings module isn't in the module list.
            self.settings.subscription().map(Message::Settings),
            crate::ipc::subscription().map(|cmd| match cmd {
                IpcCommand::ToggleVisibility => Message::ToggleVisibility,
                other => Message::IpcOsdCommand(other),
            }),
        ])
    }
}
