use std::time::Duration;

use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};

use crate::math::{ease_out_cubic, lerp};

const SPINNER_PETALS: usize = 12;
const SPINNER_REVOLUTION: Duration = Duration::from_millis(1100);
const SPINNER_FADE_IN: Duration = Duration::from_millis(180);

const SPINNER_OUTER_GLOW_RADIUS: f32 = 21.0;
const SPINNER_RING_RADIUS: f32 = 20.0;
const SPINNER_INNER_RING_RADIUS: f32 = 13.5;
const SPINNER_HALO_RING_RADIUS: f32 = 10.2;
const SPINNER_RING_GAP: f32 = 2.6;

const SPINNER_CORE_GLOW_RADIUS: f32 = 3.0;
const SPINNER_CORE_RADIUS: f32 = 2.3;

const SPINNER_PULSE_RATE: f32 = 1.35;
const SPINNER_GLOW_WAVE: f32 = 0.026;

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
        let appear = ease_out_cubic((elapsed / SPINNER_FADE_IN.as_secs_f32()).clamp(0.0, 1.0));
        let pulse = 1.0 + (elapsed * std::f32::consts::TAU * SPINNER_PULSE_RATE).sin() * SPINNER_GLOW_WAVE;
        let ring_breathe = (elapsed * 0.65).sin() * 0.25;
        let scale = lerp(0.84, 1.0, appear) * pulse;

        let glow = canvas::Path::circle(center, SPINNER_OUTER_GLOW_RADIUS * scale);
        frame.fill(
            &glow,
            Color::from_rgba(0.58, 0.86, 1.0, 0.10 * appear),
        );

        let ring = canvas::Path::circle(center, SPINNER_RING_RADIUS * scale);
        frame.stroke(
            &ring,
            canvas::Stroke::default()
                .with_width(1.2)
                .with_color(Color::from_rgba(0.95, 1.0, 1.0, 0.22 * appear)),
        );

        let inner_ring = canvas::Path::circle(center, SPINNER_INNER_RING_RADIUS * scale);
        frame.stroke(
            &inner_ring,
            canvas::Stroke::default()
                .with_width(0.82)
                .with_color(Color::from_rgba(0.58, 0.84, 1.0, 0.17 * appear)),
        );

        let halo_ring = canvas::Path::circle(center, SPINNER_HALO_RING_RADIUS * scale);
        frame.stroke(
            &halo_ring,
            canvas::Stroke::default()
                .with_width(0.6)
                .with_color(Color::from_rgba(
                    0.78,
                    0.94,
                    1.0,
                    0.18 * appear * (1.0 + ring_breathe * 0.2),
                )),
        );

        frame.with_save(|frame| {
            frame.translate(Vector::new(center.x, center.y));
            frame.scale(scale);

            for petal in 0..SPINNER_PETALS {
                let trail = petal as f32 / SPINNER_PETALS as f32;
                let serial = (phase - trail).rem_euclid(1.0);
                let intensity = (1.0 - serial).powf(3.0);
                let alpha = (0.2 + intensity * 0.64) * appear;
                let angle = std::f32::consts::TAU * trail;
                let width = lerp(2.6, 5.1, intensity);
                let length = lerp(4.8, 8.2, intensity);
                let outer_bound = SPINNER_RING_RADIUS - SPINNER_RING_GAP;
                let inner_bound = SPINNER_INNER_RING_RADIUS + 1.2;
                let offset = lerp(inner_bound + 1.2, outer_bound - 1.5, intensity);
                let shade = lerp(0.34, 1.0, intensity);
                let color = Color::from_rgba(
                    shade,
                    shade,
                    shade,
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

                    let shine = width * 0.44;
                    let shine_path = canvas::Path::rounded_rectangle(
                        Point::new(-shine / 2.0, -offset + length * 0.06),
                        Size::new(shine, length * 0.58),
                        (shine / 2.0).into(),
                    );
                    frame.fill(
                        &shine_path,
                        Color::from_rgba(0.98, 0.99, 1.0, alpha * 0.58 * intensity),
                    );

                });
            }
        });

        let core_glow = canvas::Path::circle(center, SPINNER_CORE_GLOW_RADIUS * scale);
        frame.fill(
            &core_glow,
            Color::from_rgba(0.77, 0.95, 1.0, 0.15 * appear * (1.0 + ring_breathe * 0.1)),
        );

        let core = canvas::Path::circle(center, SPINNER_CORE_RADIUS * scale);
        frame.fill(&core, Color::from_rgba(1.0, 1.0, 1.0, 0.66 * appear));

        let core_reflection = canvas::Path::circle(
            Point::new(center.x - 0.6 * scale, center.y - 0.7 * scale),
            SPINNER_CORE_RADIUS * 0.5 * scale,
        );
        frame.fill(
            &core_reflection,
            Color::from_rgba(0.98, 1.0, 1.0, 0.36 * appear),
        );

        vec![frame.into_geometry()]
    }
}
