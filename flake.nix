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
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
    in
    flake-utils.lib.eachSystem supportedSystems (
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

        reimuLaysOnWater = pkgs.callPackage ./nix/package.nix { };

        previewApp = pkgs.writeShellApplication {
          name = "reimu-lays-on-water-preview";
          text = ''
            exec ${pkgs.lib.getExe reimuLaysOnWater} preview "$@"
          '';
        };

        lockApp = pkgs.writeShellApplication {
          name = "reimu-lays-on-water-lock";
          text = ''
            exec ${pkgs.lib.getExe reimuLaysOnWater} lock "$@"
          '';
        };
      in
      {
        packages = {
          default = reimuLaysOnWater;
          reimu-lays-on-water = reimuLaysOnWater;
        };

        apps.default = {
          type = "app";
          program = "${reimuLaysOnWater}/bin/reimu-lays-on-water";
          meta.description = "Run the Reimu Lays on Water lock screen";
        };

        apps.preview = {
          type = "app";
          program = "${pkgs.lib.getExe previewApp}";
          meta.description = "Preview the Reimu Lays on Water lock screen";
        };

        apps.lock = {
          type = "app";
          program = "${pkgs.lib.getExe lockApp}";
          meta.description = "Lock the session with Reimu Lays on Water";
        };

        devShells.default = pkgs.mkShell {
          packages =
            with pkgs;
            [
              cargo
              clippy
              pam
              pkg-config
              rust-analyzer
              rustc
              rustfmt
            ]
            ++ guiRuntimeLibs;

          env = {
            RUST_BACKTRACE = "1";
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath guiRuntimeLibs;
          };
        };
      }
    )
    // {
      overlays.default = final: _prev: {
        reimu-lays-on-water = final.callPackage ./nix/package.nix { };
      };

      nixosModules.default = import ./nix/module.nix { inherit self; };
    };
}
