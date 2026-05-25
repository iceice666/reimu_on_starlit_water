#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliConfig {
    pub mode: CliMode,
    pub lock: LockOptions,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct LockOptions {
    pub daemonize: bool,
    pub ready_fd: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliMode {
    Lock,
    Preview,
}

impl CliMode {
    pub(crate) fn from_args(
        mut args: impl Iterator<Item = String>,
    ) -> Result<Option<CliConfig>, String> {
        let Some(command) = args.next() else {
            print_help();
            return Ok(None);
        };

        match command.as_str() {
            "lock" => Self::parse_lock_command(&command, args),
            "preview" => Self::parse_command(CliMode::Preview, &command, args),
            "help" | "--help" | "-h" => {
                print_help();
                Ok(None)
            }
            other => Err(format!(
                "unknown command `{other}`; try `reimu-on-starlit-water --help`"
            )),
        }
    }

    fn parse_command(
        mode: CliMode,
        command: &str,
        args: impl Iterator<Item = String>,
    ) -> Result<Option<CliConfig>, String> {
        if let Some(arg) = args.into_iter().next() {
            if arg == "--help" || arg == "-h" {
                print_command_help(mode);
                return Ok(None);
            }

            return Err(format!(
                "unexpected argument `{arg}` for `{command}`; use `{command} --help`"
            ));
        }

        Ok(Some(CliConfig {
            mode,
            lock: LockOptions::default(),
        }))
    }

    fn parse_lock_command(
        command: &str,
        args: impl Iterator<Item = String>,
    ) -> Result<Option<CliConfig>, String> {
        let mut lock = LockOptions::default();
        let mut args = args.peekable();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => {
                    print_command_help(CliMode::Lock);
                    return Ok(None);
                }
                "--daemonize" | "-f" => {
                    lock.daemonize = true;
                }
                "--ready-fd" => {
                    let Some(fd) = args.next() else {
                        return Err("missing file descriptor after `--ready-fd`".to_owned());
                    };
                    lock.ready_fd = Some(parse_ready_fd(&fd)?);
                }
                _ if arg.starts_with("--ready-fd=") => {
                    let fd = arg
                        .split_once('=')
                        .map(|(_, fd)| fd)
                        .expect("prefix check guarantees split");
                    lock.ready_fd = Some(parse_ready_fd(fd)?);
                }
                _ => {
                    return Err(format!(
                        "unexpected argument `{arg}` for `{command}`; use `{command} --help`"
                    ));
                }
            }
        }

        Ok(Some(CliConfig {
            mode: CliMode::Lock,
            lock,
        }))
    }
}

fn parse_ready_fd(value: &str) -> Result<i32, String> {
    let fd = value
        .parse::<i32>()
        .map_err(|_| format!("invalid ready file descriptor `{value}`"))?;
    if fd < 0 {
        return Err("ready file descriptor must be non-negative".to_owned());
    }
    Ok(fd)
}

fn print_help() {
    println!(
        "Reimu on Starlit Water\n\n\
Usage:\n  reimu-on-starlit-water lock [OPTIONS]\n  reimu-on-starlit-water preview\n\n\
Commands:\n  lock     Lock the session using Wayland ext-session-lock-v1 surfaces\n  preview  Show the lock UI in a normal window without locking or PAM\n\n\
Options:\n  -h, --help    Show help"
    );
}

fn print_command_help(mode: CliMode) {
    match mode {
        CliMode::Lock => println!(
            "Usage: reimu-on-starlit-water lock [OPTIONS]\n\n\
Runs the full-screen session lock frontend on Wayland ext-session-lock-v1\n\
surfaces and authenticates unlock attempts with limes-lock/PAM.\n\n\
Options:\n  -f, --daemonize      Fork into the background after the compositor confirms locking\n\
      --ready-fd <fd>  Write a newline to fd after the compositor confirms locking"
        ),
        CliMode::Preview => println!(
            "Usage: reimu-on-starlit-water preview\n\n\
Runs the same lock UI in a normal resizable window. It never locks the\n\
session and Enter only plays the authentication animation."
        ),
    }
}
