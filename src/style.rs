use iced::widget::{container, text_input};
use iced::{Background, Color, Gradient, Radians, Shadow, Vector, border, gradient};

pub(crate) fn input_shell(failed: bool) -> container::Style {
    let (background, border_color, shadow_color, shadow_offset, shadow_blur) = if failed {
        (
            liquid_gradient(
                Color::from_rgba(0.42, 0.04, 0.08, 0.58),
                Color::from_rgba(0.13, 0.00, 0.02, 0.74),
            ),
            Color::from_rgba(1.0, 0.56, 0.62, 0.76),
            Color::from_rgba(0.28, 0.0, 0.04, 0.50),
            Vector::new(0.0, 16.0),
            38.0,
        )
    } else {
        (
            liquid_gradient(
                Color::from_rgba(0.46, 0.72, 0.82, 0.68),
                Color::from_rgba(0.01, 0.05, 0.09, 0.86),
            ),
            Color::from_rgba(0.78, 0.96, 1.0, 0.68),
            Color::from_rgba(0.0, 0.03, 0.06, 0.58),
            Vector::new(0.0, 18.0),
            42.0,
        )
    };

    container::Style::default()
        .background(background)
        .border(border::rounded(32.0).width(1.15).color(border_color))
        .shadow(Shadow {
            color: shadow_color,
            offset: shadow_offset,
            blur_radius: shadow_blur,
        })
}

pub(crate) fn password_input_style(_status: text_input::Status, failed: bool) -> text_input::Style {
    text_input::Style {
        background: Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.0)),
        border: border::rounded(28.0)
            .width(0.0)
            .color(Color::from_rgba(0.0, 0.0, 0.0, 0.0)),
        icon: Color::WHITE,
        placeholder: if failed {
            Color::from_rgba(1.0, 0.84, 0.86, 0.94)
        } else {
            Color::from_rgba(0.92, 0.98, 1.0, 0.70)
        },
        value: Color::WHITE,
        selection: Color::from_rgba(0.62, 0.88, 1.0, 0.44),
    }
}

fn liquid_gradient(top: Color, bottom: Color) -> Background {
    Background::Gradient(Gradient::Linear(
        gradient::Linear::new(Radians(1.65))
            .add_stop(0.0, top)
            .add_stop(0.42, Color::from_rgba(0.12, 0.32, 0.42, 0.64))
            .add_stop(1.0, bottom),
    ))
}
