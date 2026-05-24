# AGENTS.md

Guidance for AI coding agents working in this repository.

## Project overview

Reimu Lays on Water is a standalone Rust Wayland session lock screen built with
`iced`, `iced_sessionlock`, and `limes-lock`.

It has two runtime modes:

- `preview`: runs the UI in a normal window and never calls PAM.
- `lock`: uses Wayland `ext-session-lock-v1` surfaces and authenticates through
  the `limes` PAM service.

Treat real lock-mode behavior as security-sensitive. UI code may collect input
and trigger authentication, but should not duplicate or bypass `limes-lock`
authentication/session-lock logic.

## Environment

Prefer the Nix shell for development because it provides Rust, PAM, and GUI
runtime libraries:

```sh
nix develop
```

The project uses Rust 2024 and currently declares `rust-version = "1.94"` in
`Cargo.toml`.

## Common commands

Run from the repository root:

```sh
cargo fmt --all
cargo clippy --all-targets
cargo test
```

Preview the lock screen without locking the session:

```sh
cargo run -- preview
```

Run the real session lock only under a Wayland compositor that supports
`ext-session-lock-v1`, with `/etc/pam.d/limes` configured and a recovery TTY or
SSH session available:

```sh
cargo run -- lock
```

Build the Nix package when changing flake or dependency setup:

```sh
nix build
```

## Repository layout

- `Cargo.toml`: package metadata and Rust dependencies.
- `flake.nix`: Nix development shell and package definition.
- `README.md`: user-facing overview, runtime requirements, and customization
  notes.
- `src/main.rs`: application entry point and top-level timing/config constants.
- `src/cli.rs`: command-line mode parsing.
- `src/app/`: lock-screen application state and view construction.
- `src/effects/`: custom visual effects used by the UI.
- `src/style.rs`: shared visual styling.
- `src/math.rs`: small math helpers.
- `src/*.wgsl`: bundled shaders for water, rain, and clock effects.

## Coding guidelines

- Keep changes small and focused; preserve the current module boundaries unless a
  change clearly needs a new split.
- Do not log, persist, or expose passwords or other credential material.
- Keep `preview` mode free of PAM calls and real session-lock side effects.
- Keep `lock` mode using the real Wayland session-lock path; do not imply a
  preview/no-op path is secure.
- Keep image handles and shader resources stable across redraws where practical;
  this app is animation-heavy and unnecessary resource churn is visible.
- When editing WGSL, keep uniform layouts and Rust-side bytemuck structs in sync.
- Match the existing restrained liquid-glass UI style instead of adding unrelated
  decorative systems.
- Use conventional commit subjects such as `feat:`, `fix:`, `docs:`, or
  `chore:`.
- Do not commit build artifacts from `target/` or local environment files.

## Validation expectations

For documentation-only changes, no runtime validation is required.

For Rust or shader changes, run `cargo fmt --all` and at least
`cargo clippy --all-targets` when practical. Use `cargo run -- preview` for UI
smoke testing. Only use `cargo run -- lock` when the PAM and Wayland requirements
above are satisfied.
