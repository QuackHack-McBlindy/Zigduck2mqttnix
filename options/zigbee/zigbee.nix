{
  config,
  lib,
  pkgs,
  ...
} : let
  inherit (lib) mkOption types;
  format = pkgs.formats.yaml { };
in {

  options.house.zigbee = {
    enable = lib.mkEnableOption "zigbee2mqtt service";
    dataDir = lib.mkOption {
      description = "Zigbee2mqtt data directory";
      default = "/var/lib/zigbee";
      type = lib.types.path;
    };
    settings = lib.mkOption {
      type = format.type;
      default = { };
      example = lib.literalExpression ''
        {
          homeassistant.enabled = config.services.home-assistant.enable;
          permit_join = true;
          serial = {
            port = "/dev/ttyACM1";
          };
        }
      '';
      description = ''
        Please note that this flake already handles the configuration of Zigbee2MQTT internally.
        These would be overrides.
        Your configuration.yaml as a Nix attribute set.
        Check the [documentation](https://www.zigbee2mqtt.io/information/configuration.html) for possible options.
      '';
    };
    networkKeyFile = mkOption {
      type = types.path;
      description = ''
        Path to a file containing the Zigbee network key used by
        Zigbee2MQTT (`network_key`).

        The network key identifies and secures the Zigbee network. Keep
        this key stable: changing or losing it will require Zigbee devices
        to be paired with the network again.

        The file should contain the key in the following format:
        ```
        - 86
        - 208
        ...
        ```
      '';
    };

  };}
