#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliMode {
    Lock,
    Preview,
}

impl CliMode {
    pub(crate) fn from_args(
        mut args: impl Iterator<Item = String>,
    ) -> Result<Option<Self>, String> {
        let Some(command) = args.next() else {
            print_help();
            return Ok(None);
        };

        match command.as_str() {
            "lock" => Self::parse_command(Self::Lock, &command, args),
            "preview" => Self::parse_command(Self::Preview, &command, args),
            "help" | "--help" | "-h" => {
                print_help();
                Ok(None)
            }
            other => Err(format!(
                "unknown command `{other}`; try `limes-full-screenlock --help`"
            )),
        }
    }

    fn parse_command(
        mode: Self,
        command: &str,
        args: impl Iterator<Item = String>,
    ) -> Result<Option<Self>, String> {
        let extra = args.collect::<Vec<_>>();
        if extra
            .iter()
            .any(|arg| matches!(arg.as_str(), "--help" | "-h"))
        {
            print_command_help(mode);
            return Ok(None);
        }
        if !extra.is_empty() {
            return Err(format!(
                "unexpected arguments for `{command}`: {}",
                extra.join(" ")
            ));
        }

        Ok(Some(mode))
    }
}

fn print_help() {
    println!(
        "limes full screenlock\n\n\
Usage:\n  limes-full-screenlock lock\n  limes-full-screenlock preview\n\n\
Commands:\n  lock     Lock the session using Wayland ext-session-lock-v1 surfaces\n  preview  Show the lock UI in a normal window without locking or PAM"
    );
}

fn print_command_help(mode: CliMode) {
    match mode {
        CliMode::Lock => println!(
            "Usage: limes-full-screenlock lock\n\n\
Runs the full-screen session lock frontend on Wayland ext-session-lock-v1\n\
surfaces and authenticates unlock attempts with limes-lock/PAM."
        ),
        CliMode::Preview => println!(
            "Usage: limes-full-screenlock preview\n\n\
Runs the same lock UI in a normal resizable window. It never locks the\n\
session and Enter only plays the authentication animation."
        ),
    }
}
