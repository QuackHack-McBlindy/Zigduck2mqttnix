{
  config,
  lib,
  pkgs,
  ...
} : let
  inherit (lib) mkOption types;
in {

  options.house.zigbee.dimmer = mkOption {
    type = types.submodule {
      options = {
        # which mqtt field contains the action
        message = lib.mkOption {
          type = lib.types.str;
          default = "action";
          description = "MQTT field name containing dimmer action";
        };
        # action mappings
        actions = {
          onPress = lib.mkOption {
            type = lib.types.str;
            default = "on_press_release";
            description = "Action for single press of ON button";
          };
          onHold = lib.mkOption {
            type = lib.types.str;
            default = "on_hold_release";
            description = "Action for holding ON button";
          };
          offPress = lib.mkOption {
            type = lib.types.str;
            default = "off_press_release";
            description = "Action for single press of OFF button";
          };
          offHold = lib.mkOption {
            type = lib.types.str;
            default = "off_hold_release";
            description = "Action for holding OFF button";
          };
          upPress = lib.mkOption {
            type = lib.types.str;
            default = "up_press_release";
            description = "Action for single press of UP button";
          };
          upHold = lib.mkOption {
            type = lib.types.str;
            default = "up_hold_release";
            description = "Action for holding UP button";
          };
          downPress = lib.mkOption {
            type = lib.types.str;
            default = "down_press_release";
            description = "Action for single press of DOWN button";
          };
          downHold = lib.mkOption {
            type = lib.types.str;
            default = "down_hold_release";
            description = "Action for holding DOWN button";
          };
        };
        doubleClickTimeout = mkOption {
          type = types.nullOr types.int;
          default = null;
          description = "Timeout for double‑click detection in milliseconds (defaults to 300).";
        };
      };
    };
    default = {
      message = "action";
      actions = {
        onPress = "on_press_release";
        onHold = "on_hold_release";
        offPress = "off_press_release";
        offHold = "off_hold_release";
        upPress = "up_press_release";
        upHold = "up_hold_release";
        downPress = "down_press_release";
        downHold = "down_hold_release";
      };
    };
    description = "Configuration for dimmer switches. Default configuration is for the Philips Hue Dimmer Switch. You can check the message for your specific dimmer at zigbee2MQTT documentation.";

  };}
