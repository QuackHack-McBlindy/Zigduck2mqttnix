{
  config,
  lib,
  pkgs,
  ...
} : let
  inherit (lib) mkOption types;
  Types = import ./types.nix { inherit lib; };
in {

  options.house.rooms = mkOption {
    type = types.attrsOf Types.roomType;
    default = {
      bedroom.icon = "mdi:bedroom";
      hallway.icon = "mdi:hallway";
      kitchen.icon = "mdi:sofa";
      livingroom.icon = "mdi:toilet";
      wc.icon = "mdi:toilet";
    };
    description = "A set of rooms in the house with their attributes.";

  };}    
