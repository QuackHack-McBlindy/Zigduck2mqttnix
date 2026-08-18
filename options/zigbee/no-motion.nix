{
  config,
  lib,
  pkgs,
  ...
} : let
  inherit (lib) mkOption types;
in {

  options.house.zigbee.no.motion = mkOption {
    type = lib.types.submodule {
      options = {
        trigger.all = lib.mkOption {
          type = lib.types.submodule {
            options = {
              lights.off = lib.mkOption {
                description = "No motion-triggered lighting behavior";
                type = lib.types.submodule {
                  options = {
                    enable = lib.mkEnableOption "Enable Zigbee motion handling" // {
                      default = false;
                    };
                    after = lib.mkOption {
                      type = lib.types.int;
                      default = 180;
                      description = "Time in minutes without motion that trigger all lights off.";
                    };
                    exclude = lib.mkOption {
                      type = lib.types.listOf lib.types.str;
                      default = [];
                      description = "List of devices (strings) to exclude, leaving them in current state.";
                    };
                  };
                };
                default = {};
              };
            };
          };
          default = {};
        };
      };
    };
    default = {};

  };}
