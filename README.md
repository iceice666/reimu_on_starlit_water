# Reimu Lays on Water

A standalone animated Wayland session lock screen built with `iced`, `iced_sessionlock`, and `limes-lock`.

The project displays the bundled `bg.jpg` wallpaper, keeps the image handle stable during redraws, and renders a custom WGSL top-down rain/water shader over it. The UI uses a minimal liquid-glass style with an idle clock, a password prompt, and an animated authentication spinner.

## Features

- Wayland `ext-session-lock-v1` lock surfaces for real session locking.
- `preview` mode for developing the UI in a normal window without locking the session or calling PAM.
- Bundled wallpaper with animated rain impacts, circular ripples, ambient shimmer, long-tailed raindrops, and expanding impact rings.
- Idle clock that switches to a bottom-centered password prompt on keyboard or mouse input.
- Enter submits to PAM, including empty submissions for passwordless modules such as fingerprint authentication.
- Success unlocks the compositor session; failure returns to the prompt with an error tint.
- Escape clears the prompt and returns to idle; inactivity returns to idle automatically.

## Requirements

- Linux with a Wayland compositor that supports `ext-session-lock-v1`.
- A Rust toolchain capable of building the project.
- PAM runtime/development files required by `limes-lock`.
- A PAM service named `limes` at `/etc/pam.d/limes` for real lock mode.

Before testing `lock`, make sure PAM authentication works and keep another TTY or SSH session available in case your PAM configuration is wrong.

## Run

Preview the lock screen in a normal resizable window:

```sh
cargo run -- preview
```

Run the real session lock:

```sh
cargo run -- lock
```

The shader runs at the high quality setting by default.

Build a release binary:

```sh
cargo build --release
./target/release/reimu-lays-on-water lock
```

The current Cargo package/binary name is `reimu-lays-on-water`.

## Nix

Preview directly from the flake:

```sh
nix run github:iceice666/reimu_lays_on_water -- preview
```

You can also use `nix run github:iceice666/reimu_lays_on_water#preview`.

Install into a Nix profile:

```sh
nix profile install github:iceice666/reimu_lays_on_water
```

Use the NixOS module to install the binary and create the `limes` PAM service:

```nix
{
  inputs.reimu-lays-on-water.url = "github:iceice666/reimu_lays_on_water";

  outputs = { nixpkgs, reimu-lays-on-water, ... }: {
    nixosConfigurations.host = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        reimu-lays-on-water.nixosModules.default
        {
          programs.reimu-lays-on-water.enable = true;
        }
      ];
    };
  };
}
```

If you only want the package in `pkgs`, add the overlay:

```nix
{
  nixpkgs.overlays = [
    inputs.reimu-lays-on-water.overlays.default
  ];
}
```

## PAM

Lock mode authenticates the current `$USER` through `limes-lock`, which uses the PAM service name `limes`. Configure `/etc/pam.d/limes` using the policy appropriate for your distribution, for example by including the same authentication stack used by your login or screen-locking tools.

`preview` mode never calls PAM; pressing Enter only plays the authentication animation.

## Background

The bundled `bg.jpg` source is Pixiv artwork 34844544: <https://www.pixiv.net/artworks/34844544>.

## Customization

- Replace `bg.jpg` to change the bundled wallpaper.
- Edit `src/rain_ripples.wgsl` for water ripples and `src/rain_drops.wgsl` for rain drops.
- Tune constants near the top of `src/main.rs` for timing, intensity, input sizing, and animation speed.
