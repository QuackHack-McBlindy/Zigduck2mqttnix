{
  config,
  lib,
  pkgs,
  ...
} : let
  inherit (lib) mkOption types;
in {

  options.house.zigbee.scenes = mkOption {        
    type = lib.types.attrsOf (lib.types.attrsOf (lib.types.attrs));
    default = {};
    description = "Scenes for Zigbee devices";

  };}
 
