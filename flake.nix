{
  description = "zigduck2mqttnix is a complete home automation system in Nix and Rust.";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    tinyFlake.url = "github:quackhack-mcblindy/tinyFlake";
  };

  outputs = { self, nixpkgs, tinyFlake, ... }@inputs:
    tinyFlake.lib.mkFlake {
      inherit self inputs;
      systems = [ "x86_64-linux" "aarch64-linux" ];
      packages = tinyFlake.lib.mapModules ./packages import;
      nixosModules.zigduck2mqttnix = import ./modules;
              
    };}
