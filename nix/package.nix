{
  lib,
  rustPlatform,
  pkg-config,
  makeWrapper,
  pam,
  libGL,
  libxkbcommon,
  vulkan-loader,
  wayland,
  libx11,
  libxcursor,
  libxi,
  libxrandr,
}:

let
  guiRuntimeLibs = [
    libGL
    libxkbcommon
    vulkan-loader
    wayland
    libx11
    libxcursor
    libxi
    libxrandr
  ];
in
rustPlatform.buildRustPackage {
  pname = "reimu-on-starlit-water";
  version = "0.1.0";

  src = lib.cleanSource ../.;
  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [
    makeWrapper
    pkg-config
  ];

  buildInputs = [ pam ] ++ guiRuntimeLibs;

  postFixup = ''
    wrapProgram $out/bin/reimu-on-starlit-water \
      --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath guiRuntimeLibs}
  '';

  meta = {
    description = "Animated Wayland session lock screen";
    homepage = "https://github.com/iceice666/reimu_on_starlit_water";
    license = with lib.licenses; [
      mit
      asl20
    ];
    mainProgram = "reimu-on-starlit-water";
    platforms = lib.platforms.linux;
  };
}
