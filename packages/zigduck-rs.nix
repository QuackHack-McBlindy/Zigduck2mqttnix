{
  self,
  lib,
  pkgs,
  rustPlatform,
  fetchFromGitHub,
  ...
} : let
  src = ./zigduck-rs;
  cargoToml = builtins.fromTOML (builtins.readFile (src + "/Cargo.toml"));
  version = cargoToml.package.version;
  desc = cargoToml.package.description;
in  
rustPlatform.buildRustPackage {
  pname = "zigduck-rs";
  inherit version;
  src = src;
  cargoLock = { lockFile = src + "/Cargo.lock"; };

  env.CMAKE_POLICY_VERSION_MINIMUM = "3.5";

  nativeBuildInputs = [
    pkgs.pkg-config
    pkgs.cmake
    pkgs.libclang
    rustPlatform.bindgenHook
  ];

  buildInputs = [ 
    pkgs.openssl.dev
    pkgs.mosquitto
    pkgs.zigbee2mqtt
  ];

  meta = with lib; {
    description = "Home automation system written in Rust";
    license = licenses.mit;
    maintainers = [ "QuackHack-McBlindy" ];
    mainProgram = "zigduck-rs";
    
  };}
