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
        enable = lib.mkEnableOption "Enable this Android TV device";
        room = lib.mkOption {
          type = lib.types.enum (lib.attrNames config.house.rooms);
          description = "Room where the TV is located";
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
                description = "Android Keycode for turning the TV off";
              };
              power_on = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_WAKEUP";
                description = "Android Keycode for turning the TV on";
              };
              play_pause = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_MEDIA_PLAY_PAUSE";
                description = "Android Keycode for play/pause";
              };
              next = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_MEDIA_NEXT";
                description = "Android Keycode for next media";
              };
              previous = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_MEDIA_PREVIOUS";
                description = "Android Keycode for previous media";
              };
              volume_up = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_VOLUME_UP";
                description = "Android Keycode for volume up";
              };
              volume_down = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_VOLUME_DOWN";
                description = "Android Keycode for volume down";
              };
              channel_up = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_CHANNEL_UP";
                description = "Android Keycode for channel up";
              };
              channel_down = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_CHANNEL_DOWN";
                description = "Android Keycode for channel down";
              };
              nav_up = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_DPAD_UP";
                description = "Android Keycode for navigation up";
              };
              nav_down = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_DPAD_DOWN";
                description = "Android Keycode for navigation down";
              };
              nav_left = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_DPAD_LEFT";
                description = "Android Keycode for navigation left";
              };
              nav_right = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_DPAD_RIGHT";
                description = "Android Keycode for navigation right";
              };
              nav_select = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_DPAD_CENTER";
                description = "Android Keycode for navigation select/enter";
              };
              nav_back = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_BACK";
                description = "Android Keycode for back navigation";
              };
              nav_home = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_HOME";
                description = "Android Keycode for home";
              };
              nav_menu = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_MENU";
                description = "Android Keycode for menu";
              };
              nav_recents = lib.mkOption {
                type = lib.types.str;
                default = "KEYCODE_APP_SWITCH";
                description = "Android Keycode for recent apps";
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
                description = "Optional file path for channel icon used on dashboard frontend";
                default = null;
              };
              id = lib.mkOption {
                type = lib.types.nullOr lib.types.int;
                default = null;
                description = "Numeric channel ID used when no `cmd` or `stream_url` is provided.";
              };
              cmd = lib.mkOption {
                type = lib.types.str;
                description = "Sequence of ADB commands to launch the channel, separated by `&&`. Overrides `stream_url` and `id`.";
                default = "";
              };
              stream_url = lib.mkOption {
                type = lib.types.str;
                description = "Direct stream URL to play on the device. Used when `cmd` is empty, overrides `id`.";
                default = "";
              };
              scrape_url = lib.mkOption {
                type = lib.types.str;
                description = ''
                  URL used by external tools to scrape TV guide data for this channel.
                  Not used directly by the TV binary.
                '';  
                default = "";
              };
            };
          });
          description = "TV channel options";
          default = {};
        };
      };
    });
    default = {};

  };}
