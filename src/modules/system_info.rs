use crate::{
    components::MenuSize,
    components::divider,
    components::icons::{StaticIcon, icon},
    t,
    theme::use_theme,
};
use iced::{
    Alignment, Element, Length, Subscription, Theme,
    time::every,
    widget::{Column, Row, column, container, row, text},
};
use itertools::Itertools;
use std::time::{Duration, Instant};
use sysinfo::{Components, Networks};

const MAX_IP_LEN: usize = 45;

#[derive(Clone, Copy)]
struct FixedIp([u8; MAX_IP_LEN], usize);

impl FixedIp {
    fn from_str(s: &str) -> Option<Self> {
        if s.len() < MAX_IP_LEN {
            let mut arr = [0u8; MAX_IP_LEN];
            arr[..s.len()].copy_from_slice(s.as_bytes());
            Some(Self(arr, s.len()))
        } else {
            None
        }
    }
}

impl std::fmt::Display for FixedIp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::str::from_utf8(&self.0[..self.1]).unwrap_or(""))
    }
}

struct NetworkData {
    #[allow(dead_code)]
    ip: FixedIp,
    download_speed: u32,
    upload_speed: u32,
    last_check: Instant,
}

struct SystemInfoData {
    network: Option<NetworkData>,
}

#[allow(clippy::too_many_arguments)]
fn get_system_info(components: &mut Components, (networks, last_check): (&mut Networks, Option<Instant>)) -> SystemInfoData {
    components.refresh(true);
    networks.refresh(true);

    let elapsed = last_check.map(|v| v.elapsed().as_secs());

    let network = networks
        .iter()
        .filter(|(name, _)| {
            name.contains("en") || name.contains("eth") || name.contains("wl") || name.contains("wlan") || name.contains("br")
        })
        .sorted_by_key(|(name, _)| {
            if name.contains("en") {
                return 0;
            }

            if name.contains("eth") {
                return 1;
            }

            if name.contains("wl") {
                return 2;
            }

            if name.contains("wlan") {
                return 3;
            }
            if name.contains("br") {
                return 4;
            }

            99
        })
        .fold((None, 0, 0), |(first_ip, total_received, total_transmitted), (_, data)| {
            let ip =
                first_ip.or_else(|| data.ip_networks().iter().sorted_by(|a, b| a.addr.cmp(&b.addr)).next().map(|ip| ip.addr));

            let received = data.received();
            let transmitted = data.transmitted();

            (first_ip.or(ip), total_received + received, total_transmitted + transmitted)
        });

    let network_speed = |value: u64| {
        match elapsed {
            None | Some(0) => 0, // avoid division by zero
            Some(elapsed) => (value / 1000) as u32 / elapsed as u32,
        }
    };

    SystemInfoData {
        network: network.0.and_then(|ip| {
            let ip_str = ip.to_string();
            FixedIp::from_str(&ip_str).map(|ip| NetworkData {
                ip,
                download_speed: network_speed(network.1),
                upload_speed: network_speed(network.2),
                last_check: Instant::now(),
            })
        }),
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Update,
}

#[derive(Clone, Debug)]
pub enum SystemInfoIndicator {
    DownloadSpeed,
    UploadSpeed,
}

#[derive(Clone, Debug)]
pub struct SystemInfoModuleConfig {
    pub indicators: Vec<SystemInfoIndicator>,
    pub interval: u64,
}

pub struct SystemInfo {
    config: SystemInfoModuleConfig,
    components: Components,
    networks: Networks,
    data: SystemInfoData,
}

impl SystemInfo {
    pub fn new() -> Self {
        let config = SystemInfoModuleConfig {
            indicators: vec![SystemInfoIndicator::DownloadSpeed, SystemInfoIndicator::UploadSpeed],
            interval: 1,
        };

        let mut components = Components::new_with_refreshed_list();
        let mut networks = Networks::new_with_refreshed_list();

        let data = get_system_info(&mut components, (&mut networks, None));

        Self {
            config,
            components,
            data,
            networks,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Update => {
                self.data = get_system_info(
                    &mut self.components,
                    (&mut self.networks, self.data.network.as_ref().map(|n| n.last_check)),
                );
            }
        }
    }

    fn info_element<'a>(info_icon: StaticIcon, label: String, value: String) -> Element<'a, Message> {
        let (font_size, space) = use_theme(|t| (t.font_size, t.space));
        row!(
            container(icon(info_icon).size(font_size.xl)).center_x(Length::Fixed(space.xl)),
            text(label).width(Length::Fill),
            text(value)
        )
        .align_y(Alignment::Center)
        .spacing(space.xs)
        .into()
    }

    fn indicator_info_element<'a, V: PartialOrd + 'a>(
        info_icon: StaticIcon,
        (display, unit): (impl std::fmt::Display + 'a, &str),
        threshold: Option<(V, V, V)>,
        prefix: Option<String>,
    ) -> Element<'a, Message> {
        let space = use_theme(|t| t.space);
        let element = container(
            row!(
                icon(info_icon),
                if let Some(prefix) = prefix {
                    text(format!("{prefix} {display}{unit}"))
                } else {
                    text(format!("{display}{unit}"))
                }
            )
            .spacing(space.xxs),
        );

        if let Some((value, warn_threshold, alert_threshold)) = threshold {
            element
                .style(move |theme: &Theme| container::Style {
                    text_color: if value > warn_threshold && value < alert_threshold {
                        Some(theme.palette().warning)
                    } else if value >= alert_threshold {
                        Some(theme.palette().danger)
                    } else {
                        None
                    },
                    ..Default::default()
                })
                .into()
        } else {
            element.into()
        }
    }

    pub fn menu_view(&'_ self) -> Element<'_, Message> {
        let (font_size, space) = use_theme(|t| (t.font_size, t.space));
        container(
            column!(
                text(t!("system-info-heading")).size(font_size.lg),
                divider(),
                Column::with_capacity(6)
                    .push(self.data.network.as_ref().map(|network| {
                        Column::with_children(vec![
                            Self::info_element(
                                StaticIcon::DownloadSpeed,
                                t!("system-info-download-speed"),
                                if network.download_speed > 1000 {
                                    format!("{} MB/s", network.download_speed / 1000)
                                } else {
                                    format!("{} KB/s", network.download_speed)
                                },
                            ),
                            Self::info_element(
                                StaticIcon::UploadSpeed,
                                t!("system-info-upload-speed"),
                                if network.upload_speed > 1000 {
                                    format!("{} MB/s", network.upload_speed / 1000)
                                } else {
                                    format!("{} KB/s", network.upload_speed)
                                },
                            ),
                        ])
                    }))
                    .spacing(space.xxs)
                    .padding([0.0, space.xs])
            )
            .spacing(space.xs),
        )
        .width(MenuSize::Medium)
        .into()
    }

    pub fn view(&'_ self) -> Element<'_, Message> {
        let space = use_theme(|t| t.space);
        let indicators = self.config.indicators.iter().filter_map(|i| match i {
            SystemInfoIndicator::DownloadSpeed => self.data.network.as_ref().map(|network| {
                Self::indicator_info_element(
                    StaticIcon::DownloadSpeed,
                    (
                        if network.download_speed > 1000 {
                            network.download_speed / 1000
                        } else {
                            network.download_speed
                        },
                        if network.download_speed > 1000 { "MB/s" } else { "KB/s" },
                    ),
                    None::<(u32, u32, u32)>,
                    None,
                )
            }),
            SystemInfoIndicator::UploadSpeed => self.data.network.as_ref().map(|network| {
                Self::indicator_info_element(
                    StaticIcon::UploadSpeed,
                    (
                        if network.upload_speed > 1000 {
                            network.upload_speed / 1000
                        } else {
                            network.upload_speed
                        },
                        if network.upload_speed > 1000 { "MB/s" } else { "KB/s" },
                    ),
                    None::<(u32, u32, u32)>,
                    None,
                )
            }),
        });

        Row::with_children(indicators).align_y(Alignment::Center).spacing(space.xxs).into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        every(Duration::from_secs(self.config.interval)).map(|_| Message::Update)
    }
}
