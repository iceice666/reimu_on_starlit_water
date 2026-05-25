#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliConfig {
    pub mode: CliMode,
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
            "lock" => Self::parse_command(CliMode::Lock, &command, args),
            "preview" => Self::parse_command(CliMode::Preview, &command, args),
            "help" | "--help" | "-h" => {
                print_help();
                Ok(None)
            }
            other => Err(format!(
                "unknown command `{other}`; try `reimu-lays-on-water --help`"
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

        Ok(Some(CliConfig { mode }))
    }
}

fn print_help() {
    println!(
        "Reimu Lays on Water\n\n\
Usage:\n  reimu-lays-on-water lock\n  reimu-lays-on-water preview\n\n\
Commands:\n  lock     Lock the session using Wayland ext-session-lock-v1 surfaces\n  preview  Show the lock UI in a normal window without locking or PAM\n\n\
Options:\n  -h, --help    Show help"
    );
}

fn print_command_help(mode: CliMode) {
    match mode {
        CliMode::Lock => println!(
            "Usage: reimu-lays-on-water lock\n\n\
Runs the full-screen session lock frontend on Wayland ext-session-lock-v1\n\
surfaces and authenticates unlock attempts with limes-lock/PAM."
        ),
        CliMode::Preview => println!(
            "Usage: reimu-lays-on-water preview\n\n\
Runs the same lock UI in a normal resizable window. It never locks the\n\
session and Enter only plays the authentication animation."
        ),
    }
}
