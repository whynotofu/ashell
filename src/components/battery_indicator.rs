use crate::components::icons::StaticIcon;
use crate::services::device::BatteryStatus;
use iced::{
    Color, Font, Pixels, Point, Rectangle,
    advanced::text::Paragraph,
    border::Radius,
    widget::canvas,
    widget::canvas::{Text, gradient::Linear},
};

pub struct BatteryIndicator {
    pub percent: u8,
    pub status: BatteryStatus,
}

impl<Message> canvas::Program<Message> for BatteryIndicator {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        let padding = 3.0;

        let pill = canvas::Path::rounded_rectangle(
            [0.0, padding].into(),
            [bounds.width, bounds.height - (padding * 2.0)].into(),
            Radius::from(8.0),
        );

        let fill_width = (self.percent as f32) / 100.0;

        let start = Point::new(0.0, padding);
        let end = Point::new(bounds.width, padding);

        let color = match self.status {
            BatteryStatus::Charging => Color::from_rgb(0.2, 0.8, 1.0),
            BatteryStatus::Charged | BatteryStatus::BatteryProtection => Color::from_rgb(0.2, 0.8, 0.2),
            BatteryStatus::Discharging => match self.percent {
                (0..15) => Color::from_rgb(1.0, 0.3, 0.3),
                (15..40) => Color::from_rgb(0.8, 0.8, 0.3),
                (40..) => Color::from_rgb(0.2, 0.8, 0.2),
            },
        };

        let gradient = Linear::new(start, end)
            .add_stop(0.0, color)
            .add_stop(fill_width, color)
            .add_stop(fill_width + 0.0001, Color::from_rgb(0.55, 0.55, 0.55))
            .add_stop(1.0, Color::from_rgb(0.55, 0.55, 0.55));

        frame.fill(&pill, gradient);

        let text = match self.status {
            BatteryStatus::Charging => format!("{} {}", StaticIcon::Lightning.get_str(), self.percent),
            //BatteryStatus::BatteryProtection => format!("{} {}", StaticIcon::Shield.get_str(), self.percent),
            _ => self.percent.to_string(),
        };

        let size = Pixels(12.0);
        let font = Font {
            weight: iced::font::Weight::Semibold,
            ..Font::default()
        };

        let text_layout = iced::advanced::text::Text {
            content: text.as_str(),
            bounds: iced::Size::INFINITE,
            size,
            font,
            line_height: iced::advanced::text::LineHeight::default(),
            align_x: iced::text::Alignment::Left,
            align_y: iced::alignment::Vertical::Top,
            shaping: iced::advanced::text::Shaping::Basic,
            wrapping: iced::advanced::text::Wrapping::None,
        };

        let paragraph = <<iced::Renderer as iced::advanced::text::Renderer>::Paragraph>::with_text(text_layout);

        let text_size = paragraph.min_bounds();
        let text_width = text_size.width;
        let text_height = text_size.height;

        frame.fill_text(Text {
            content: text,
            position: Point::new((bounds.width - text_width) / 2.0, (bounds.height - text_height) / 2.0),
            color: Color::from_rgb(0.0, 0.0, 0.0),
            size,
            font,
            ..Text::default()
        });

        vec![frame.into_geometry()]
    }
}

/*
use iced::{
    Border, Color, Element, Length, Rectangle, Size, Theme,
    advanced::{
        Widget,
        layout::{self, Layout, Node},
        renderer,
        widget::Tree,
    },
    widget::text::Shaping,
    widget::{column, container, text},
};

#[derive(Debug, Clone, Copy)]
pub struct BatteryWidget {
    pub level: u8, // 0.0 - 1.0
    pub charging: bool,
}

impl BatteryWidget {
    pub fn new(level: u8, charging: bool) -> Self {
        Self { level, charging }
    }
}

impl<Message, Renderer> Widget<Message, Theme, Renderer> for BatteryWidget
where
    Renderer: renderer::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &Renderer, _limits: &layout::Limits) -> Node {
        layout::Node::new(Size::new(34.0, 24.0))
    }

    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        let level = self.level.clamp(0, 100);

        // background
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    radius: 16.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                ..Default::default()
            },
            Color::from_rgb(0.1, 0.1, 0.1),
        );

        // fill bar
        let fill_width = bounds.width * (level as f32 / 100.0);

        let color = if self.charging {
            Color::from_rgb(0.2, 0.8, 1.0)
        } else if level > 60 {
            Color::from_rgb(0.2, 0.8, 0.2)
        } else if level > 30 {
            Color::from_rgb(1.0, 0.8, 0.2)
        } else {
            Color::from_rgb(1.0, 0.3, 0.3)
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x,
                    y: bounds.y,
                    width: fill_width,
                    height: bounds.height,
                },
                border: Border {
                    radius: 10.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                ..Default::default()
            },
            color,
        );
    }
}

impl<'a, Message> From<BatteryWidget> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(battery: BatteryWidget) -> Self {
        Self::new(battery)
    }
}
*/
