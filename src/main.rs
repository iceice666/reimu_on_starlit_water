mod app;
mod cli;
mod effects;
mod math;
mod style;

use std::{env, process};

use cli::CliMode;

fn main() {
    if let Err(error) = run() {
        eprintln!("limes-full-screenlock: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    match CliMode::from_args(env::args().skip(1))? {
        Some(config) => match config.mode {
            CliMode::Lock => app::run_lock(),
            CliMode::Preview => app::run_preview(),
        },
        None => Ok(()),
    }
}
