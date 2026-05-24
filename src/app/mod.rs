mod view;

use std::{
    env,
    sync::Arc,
    time::{Duration, Instant},
};

use iced::widget::operation::focus;
use iced::{Event, Size, Subscription, Task, event, keyboard, mouse, time, window};
use iced_sessionlock::{actions::UnLockAction, application as sessionlock_application};
use limes_lock::{
    AuthFailure as ProtoAuthFailure, AuthOutcome, AuthRequest, Config, EventBus, LockRuntime,
    LockState, NoopDisplayBackend, NoopLockBackend, StderrEventSink,
};

const IDLE_AFTER: Duration = Duration::from_secs(8);
const PASSWORD_INPUT_ID: &str = "password-input";
const SPINNER_FRAME: Duration = Duration::from_millis(16);
const RESTING_FRAME: Duration = Duration::from_millis(16);
const PREVIEW_AUTH_DELAY: Duration = Duration::from_millis(900);
const WALLPAPER_BYTES: &[u8] = include_bytes!("../../bg.jpg");

pub(crate) fn run_lock() -> Result<(), String> {
    let runtime = Arc::new(
        LockRuntime::from_env()
            .map_err(|error| format!("cannot initialize limes lock runtime: {error}"))?,
    );
    runtime.events().subscribe(Arc::new(StderrEventSink));

    sessionlock_application(
        move || FullScreenLock::new_lock(Arc::clone(&runtime)),
        FullScreenLock::update,
        FullScreenLock::view,
    )
    .subscription(FullScreenLock::subscription)
    .run()
    .map_err(|error| error.to_string())
}

pub(crate) fn run_preview() -> Result<(), String> {
    iced::application(
        FullScreenLock::new_preview,
        FullScreenLock::update,
        FullScreenLock::preview_view,
    )
    .title("limes full screenlock preview")
    .window(window::Settings {
        size: Size::new(1280.0, 720.0),
        min_size: Some(Size::new(640.0, 360.0)),
        ..window::Settings::default()
    })
    .subscription(FullScreenLock::subscription)
    .run()
    .map_err(|error| error.to_string())
}

struct FullScreenLock {
    mode: RunMode,
    runtime: Option<Arc<LockRuntime>>,
    preview_window: window::Id,
    wallpaper: iced::widget::image::Handle,
    rain_started: Instant,
    username: String,
    password: String,
    lock_state: LockState,
    screen_state: ScreenState,
    last_input: Instant,
    auth_started: Option<Instant>,
    failure_shade: bool,
    status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunMode {
    Lock,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScreenState {
    Idle,
    Typing,
    Authenticating,
}

#[derive(Debug, Clone)]
enum Message {
    PasswordChanged(String),
    Submit,
    AuthFinished(AuthOutcome),
    PreviewAuthFinished,
    UnlockSession,
    WindowCloseRequested,
    WindowClosed,
    Tick(Instant),
    IcedEvent(Event),
}

impl TryFrom<Message> for UnLockAction {
    type Error = Message;

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        match value {
            Message::UnlockSession => Ok(UnLockAction),
            other => Err(other),
        }
    }
}

impl FullScreenLock {
    fn new_lock(runtime: Arc<LockRuntime>) -> (Self, Task<Message>) {
        (Self::new(RunMode::Lock, Some(runtime)), Task::none())
    }

    fn new_preview() -> (Self, Task<Message>) {
        (
            Self::new(RunMode::Preview, Some(Arc::new(noop_lock_runtime()))),
            Task::none(),
        )
    }

    fn new(mode: RunMode, runtime: Option<Arc<LockRuntime>>) -> Self {
        let now = Instant::now();

        Self {
            mode,
            runtime,
            preview_window: window::Id::unique(),
            wallpaper: iced::widget::image::Handle::from_bytes(WALLPAPER_BYTES),
            rain_started: now,
            username: env::var("USER").unwrap_or_default(),
            password: String::new(),
            lock_state: LockState::Locked,
            screen_state: ScreenState::Idle,
            last_input: now,
            auth_started: None,
            failure_shade: false,
            status: if mode == RunMode::Preview {
                "Preview mode: PAM is not called.".to_owned()
            } else {
                String::new()
            },
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let frame_time = if self.screen_state == ScreenState::Authenticating {
            SPINNER_FRAME
        } else {
            RESTING_FRAME
        };

        let mut subscriptions = vec![
            event::listen().map(Message::IcedEvent),
            time::every(frame_time).map(Message::Tick),
        ];

        if self.mode == RunMode::Preview {
            subscriptions.push(window::close_requests().map(|_| Message::WindowCloseRequested));
            subscriptions.push(window::close_events().map(|_| Message::WindowClosed));
        }

        Subscription::batch(subscriptions)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PasswordChanged(password) => {
                self.password = password;
                self.failure_shade = false;
                self.status.clear();
                self.activate_typing()
            }
            Message::Submit => self.submit(),
            Message::AuthFinished(outcome) => self.finish_auth(outcome),
            Message::PreviewAuthFinished => self.finish_preview_auth(),
            Message::UnlockSession => Task::none(),
            Message::WindowCloseRequested => iced::exit(),
            Message::WindowClosed => iced::exit(),
            Message::Tick(now) => {
                if self.screen_state == ScreenState::Typing
                    && now.duration_since(self.last_input) > IDLE_AFTER
                {
                    self.password.clear();
                    self.failure_shade = false;
                    self.status.clear();
                    self.screen_state = ScreenState::Idle;
                }
                Task::none()
            }
            Message::IcedEvent(event) => match event {
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key,
                    text: typed_text,
                    ..
                }) => match key {
                    keyboard::Key::Named(keyboard::key::Named::Enter) => self.submit(),
                    keyboard::Key::Named(keyboard::key::Named::Escape) => {
                        self.password.clear();
                        self.failure_shade = false;
                        self.status.clear();
                        self.screen_state = ScreenState::Idle;
                        Task::none()
                    }
                    _ => {
                        if self.screen_state == ScreenState::Idle
                            && let Some(typed_text) = typed_text
                        {
                            let typed_text: &str = typed_text.as_ref();

                            if !typed_text.is_empty()
                                && typed_text.chars().all(|character| !character.is_control())
                            {
                                self.password.push_str(typed_text);
                                self.failure_shade = false;
                                self.status.clear();
                            }
                        }

                        self.activate_typing()
                    }
                },
                Event::Mouse(mouse::Event::ButtonPressed(_)) => self.activate_typing(),
                _ => Task::none(),
            },
        }
    }

    fn activate_typing(&mut self) -> Task<Message> {
        if self.screen_state != ScreenState::Authenticating {
            self.screen_state = ScreenState::Typing;
            self.last_input = Instant::now();
            focus(PASSWORD_INPUT_ID)
        } else {
            Task::none()
        }
    }

    fn submit(&mut self) -> Task<Message> {
        if self.screen_state == ScreenState::Authenticating || self.lock_state != LockState::Locked
        {
            return Task::none();
        }

        // Allow an empty secret to reach PAM so passwordless modules (for
        // example fingerprint auth) can start from an Enter-only submit.
        let mut password = std::mem::take(&mut self.password);

        self.failure_shade = false;
        self.screen_state = ScreenState::Authenticating;
        self.lock_state = LockState::Unlocking;
        self.auth_started = Some(Instant::now());

        match self.mode {
            RunMode::Lock => {
                let Some(runtime) = self.runtime.as_ref().map(Arc::clone) else {
                    self.lock_state = LockState::Locked;
                    self.screen_state = ScreenState::Typing;
                    self.auth_started = None;
                    self.failure_shade = true;
                    self.status = "Lock runtime unavailable.".to_owned();
                    return focus(PASSWORD_INPUT_ID);
                };
                let username = self.username.clone();
                self.status = "Verifying with PAM…".to_owned();

                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let mut request = AuthRequest::new(username, password);
                            let outcome = runtime.authenticate_unlock(&request);
                            request.clear_secret();
                            outcome
                        })
                        .await
                        .unwrap_or_else(|error| {
                            Err(ProtoAuthFailure::Internal(format!(
                                "authentication task failed: {error}"
                            )))
                        })
                    },
                    Message::AuthFinished,
                )
            }
            RunMode::Preview => {
                password.clear();
                self.status = "Preview mode: simulating authentication…".to_owned();

                Task::perform(
                    async {
                        tokio::time::sleep(PREVIEW_AUTH_DELAY).await;
                    },
                    |_| Message::PreviewAuthFinished,
                )
            }
        }
    }

    fn finish_auth(&mut self, outcome: AuthOutcome) -> Task<Message> {
        match outcome {
            Ok(_) => {
                self.lock_state = LockState::Unlocked;
                self.auth_started = None;
                self.status = "Unlocked.".to_owned();
                Task::done(Message::UnlockSession)
            }
            Err(error) => {
                self.lock_state = LockState::Locked;
                self.screen_state = ScreenState::Typing;
                self.auth_started = None;
                self.failure_shade = true;
                self.status = auth_error_message(&error);
                self.last_input = Instant::now();
                focus(PASSWORD_INPUT_ID)
            }
        }
    }

    fn finish_preview_auth(&mut self) -> Task<Message> {
        self.lock_state = LockState::Locked;
        self.screen_state = ScreenState::Typing;
        self.auth_started = None;
        self.failure_shade = false;
        self.status = "Preview mode: authentication skipped.".to_owned();
        self.last_input = Instant::now();
        focus(PASSWORD_INPUT_ID)
    }
}

fn noop_lock_runtime() -> LockRuntime {
    LockRuntime::with_parts(
        Config {
            login_frontend: None,
            lock_frontend: None,
            session_command: Vec::new(),
            max_auth_attempts: 1,
        },
        Arc::new(NoopLockBackend),
        Arc::new(NoopDisplayBackend),
        EventBus::new(),
    )
}

fn auth_error_message(error: &ProtoAuthFailure) -> String {
    match error {
        ProtoAuthFailure::InvalidCredentials => "Invalid password. Try again.".to_owned(),
        ProtoAuthFailure::LockedOut => "Account is locked out.".to_owned(),
        ProtoAuthFailure::BackendUnavailable(reason) => {
            format!("PAM backend unavailable: {reason}")
        }
        ProtoAuthFailure::Internal(reason) => format!("Authentication error: {reason}"),
    }
}
