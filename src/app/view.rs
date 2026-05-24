use std::time::{Duration, Instant};

use iced::widget::{
    Canvas, Shader, Space, column, container, image, stack, text, text::Wrapping, text_input,
};
use iced::{Alignment, Color, ContentFit, Element, Length, window};

use crate::effects::{FlowerSpinner, MacosGlassClock, RainDrops};
use crate::math::{ease_out_cubic, lerp};
use crate::style::{input_shell, password_input_style};

use super::{FullScreenLock, Message, PASSWORD_INPUT_ID, ScreenState};

const SLOW_AUTH_AFTER: Duration = Duration::from_secs(3);
const RAIN_INTENSITY: f32 = 0.90;
const INPUT_WIDTH: f32 = 200.0;
const INPUT_HEIGHT: f32 = 45.0;
const PASSWORD_TEXT_SIZE: f32 = 20.0;
const STATUS_TEXT_SIZE: f32 = 16.0;
const STATUS_MAX_WIDTH: f32 = 480.0;
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
            ScreenState::Idle => self.idle_view(window),
            ScreenState::Typing => self.typing_view(false),
            ScreenState::Authenticating => self.typing_view(true),
        };

        stack![background, rain, content].into()
    }

    fn idle_view(&self, window: window::Id) -> Element<'_, Message> {
        container(
            Shader::new(MacosGlassClock::new(
                window,
                self.clock_date.clone(),
                self.clock_time.clone(),
            ))
            .width(Length::Fill)
            .height(Length::Fill),
        )
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

            let input = text_input("", &self.password)
                .id(PASSWORD_INPUT_ID)
                .on_input(Message::PasswordChanged)
                .on_submit(Message::Submit)
                .secure(true)
                .padding([6, 18])
                .size(PASSWORD_TEXT_SIZE)
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
            let pam_status =
                status_badge(auth_status_message(self.auth_started, &self.status), false);

            column![
                Space::new().height(Length::Fill),
                field,
                pam_status,
                Space::new().height(Length::Fixed(54.0))
            ]
            .align_x(Alignment::Center)
            .spacing(12)
        } else if self.status.is_empty() {
            column![
                Space::new().height(Length::Fill),
                field,
                Space::new().height(Length::Fixed(84.0))
            ]
            .align_x(Alignment::Center)
        } else {
            column![
                Space::new().height(Length::Fill),
                field,
                status_badge(&self.status, self.failure_shade),
                Space::new().height(Length::Fixed(54.0))
            ]
            .align_x(Alignment::Center)
            .spacing(12)
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .into()
    }
}

fn auth_status_message(auth_started: Option<Instant>, status: &str) -> &str {
    if auth_started.is_some_and(|started| started.elapsed() > SLOW_AUTH_AFTER) {
        "Still verifying…"
    } else if status.is_empty() {
        "Verifying…"
    } else {
        status
    }
}

fn status_badge(status: &str, failed: bool) -> Element<'_, Message> {
    container(
        text(status)
            .size(STATUS_TEXT_SIZE)
            .color(status_text_color(failed))
            .align_x(Alignment::Center)
            .wrapping(Wrapping::WordOrGlyph),
    )
    .padding([6, 16])
    .width(Length::Shrink)
    .max_width(STATUS_MAX_WIDTH)
    .style(move |_| input_shell(failed))
    .into()
}

fn status_text_color(failed: bool) -> Color {
    if failed {
        Color::from_rgba(1.0, 0.84, 0.86, 0.94)
    } else {
        Color::from_rgba(1.0, 1.0, 1.0, 0.74)
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
