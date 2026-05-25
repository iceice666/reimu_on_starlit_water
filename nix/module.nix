{ self }:

{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.programs.reimu-on-starlit-water;
  defaultPackage = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
in
{
  options.programs.reimu-on-starlit-water = {
    enable = lib.mkEnableOption "Reimu on Starlit Water";

    package = lib.mkOption {
      type = lib.types.package;
      default = defaultPackage;
      defaultText = lib.literalExpression "reimu-on-starlit-water.packages.\${pkgs.stdenv.hostPlatform.system}.default";
      description = "Package to install for Reimu on Starlit Water.";
    };

    configurePam = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Whether to create the limes PAM service used by lock mode.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];

    security.pam.services.limes = lib.mkIf cfg.configurePam { };
  };
}
