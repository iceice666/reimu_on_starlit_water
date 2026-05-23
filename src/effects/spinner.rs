use std::time::Duration;

use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};

use crate::math::{ease_out_cubic, lerp};

const SPINNER_PETALS: usize = 12;
const SPINNER_REVOLUTION: Duration = Duration::from_millis(1100);
const SPINNER_FADE_IN: Duration = Duration::from_millis(180);

#[derive(Debug, Clone, Copy)]
pub(crate) struct FlowerSpinner {
    elapsed: Duration,
}

impl FlowerSpinner {
    pub(crate) fn new(elapsed: Duration) -> Self {
        Self { elapsed }
    }
}

impl<Message> canvas::Program<Message> for FlowerSpinner {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center = frame.center();
        let elapsed = self.elapsed.as_secs_f32();
        let phase = (elapsed / SPINNER_REVOLUTION.as_secs_f32()).fract();
        let rotation = phase * std::f32::consts::TAU;
        let appear = ease_out_cubic((elapsed / SPINNER_FADE_IN.as_secs_f32()).clamp(0.0, 1.0));
        let pulse = 1.0 + (elapsed * std::f32::consts::TAU * 1.35).sin() * 0.025;
        let scale = lerp(0.84, 1.0, appear) * pulse;

        let glow = canvas::Path::circle(center, 19.0 * scale);
        frame.fill(&glow, Color::from_rgba(0.58, 0.74, 1.0, 0.08 * appear));

        let ring = canvas::Path::circle(center, 20.0 * scale);
        frame.stroke(
            &ring,
            canvas::Stroke::default()
                .with_width(1.0)
                .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.12 * appear)),
        );

        frame.with_save(|frame| {
            frame.translate(Vector::new(center.x, center.y));
            frame.scale(scale);

            for petal in 0..SPINNER_PETALS {
                let trail = petal as f32 / SPINNER_PETALS as f32;
                let intensity = (1.0 - trail).powf(2.35);
                let alpha = (0.14 + intensity * 0.76) * appear;
                let angle = rotation - std::f32::consts::TAU * trail;
                let width = lerp(3.1, 5.0, intensity);
                let length = lerp(9.5, 15.5, intensity);
                let offset = lerp(17.0, 22.5, intensity);
                let color = Color::from_rgba(
                    lerp(0.72, 1.0, intensity),
                    lerp(0.84, 1.0, intensity),
                    1.0,
                    alpha,
                );

                frame.with_save(|frame| {
                    frame.rotate(angle);

                    let petal = canvas::Path::rounded_rectangle(
                        Point::new(-width / 2.0, -offset),
                        Size::new(width, length),
                        (width / 2.0).into(),
                    );

                    frame.fill(&petal, color);
                });
            }
        });

        let core = canvas::Path::circle(center, 2.5 * scale);
        frame.fill(&core, Color::from_rgba(1.0, 1.0, 1.0, 0.52 * appear));

        vec![frame.into_geometry()]
    }
}
