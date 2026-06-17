use crate::{
    components::{
        ButtonKind, MenuSize,
        icons::{StaticIcon, icon_button},
    },
    config::ClockModuleConfig,
    i18n::chrono_locale,
    theme::{AshellTheme, use_theme},
};
use chrono::{DateTime, Datelike, Days, Local, Months, NaiveDate, TimeZone, Utc, Weekday};
use iced::{
    Background, Border, Color, Element, Length, Subscription, Theme,
    alignment::{Horizontal, Vertical},
    futures::SinkExt,
    mouse::ScrollDelta,
    stream::channel,
    widget::{Column, MouseArea, Row, Space, button, column, container, row, text},
};

use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Message {
    Update,
    Reset,
    ChangeFormat(Direction),
    NextFormat,
    ChangeMonth(Direction),
    ConfigReloaded(ClockModuleConfig),
    None,
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Next,
    Previous,
}

pub enum Action {
    None,
}

pub struct Clock {
    config: ClockModuleConfig,
    date: DateTime<Local>,
    current_month: Option<NaiveDate>,
    current_format_index: usize,
}

impl Clock {
    pub fn new(config: ClockModuleConfig) -> Self {
        Self {
            config,
            date: Local::now(),
            current_month: None,
            current_format_index: 0,
        }
    }

    fn current_format(&self) -> &str {
        if !self.config.formats.is_empty() {
            self.config
                .formats
                .get(self.current_format_index)
                .or_else(|| self.config.formats.first())
                .unwrap_or(&self.config.clock_format)
        } else {
            &self.config.clock_format
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Update => {
                self.date = Local::now();
                Action::None
            }
            Message::Reset => {
                self.current_month = Some(self.date.date_naive());
                Action::None
            }
            Message::ChangeFormat(direction) => {
                let format_index = self.current_format_index as i32;
                self.current_format_index = match direction {
                    Direction::Next => (format_index + 1).min(self.config.formats.len() as i32 - 1),
                    Direction::Previous => (format_index - 1).max(0),
                } as usize;
                Action::None
            }
            Message::NextFormat => {
                if !self.config.formats.is_empty() {
                    self.current_format_index = (self.current_format_index + 1) % self.config.formats.len();
                }
                Action::None
            }
            Message::ChangeMonth(direction) => {
                self.current_month = if let Some(current_month) = self.current_month {
                    match direction {
                        Direction::Next => current_month.checked_add_months(Months::new(1)),
                        Direction::Previous => current_month.checked_sub_months(Months::new(1)),
                    }
                } else {
                    None
                };
                Action::None
            }
            Message::ConfigReloaded(new_config) => {
                self.current_format_index = 0;
                self.config = new_config;
                Action::None
            }
            Message::None => Action::None,
        }
    }

    pub fn view(&'_ self) -> Element<'_, Message> {
        let format = self.current_format();
        let naive_utc_now = self.date.with_timezone(&Utc).naive_utc();
        let locale = chrono_locale();

        let datetime_string = Local.from_utc_datetime(&naive_utc_now).format_localized(format, locale).to_string();

        container(text(datetime_string)).align_y(Vertical::Center).into()
    }

    pub fn menu_view<'a>(&'a self) -> Element<'a, Message> {
        //let space = use_theme(|t| t.space);
        container(self.calendar()).max_width(MenuSize::XLarge).into()
    }

    fn calendar<'a>(&'a self) -> Element<'a, Message> {
        use_theme(|theme| self.calendar_with_theme(theme))
    }

    fn calendar_with_theme<'a>(&'a self, theme: &AshellTheme) -> Element<'a, Message> {
        let locale = chrono_locale();
        let current_month = self.current_month.unwrap_or(self.date.date_naive());

        let first_day_month = current_month.with_day0(0).unwrap_or_default();
        let day_of_week_first_day = first_day_month.weekday();

        let mut current = first_day_month
            .checked_sub_days(Days::new(day_of_week_first_day.num_days_from_monday() as u64))
            .unwrap_or_default();

        let weeks_in_month = 6; /*if current
        .checked_add_days(Days::new(5 * 7))
        .map(|d| d.month0())
        .unwrap_or_default()
        != current_month.month0()
        {
        5
        } else {
        6
        };*/

        let calendar = container(
            MouseArea::new(
                column![
                    row![
                        icon_button::<Message>(StaticIcon::LeftChevron)
                            .kind(ButtonKind::Solid)
                            .on_press(Message::ChangeMonth(Direction::Previous)),
                        text(current_month.format_localized("%B, %Y", locale).to_string())
                            .size(theme.font_size.md)
                            .width(Length::Fill)
                            .align_x(Horizontal::Center),
                        icon_button::<Message>(StaticIcon::RightChevron)
                            .kind(ButtonKind::Solid)
                            .on_press(Message::ChangeMonth(Direction::Next)),
                    ]
                    .width(Length::Fill)
                    .align_y(Vertical::Center),
                    Row::with_children(
                        [
                            Weekday::Mon,
                            Weekday::Tue,
                            Weekday::Wed,
                            Weekday::Thu,
                            Weekday::Fri,
                            Weekday::Sat,
                            Weekday::Sun,
                        ]
                        .into_iter()
                        .map(|i| {
                            text(
                                NaiveDate::from_isoywd_opt(2000, 20, i)
                                    .expect("valid NaiveDate")
                                    .format_localized("%a", locale)
                                    .to_string(),
                            )
                            .align_x(Horizontal::Center)
                            .width(Length::Fill)
                            .into()
                        })
                        .collect::<Vec<Element<'a, Message>>>(),
                    )
                    .width(Length::Fill)
                    .spacing(theme.space.sm),
                    Column::with_children(
                        (0..weeks_in_month)
                            .map(|_| {
                                Row::with_children(
                                    (0..7)
                                        .map(|_| {
                                            let day = current;
                                            current = current.succ_opt().unwrap_or(current);

                                            let (background_color, text_color) = match day == self.date.date_naive() {
                                                true => {
                                                    (theme.iced_theme.palette().primary, theme.iced_theme.palette().background)
                                                }
                                                false => {
                                                    (theme.iced_theme.palette().background, theme.iced_theme.palette().text)
                                                }
                                            };

                                            if day.month0() == current_month.month0() {
                                                button(
                                                    text(day.format_localized("%-d", locale).to_string())
                                                        .align_x(Horizontal::Center),
                                                )
                                                .style(move |_t: &Theme, _status: button::Status| {
                                                    button::Style {
                                                        background: Some(Background::Color(background_color)),
                                                        text_color: text_color,
                                                        border: Border {
                                                            color: Color::TRANSPARENT,
                                                            width: 0.,
                                                            radius: (4.).into(), //*theme.iced_theme.radius.sm.into(),
                                                        },
                                                        ..Default::default()
                                                    }
                                                })
                                                .width(Length::Fill)
                                                .into()
                                            } else {
                                                Space::new().width(Length::Fill).into()
                                            }
                                        })
                                        .collect::<Vec<Element<'a, Message>>>(),
                                )
                                .spacing(theme.space.xs)
                                .width(Length::Fill)
                                .height(32.)
                                .align_y(Vertical::Center)
                                .into()
                            })
                            .collect::<Vec<Element<'a, Message>>>(),
                    )
                    .spacing(theme.space.xs),
                ]
                .spacing(theme.space.md),
            )
            .on_scroll(Self::calendar_on_scroll()),
        );

        calendar.width(253).into()
    }

    fn calendar_on_scroll() -> impl Fn(ScrollDelta) -> Message {
        move |delta| match delta {
            ScrollDelta::Lines { y, .. } => {
                let direction = match y > 0. {
                    true => Direction::Next,
                    false => Direction::Previous,
                };
                Message::ChangeMonth(direction)
            }
            _ => Message::None,
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let second_specifiers = [
            "%S",  // Seconds (00-60)
            "%T",  // Hour:Minute:Second
            "%X",  // Locale time representation with seconds
            "%r",  // 12-hour clock time with seconds
            "%:z", // UTC offset with seconds
            "%s",  // Unix timestamp (seconds since epoch)
        ];

        let current_format = self.current_format();

        let interval = if second_specifiers.iter().any(|&spec| current_format.contains(spec)) {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(5)
        };

        Subscription::run_with(interval, |interval| {
            let interval = *interval;
            channel(100, async move |mut output: iced::futures::channel::mpsc::Sender<Message>| {
                let mut interval = tokio::time::interval(interval);
                loop {
                    interval.tick().await;
                    output.send(Message::Update).await.ok();
                }
            })
        })
    }
}
