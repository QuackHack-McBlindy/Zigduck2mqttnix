{
  config,
  lib,
  pkgs,
  ...
} : let
  inherit (lib) mkOption types;
in {

  options.house.zigbee.devices = mkOption {
    type = types.attrsOf (types.submodule {
      options = {
        friendly_name = mkOption {
          type = types.str;
          description = "A human-readable device name.";
          example = "Kitchen Dimmer";
        };
        room = mkOption {
          type = types.strMatching (lib.concatStringsSep "|" (lib.attrNames config.house.rooms));
          description = "The room this device belongs to.";
          example = "kitchen";
        };
        type = mkOption {
          type = types.enum [ "light" "hue_light" "dimmer" "sensor" "motion" "outlet" "remote" "pusher" "blind" ];
          description = "The type of device.";
          example = "light";
        };
        icon = mkOption {
          type = types.str;
          default = "mdi:monitor-shimmer";
          description = "Material Design icon name representing this device.";
        };
        batteryType = mkOption {
          type = types.nullOr types.str;
          default = null;
          description = "Optional type of battery the device uses.";
        };
        supports_color = mkOption {
          type = types.bool;
          default = false;
          description = "Whether the light supports color.";
        };
        supports_temperature = mkOption {
          type = types.bool;
          default = false;
          description = "Whether the light supports temperature.";
        };
        endpoint = mkOption {
          type = types.int;
          description = "The Zigbee endpoint to control this device.";
          example = 11;
        };
        hue_id = mkOption {
          type = types.nullOr types.int;
          default = null;
          description = "The light_id for the device (Hue integration).";
        };
      };
    });
    default = {};
    description = "Zigbee device definitions keyed by device ID.";

  };}    
