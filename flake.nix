{
  description = "Reimu Lays on Water Wayland session lock screen";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        guiRuntimeLibs = with pkgs; [
          libGL
          libxkbcommon
          vulkan-loader
          wayland
          libx11
          libxcursor
          libxi
          libxrandr
        ];

        reimuLaysOnWater = pkgs.rustPlatform.buildRustPackage {
          pname = "limes-full-screenlock";
          version = "0.1.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.pam ] ++ guiRuntimeLibs;
        };
      in
      {
        packages = {
          default = reimuLaysOnWater;
          limes-full-screenlock = reimuLaysOnWater;
        };

        apps.default = {
          type = "app";
          program = "${reimuLaysOnWater}/bin/limes-full-screenlock";
          meta.description = "Run the Reimu Lays on Water lock screen";
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            clippy
            pam
            pkg-config
            rust-analyzer
            rustc
            rustfmt
          ] ++ guiRuntimeLibs;

          env = {
            RUST_BACKTRACE = "1";
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath guiRuntimeLibs;
          };
        };
      }
    );
}
