{
  config,
  lib,
  pkgs,
  ...
} : let
  inherit (lib) mkOption types mkEnableOption;
in {

  options.house.zigbee.motion = mkOption {
    type = lib.types.submodule {
      options = {
        enable = lib.mkEnableOption "Enable Zigbee motion handling" // {
          default = true;
        };
        when.dark.enable = lib.mkEnableOption "Enable Zigbee motion handling" // {
          default = true;
        };
        trigger = lib.mkOption {
          type = lib.types.submodule {
            options = {
              lights = lib.mkOption {
                description = "Motion-triggered lighting behavior";
                type = lib.types.submodule {
                  options = {
                    after = lib.mkOption {
                      type = lib.types.int;
                      default = 16;
                      description = "Lights activate only after this hour (24h)";
                    };
                    before = lib.mkOption {
                      type = lib.types.int;
                      default = 9;
                      description = "Lights activate only before this hour (24h)";
                    };
                    duration = lib.mkOption {
                      type = lib.types.int;
                      default = 900;
                      description = "Seconds to keep lights on after motion";
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

