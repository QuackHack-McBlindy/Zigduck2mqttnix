{
  config,
  lib,
  pkgs,
  ...
} : let
  inherit (lib) mkOption types mkEnableOption;
in {

  options.house.zigbee.hueSyncBox = mkOption {
    type = types.nullOr (types.submodule {
      options = {
        enable = mkEnableOption "Enable Philips Hue Bridge & Sync Box integration";      
        # hue bridge configuration
        bridge = {
          ip = mkOption {
            type = types.str;
            description = "IP address of the Philips Hue Bridge";
          };
          passwordFile = mkOption {
            type = types.path;
            description = ''
              Path to a file containing the Hue Bridge API key (username).
              To get a key, press the physical button on your Hue Bridge, then run:
                curl -X POST http://<bridge-ip>/api -d '{"devicetype":"house#nixos"}'
              The response will contain a `username` field – save that string in this file.
            '';
          };
        };      
        # hue sync box configuration
        syncBox = {
          ip = mkOption {
            type = types.str;
            description = "IP address of the Philips Hue Sync Box";
          };
          passwordFile = mkOption {
            type = types.path;
            description = "File containing the Hue Sync Box API key";
          };
          tv = mkOption {
            type = types.str;
            description = "What TV should syncBox syncronize the lights to. Available TVs: ${lib.concatStringsSep ", " (lib.attrNames config.house.tv)}";
            default = "";
            example = "shield";
            apply = tvName:
              if tvName == "" then tvName
              else if builtins.hasAttr tvName config.house.tv then tvName
              else throw "TV '${tvName}' is not defined in house.tv. Available: ${lib.concatStringsSep ", " (lib.attrNames config.house.tv)}";
          };
        };      
        insecure = mkOption {
          type = types.bool;
          default = false;
          description = "Allow insecure HTTP for Bridge (use with caution!)";
        };
        skipCertCheck = mkOption {
          type = types.bool;
          default = true;
          description = "Skip SSL certificate verification for Sync Box (self-signed cert)";
        };
      };
    });
    default = null;
    description = "Philips Hue Bridge & Sync Box configuration for TV to lights syncing";

  };}    
