use crate::{
    components::icons::{IconKind, StaticIcon},
    theme::use_theme,
};
use iced::{
    Alignment, Element,
    mouse::ScrollDelta,
    widget::{MouseArea, Row, slider},
};

pub struct BrightnessSliderControl<Msg, F1>
where
    F1: Fn(u8) -> Msg + Clone,
{
    value: u8,
    on_change: F1,
}

pub fn brightness_slider_control<'a, Msg: 'static + Clone, F1>(value: u8, on_change: F1) -> BrightnessSliderControl<Msg, F1>
where
    F1: Fn(u8) -> Msg + Clone + 'a,
{
    BrightnessSliderControl { value, on_change }
}

impl<'a, Msg: 'static + Clone, F1> From<BrightnessSliderControl<Msg, F1>> for Element<'a, Msg>
where
    F1: Fn(u8) -> Msg + Clone + 'a,
{
    fn from(ctrl: BrightnessSliderControl<Msg, F1>) -> Self {
        let space = use_theme(|theme| theme.space);

        let icon: IconKind = StaticIcon::Brightness.into();
        let icon_element: Element<'a, Msg> =
            iced::widget::container(icon.to_text_mono()).center_x(32.).center_y(32.).clip(true).into();

        let slider_element = MouseArea::new(Element::<'a, Msg>::from(slider(0..=100, ctrl.value, ctrl.on_change.clone())))
            .on_scroll(brightness_on_scroll(ctrl.value, ctrl.on_change.clone()));

        Row::with_capacity(3).push(icon_element).push(slider_element).align_y(Alignment::Center).spacing(space.xs).into()
    }
}

fn brightness_on_scroll<F, Msg: 'static + Clone>(current: u8, make_msg: F) -> impl Fn(ScrollDelta) -> Msg
where
    F: Fn(u8) -> Msg,
{
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
        make_msg(new)
    }
}
