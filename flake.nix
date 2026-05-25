{
  description = "Reimu on Starlit Water Wayland session lock screen";

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

        reimuOnStarlitWater = pkgs.callPackage ./nix/package.nix { };

        previewApp = pkgs.writeShellApplication {
          name = "reimu-on-starlit-water-preview";
          text = ''
            exec ${pkgs.lib.getExe reimuOnStarlitWater} preview "$@"
          '';
        };

        lockApp = pkgs.writeShellApplication {
          name = "reimu-on-starlit-water-lock";
          text = ''
            exec ${pkgs.lib.getExe reimuOnStarlitWater} lock "$@"
          '';
        };
      in
      {
        packages = {
          default = reimuOnStarlitWater;
          reimu-on-starlit-water = reimuOnStarlitWater;
        };

        apps.default = {
          type = "app";
          program = "${reimuOnStarlitWater}/bin/reimu-on-starlit-water";
          meta.description = "Run the Reimu on Starlit Water lock screen";
        };

        apps.preview = {
          type = "app";
          program = "${pkgs.lib.getExe previewApp}";
          meta.description = "Preview the Reimu on Starlit Water lock screen";
        };

        apps.lock = {
          type = "app";
          program = "${pkgs.lib.getExe lockApp}";
          meta.description = "Lock the session with Reimu on Starlit Water";
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
        reimu-on-starlit-water = final.callPackage ./nix/package.nix { };
      };

      nixosModules.default = import ./nix/module.nix { inherit self; };
    };
}
