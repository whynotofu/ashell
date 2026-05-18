use crate::{
    components::{
        ButtonSize,
        icons::{IconKind, StaticIcon, icon_button},
    },
    modules::settings::SubMenu,
    theme::use_theme,
};
use iced::{
    Alignment, Element, Length, Padding,
    widget::{Column, MouseArea, Row, button, container, row, text},
};

#[allow(clippy::too_many_arguments)]
pub fn quick_setting_button<'a, Msg: Clone + 'static>(
    icon: impl Into<IconKind>,
    title: String,
    subtitle: Option<String>,
    active: bool,
    on_press: Msg,
    on_right_press: Option<Msg>,
    with_submenu: Option<(SubMenu, Option<SubMenu>, Msg)>,
) -> Element<'a, Msg> {
    let (space, font_size, submenu_btn_style, settings_btn_style, icon_container_style) =
        use_theme(|theme| {
            (
                theme.space,
                theme.font_size,
                theme.quick_settings_submenu_button_style(active),
                theme.quick_settings_button_style(active),
                theme.quick_settings_icon_container_style(active),
            )
        });

    let main_content = row!(
        container(icon.into().to_text().size(font_size.lg))
            .center_x(32.)
            .center_y(32.)
            .style(icon_container_style),
        container(
            Column::with_capacity(2)
                .push(text(title).size(font_size.sm))
                .push(
                    subtitle.map(|s| { text(s).wrapping(text::Wrapping::None).size(font_size.xs) })
                )
                .spacing(space.xxs)
        )
        .clip(true)
    )
    .spacing(space.xs)
    .padding(Padding::ZERO.left(2.))
    .width(Length::Fill)
    .align_y(Alignment::Center);

    let content = if let Some((submenu1, submenu2, msg)) = with_submenu {
        Row::with_capacity(2)
            .push(main_content)
            .push(
                icon_button(if Some(submenu1) == submenu2 {
                    StaticIcon::Close
                } else {
                    StaticIcon::RightChevron
                })
                .on_press(msg)
                .size(ButtonSize::Small)
                .style(submenu_btn_style),
            )
            .spacing(space.xxs)
            .align_y(Alignment::Center)
            .height(Length::Fill)
    } else {
        Row::with_capacity(1)
            .push(main_content)
            .align_y(Alignment::Center)
            .height(Length::Fill)
    };

    let btn = button(content)
        .padding([space.xxs, space.xs])
        .on_press(on_press)
        .style(settings_btn_style)
        .width(Length::Fill)
        .height(Length::Fixed(50.));

    if let Some(on_right_press) = on_right_press {
        MouseArea::new(btn).on_right_press(on_right_press).into()
    } else {
        btn.into()
    }
}
