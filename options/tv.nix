{
  config,
  lib,
  pkgs,
  ...
} : let
  inherit (lib) mkOption types;
in {

  options.house.tv = mkOption {
    type = lib.types.attrsOf (lib.types.submodule {
      options = {
        enable = lib.mkEnableOption "Enable this Android TVOS device";
        room = lib.mkOption {
          type = lib.types.strMatching (lib.concatStringsSep "|" (lib.attrNames config.house.rooms));
          description = "Room where TV is located";
        };
        ip = lib.mkOption {
          type = lib.types.str;
          description = "TV's static IP address";
        };
        isDefault = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = ''
            Whether this TV is the default device when multiple TVs are defined
            and no specific room or IP is requested.
          '';
        };

        apps = lib.mkOption {
          type = lib.types.attrsOf lib.types.str;
          default = {};
          description = "App package names and activities for this TV device";
          example = {
            telenor = "se.telenor.stream/.MainActivity";
            tv4 = "se.tv4.tv4playtab/se.tv4.tv4play.ui.mobile.main.BottomNavigationActivity";
          };
        };

        # key event mapping as individual options
        keymap = lib.mkOption {
          type = lib.types.submodule {
            options = {
              power_off = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_SLEEP";
                description = "Keycode for turning the TV off";
              };
              power_on = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_WAKEUP";
                description = "Keycode for turning the TV on";
              };
              play_pause = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_MEDIA_PLAY_PAUSE";
                description = "Keycode for play/pause";
              };
              next = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_MEDIA_NEXT";
                description = "Keycode for next media";
              };
              previous = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_MEDIA_PREVIOUS";
                description = "Keycode for previous media";
              };
              volume_up = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_VOLUME_UP";
                description = "Keycode for volume up";
              };
              volume_down = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_VOLUME_DOWN";
                description = "Keycode for volume down";
              };
              channel_up = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_CHANNEL_UP";
                description = "Keycode for channel up";
              };
              channel_down = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_CHANNEL_DOWN";
                description = "Keycode for channel down";
              };
              nav_up = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_DPAD_UP";
                description = "Keycode for navigation up";
              };
              nav_down = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_DPAD_DOWN";
                description = "Keycode for navigation down";
              };
              nav_left = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_DPAD_LEFT";
                description = "Keycode for navigation left";
              };
              nav_right = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_DPAD_RIGHT";
                description = "Keycode for navigation right";
              };
              nav_select = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_DPAD_CENTER";
                description = "Keycode for navigation select/enter";
              };
              nav_back = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_BACK";
                description = "Keycode for back navigation";
              };
              nav_home = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_HOME";
                description = "Keycode for home";
              };
              nav_menu = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_MENU";
                description = "Keycode for menu";
              };
              nav_recents = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_APP_SWITCH";
                description = "Keycode for recent apps";
              };
            };
          };
          default = {};
          description = "Mapping of logical actions to Android keycodes";
        };

        # TV channel definitions
        channels = lib.mkOption {
          type = lib.types.attrsOf (lib.types.submodule {
            options = {
              name = lib.mkOption {
                type = lib.types.str;
                description = "Channel display name";
              };
              icon = lib.mkOption {
                type = types.nullOr types.path;
                description = "Optional file path for channel icon used for the generated TV-guide web frontend";
                default = null;
              };
              id = lib.mkOption {
                type = lib.types.nullOr lib.types.int;
                default = null;
                description = "Channel ID number, when set will send defined value as ADB channel command.";
              };
              cmd = lib.mkOption {
                type = lib.types.str;
                description = "Sequence of ADB commands to launch channel. Seperated with && (Overrides ID)";
                default = "";
              };
              stream_url = lib.mkOption {
                type = lib.types.str;
                description = "Stream URL to send to device. (Overrides ID)";
                default = "";
              };
              scrape_url = lib.mkOption {
                type = lib.types.str;
                description = ''
                  URL from which to scrape TV guide data for this channel.
                  Only used as reference for user provided external script.
                '';
                default = "";
              };
            };
          });
          description = "TV channel options";
        };
      };
    });
    default = {};

  };}    
