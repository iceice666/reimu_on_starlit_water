use std::time::{Duration, Instant};

use chrono::Local;
use iced::widget::{Canvas, Shader, Space, column, container, image, stack, text, text_input};
use iced::{Alignment, Color, ContentFit, Element, Length, window};

use crate::effects::{FlowerSpinner, RainDrops};
use crate::math::{ease_out_cubic, lerp};
use crate::style::{clock_glass, input_shell, password_input_style};

use super::{FullScreenLock, Message, PASSWORD_INPUT_ID, ScreenState};

const SLOW_AUTH_AFTER: Duration = Duration::from_secs(3);
const RAIN_INTENSITY: f32 = 0.90;
const INPUT_WIDTH: f32 = 420.0;
const INPUT_HEIGHT: f32 = 64.0;
const CIRCLE_TRANSITION: Duration = Duration::from_millis(260);

impl FullScreenLock {
    pub(super) fn preview_view(&self) -> Element<'_, Message> {
        self.view(self.preview_window)
    }

    pub(super) fn view(&self, window: window::Id) -> Element<'_, Message> {
        // Keep a stable image handle. Recreating `Handle::from_bytes` in every
        // view pass gives iced a fresh cache id each frame and can leave the
        // wallpaper blank while the lock screen is redrawing.
        let background = image(self.wallpaper.clone())
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(ContentFit::Cover);

        let rain = Shader::new(RainDrops::new(
            window,
            self.rain_started.elapsed().as_secs_f32(),
            RAIN_INTENSITY,
        ))
        .width(Length::Fill)
        .height(Length::Fill);

        let content = match self.screen_state {
            ScreenState::Idle => self.idle_view(),
            ScreenState::Typing => self.typing_view(false),
            ScreenState::Authenticating => self.typing_view(true),
        };

        stack![background, rain, content].into()
    }

    fn idle_view(&self) -> Element<'_, Message> {
        let now = Local::now();
        let clock = column![
            text(now.format("%H:%M").to_string())
                .size(112)
                .color(Color::WHITE),
            text(now.format("%A, %B %-d").to_string())
                .size(25)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.86)),
        ]
        .align_x(Alignment::Center)
        .spacing(2);

        let glass_clock = container(clock)
            .padding([24, 48])
            .width(Length::Shrink)
            .style(|_| clock_glass());

        container(column![
            Space::new().height(Length::Fixed(56.0)),
            glass_clock
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .into()
    }

    fn typing_view(&self, loading: bool) -> Element<'_, Message> {
        let field: Element<'_, Message> = if loading {
            let elapsed = self
                .auth_started
                .map(|started| started.elapsed())
                .unwrap_or(Duration::ZERO);
            let morph = loading_morph(self.auth_started);
            let width = lerp(INPUT_WIDTH, INPUT_HEIGHT, morph);

            container(
                Canvas::new(FlowerSpinner::new(elapsed))
                    .width(Length::Fixed(44.0))
                    .height(Length::Fixed(44.0)),
            )
            .width(Length::Fixed(width))
            .height(Length::Fixed(INPUT_HEIGHT))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(|_| input_shell(false))
            .into()
        } else {
            let failed = self.failure_shade;
            let placeholder = if !self.status.is_empty() {
                self.status.as_str()
            } else {
                "Password"
            };

            let input = text_input(placeholder, &self.password)
                .id(PASSWORD_INPUT_ID)
                .on_input(Message::PasswordChanged)
                .on_submit(Message::Submit)
                .secure(true)
                .padding([14, 22])
                .size(22)
                .width(Length::Fill)
                .style(move |_, status| password_input_style(status, failed));

            container(input)
                .width(Length::Fixed(INPUT_WIDTH))
                .height(Length::Fixed(INPUT_HEIGHT))
                .align_y(Alignment::Center)
                .style(move |_| input_shell(failed))
                .into()
        };

        let content = if loading {
            let pam_status = container(
                text(auth_status_message(self.auth_started, &self.status))
                    .size(15)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.74)),
            )
            .padding([8, 18])
            .width(Length::Shrink)
            .style(|_| input_shell(false));

            column![
                Space::new().height(Length::Fill),
                field,
                pam_status,
                Space::new().height(Length::Fixed(54.0))
            ]
            .align_x(Alignment::Center)
            .spacing(12)
        } else {
            column![
                Space::new().height(Length::Fill),
                field,
                Space::new().height(Length::Fixed(84.0))
            ]
            .align_x(Alignment::Center)
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .into()
    }
}

fn auth_status_message<'a>(auth_started: Option<Instant>, status: &'a str) -> &'a str {
    if auth_started.is_some_and(|started| started.elapsed() > SLOW_AUTH_AFTER) {
        "Still verifying…"
    } else if status.is_empty() {
        "Verifying…"
    } else {
        status
    }
}

fn loading_morph(auth_started: Option<Instant>) -> f32 {
    auth_started
        .map(|started| {
            let progress =
                (started.elapsed().as_secs_f32() / CIRCLE_TRANSITION.as_secs_f32()).clamp(0.0, 1.0);
            ease_out_cubic(progress)
        })
        .unwrap_or(1.0)
}
