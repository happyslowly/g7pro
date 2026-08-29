{ pkgs ? import <nixpkgs> { } }:

pkgs.rustPlatform.buildRustPackage {
  pname = "g7pro";
  version = "0.1.0";

  src = pkgs.lib.cleanSource ./.;

  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  buildInputs = with pkgs; [
    systemd
  ];

  meta = {
    description = "CLI tool for the GameSir G7 Pro 8K controller (battery, rumble, button test)";
    mainProgram = "g7pro";
    platforms = pkgs.lib.platforms.linux;
  };
}
