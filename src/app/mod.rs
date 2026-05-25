mod view;

use std::{
    collections::VecDeque,
    env,
    fs::{File, OpenOptions},
    io::Read,
    os::fd::{AsRawFd, FromRawFd, RawFd},
    path::PathBuf,
    process,
    sync::mpsc,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::{Datelike, Local, Weekday};
use iced::widget::operation::focus;
use iced::{Event, Size, Subscription, Task, event, keyboard, mouse, time, window};
use iced_sessionlock::{
    Settings as SessionlockSettings, actions::UnLockAction, application as sessionlock_application,
};
use limes_lock::{
    AuthFailure as ProtoAuthFailure, AuthOutcome, AuthRequest, Config, EventBus, LockRuntime,
    LockState, NoopDisplayBackend, NoopLockBackend, StderrEventSink,
    common::EventSink,
    proto::{LimesEvent, PamMessageKind},
};

use crate::cli::LockOptions;

const IDLE_AFTER: Duration = Duration::from_secs(8);
const PASSWORD_INPUT_ID: &str = "password-input";
const SPINNER_FRAME: Duration = Duration::from_millis(16);
const RESTING_FRAME: Duration = Duration::from_millis(33);
const PREVIEW_AUTH_DELAY: Duration = Duration::from_millis(900);
const WALLPAPER_BYTES: &[u8] = include_bytes!("../../bg.jpg");

pub(crate) fn run_lock(options: LockOptions) -> Result<(), String> {
    let _single_instance = SingleInstanceLock::acquire()?;
    let mut daemon_ready = DaemonReady::prepare(options.daemonize)?;

    if options.daemonize
        && let Some(parent) = daemon_ready.fork()?
    {
        return parent.wait();
    }

    let ready = ReadyNotifier::new(options.ready_fd, daemon_ready.take_child_ready_fd());

    let pam_messages = PamMessageQueue::default();
    let runtime = Arc::new(
        LockRuntime::from_env()
            .map_err(|error| format!("cannot initialize limes lock runtime: {error}"))?,
    );
    runtime.events().subscribe(Arc::new(StderrEventSink));
    runtime
        .events()
        .subscribe(Arc::new(PamMessageSink::new(pam_messages.clone())));

    let (locked_sender, locked_receiver) = mpsc::channel();
    let settings = SessionlockSettings {
        locked_sender: Some(locked_sender),
        ..SessionlockSettings::default()
    };
    std::thread::spawn(move || {
        let _ = locked_receiver.recv();
        ready.notify();
    });

    sessionlock_application(
        move || FullScreenLock::new_lock(Arc::clone(&runtime), pam_messages.clone()),
        FullScreenLock::update,
        FullScreenLock::view,
    )
    .settings(settings)
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
    .title("Reimu on Starlit Water preview")
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
    pam_messages: PamMessageQueue,
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
    clock_date: String,
    clock_time: String,
    clock_minute: i64,
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

#[derive(Debug, Clone)]
struct PamStatusMessage {
    kind: PamMessageKind,
    message: String,
}

#[derive(Debug, Clone, Default)]
struct PamMessageQueue {
    messages: Arc<Mutex<VecDeque<PamStatusMessage>>>,
}

struct SingleInstanceLock {
    file: File,
}

impl SingleInstanceLock {
    fn acquire() -> Result<Self, String> {
        let path = lock_file_path()?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| format!("cannot open lock file {}: {error}", path.display()))?;
        let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if result == 0 {
            return Ok(Self { file });
        }

        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::EWOULDBLOCK)
            || error.raw_os_error() == Some(libc::EAGAIN)
        {
            eprintln!("reimu-on-starlit-water: lock screen is already running");
            process::exit(0);
        }

        Err(format!(
            "cannot acquire lock file {}: {error}",
            path.display()
        ))
    }
}

impl Drop for SingleInstanceLock {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

fn lock_file_path() -> Result<PathBuf, String> {
    let runtime_dir = env::var_os("XDG_RUNTIME_DIR")
        .ok_or_else(|| "XDG_RUNTIME_DIR is not set; cannot create lock guard".to_owned())?;
    Ok(PathBuf::from(runtime_dir).join("reimu-on-starlit-water.lock"))
}

struct ReadyNotifier {
    fds: Vec<RawFd>,
}

impl ReadyNotifier {
    fn new(ready_fd: Option<i32>, daemon_fd: Option<RawFd>) -> Self {
        let mut fds = Vec::new();
        if let Some(fd) = ready_fd {
            fds.push(fd);
        }
        if let Some(fd) = daemon_fd {
            fds.push(fd);
        }
        Self { fds }
    }

    fn notify(mut self) {
        for fd in self.fds.drain(..) {
            unsafe {
                libc::write(fd, b"\n".as_ptr().cast(), 1);
                libc::close(fd);
            }
        }
    }
}

impl Drop for ReadyNotifier {
    fn drop(&mut self) {
        for fd in self.fds.drain(..) {
            unsafe {
                libc::close(fd);
            }
        }
    }
}

struct DaemonReady {
    read_fd: Option<RawFd>,
    write_fd: Option<RawFd>,
}

struct DaemonParent {
    read_fd: RawFd,
    child_pid: libc::pid_t,
}

impl DaemonReady {
    fn prepare(daemonize: bool) -> Result<Self, String> {
        if !daemonize {
            return Ok(Self {
                read_fd: None,
                write_fd: None,
            });
        }

        let mut fds = [0; 2];
        let result = unsafe { libc::pipe(fds.as_mut_ptr()) };
        if result != 0 {
            return Err(format!(
                "cannot create daemon readiness pipe: {}",
                std::io::Error::last_os_error()
            ));
        }

        Ok(Self {
            read_fd: Some(fds[0]),
            write_fd: Some(fds[1]),
        })
    }

    fn take_child_ready_fd(&mut self) -> Option<RawFd> {
        self.write_fd.take()
    }

    fn fork(&mut self) -> Result<Option<DaemonParent>, String> {
        let Some(read_fd) = self.read_fd.take() else {
            return Ok(None);
        };
        let write_fd = self
            .write_fd
            .take()
            .expect("daemon mode should have write fd");

        let pid = unsafe { libc::fork() };
        if pid < 0 {
            unsafe {
                libc::close(read_fd);
                libc::close(write_fd);
            }
            return Err(format!(
                "cannot daemonize: {}",
                std::io::Error::last_os_error()
            ));
        }

        if pid > 0 {
            unsafe {
                libc::close(write_fd);
            }
            return Ok(Some(DaemonParent {
                read_fd,
                child_pid: pid,
            }));
        }

        unsafe {
            libc::close(read_fd);
            libc::setsid();
        }
        Ok(None)
    }
}

impl Drop for DaemonReady {
    fn drop(&mut self) {
        if let Some(fd) = self.read_fd.take() {
            unsafe {
                libc::close(fd);
            }
        }
        if let Some(fd) = self.write_fd.take() {
            unsafe {
                libc::close(fd);
            }
        }
    }
}

impl DaemonParent {
    fn wait(self) -> Result<(), String> {
        let mut file = unsafe { File::from_raw_fd(self.read_fd) };
        let mut byte = [0; 1];
        match file.read(&mut byte) {
            Ok(1) => Ok(()),
            Ok(_) => {
                let mut status = 0;
                unsafe {
                    libc::waitpid(self.child_pid, &mut status, 0);
                }
                Err("lock daemon exited before the compositor confirmed locking".to_owned())
            }
            Err(error) => Err(format!("cannot wait for lock daemon readiness: {error}")),
        }
    }
}

impl PamMessageQueue {
    fn push(&self, message: PamStatusMessage) {
        if let Ok(mut messages) = self.messages.lock() {
            messages.push_back(message);
        }
    }

    fn drain(&self) -> Vec<PamStatusMessage> {
        self.messages
            .lock()
            .map(|mut messages| messages.drain(..).collect())
            .unwrap_or_default()
    }

    fn clear(&self) {
        if let Ok(mut messages) = self.messages.lock() {
            messages.clear();
        }
    }
}

#[derive(Debug)]
struct PamMessageSink {
    messages: PamMessageQueue,
}

impl PamMessageSink {
    fn new(messages: PamMessageQueue) -> Self {
        Self { messages }
    }
}

impl EventSink for PamMessageSink {
    fn emit(&self, event: &LimesEvent) {
        let LimesEvent::AuthPamMessage { kind, message, .. } = event else {
            return;
        };

        let message = message.trim();
        if message.is_empty() {
            return;
        }

        self.messages.push(PamStatusMessage {
            kind: *kind,
            message: message.to_owned(),
        });
    }
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
    fn new_lock(runtime: Arc<LockRuntime>, pam_messages: PamMessageQueue) -> (Self, Task<Message>) {
        (
            Self::new(RunMode::Lock, Some(runtime), pam_messages),
            Task::none(),
        )
    }

    fn new_preview() -> (Self, Task<Message>) {
        (
            Self::new(
                RunMode::Preview,
                Some(Arc::new(noop_lock_runtime())),
                PamMessageQueue::default(),
            ),
            Task::none(),
        )
    }

    fn new(
        mode: RunMode,
        runtime: Option<Arc<LockRuntime>>,
        pam_messages: PamMessageQueue,
    ) -> Self {
        let now = Instant::now();
        let now_local = Local::now();
        let (clock_date, clock_time, clock_minute) = Self::format_clock(now_local);

        Self {
            mode,
            runtime,
            pam_messages,
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
            clock_date,
            clock_time,
            clock_minute,
        }
    }

    fn format_clock(now: chrono::DateTime<Local>) -> (String, String, i64) {
        (
            chinese_date(now.weekday(), now.month(), now.day()),
            now.format("%H:%M").to_string(),
            now.timestamp() / 60,
        )
    }

    fn update_clock_text(&mut self) {
        let now = Local::now();
        let current_minute = now.timestamp() / 60;

        if current_minute != self.clock_minute {
            let (date, time, minute) = Self::format_clock(now);
            self.clock_date = date;
            self.clock_time = time;
            self.clock_minute = minute;
        }
    }

    fn resting_frame(&self) -> Duration {
        RESTING_FRAME
    }

    fn subscription(&self) -> Subscription<Message> {
        let frame_time = if self.screen_state == ScreenState::Authenticating {
            SPINNER_FRAME
        } else {
            self.resting_frame()
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
                self.update_clock_text();
                self.drain_pam_messages();

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

    fn drain_pam_messages(&mut self) {
        if self.screen_state != ScreenState::Authenticating {
            self.pam_messages.clear();
            return;
        }

        for message in self.pam_messages.drain() {
            if should_show_pam_message(message.kind, &message.message) {
                self.status = message.message;
            }
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
        self.pam_messages.clear();

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
                self.status.clear();

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

fn should_show_pam_message(kind: PamMessageKind, message: &str) -> bool {
    if kind != PamMessageKind::PromptEchoOff {
        return true;
    }

    let message = message.trim().trim_end_matches(':').to_ascii_lowercase();
    !matches!(message.as_str(), "password" | "passphrase")
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

fn chinese_date(weekday: Weekday, month: u32, day: u32) -> String {
    let weekday = match weekday {
        Weekday::Mon => "週一",
        Weekday::Tue => "週二",
        Weekday::Wed => "週三",
        Weekday::Thu => "週四",
        Weekday::Fri => "週五",
        Weekday::Sat => "週六",
        Weekday::Sun => "週日",
    };

    format!("{month}月{day}日 {weekday}")
}
