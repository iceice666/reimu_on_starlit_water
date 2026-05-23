use iced::widget::{container, text_input};
use iced::{Background, Color, Shadow, Vector, border};

pub(crate) fn clock_glass() -> container::Style {
    container::Style::default()
        .background(Background::Color(Color::from_rgba(
            0.01, 0.014, 0.022, 0.62,
        )))
        .border(
            border::rounded(36.0)
                .width(1.0)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.22)),
        )
        .shadow(Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.52),
            offset: Vector::new(0.0, 18.0),
            blur_radius: 42.0,
        })
}

pub(crate) fn input_shell(failed: bool) -> container::Style {
    let (background, border_color, shadow_color) = if failed {
        (
            Color::from_rgba(0.22, 0.01, 0.02, 0.66),
            Color::from_rgba(1.0, 0.50, 0.56, 0.68),
            Color::from_rgba(0.18, 0.0, 0.0, 0.52),
        )
    } else {
        (
            Color::from_rgba(0.01, 0.014, 0.022, 0.62),
            Color::from_rgba(1.0, 1.0, 1.0, 0.23),
            Color::from_rgba(0.0, 0.0, 0.0, 0.52),
        )
    };

    container::Style::default()
        .background(Background::Color(background))
        .border(border::rounded(32.0).width(1.0).color(border_color))
        .shadow(Shadow {
            color: shadow_color,
            offset: Vector::new(0.0, 14.0),
            blur_radius: 34.0,
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
            Color::from_rgba(1.0, 0.82, 0.84, 0.92)
        } else {
            Color::from_rgba(1.0, 1.0, 1.0, 0.58)
        },
        value: Color::WHITE,
        selection: Color::from_rgba(0.38, 0.70, 1.0, 0.42),
    }
}
