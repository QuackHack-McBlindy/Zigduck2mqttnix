{
  self,
  lib,
  pkgs,
  rustPlatform,
  fetchFromGitHub,
  ...
} : let

in  
rustPlatform.buildRustPackage {
  pname = "zigduck-rs";
  version = "0.1.1";

  src = ./zigduck-rs;
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
