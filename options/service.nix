{
  config,
  lib,
  pkgs,
  ...
} : let
  inherit (lib) mkOption mkEnableOption types;
  Types = import ./types.nix { inherit lib; };
in {

  options.services.zigduck = {
    enable = mkEnableOption "Zigduck";

    # zigduck-rs service options
    broker = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "MQTT broker hostname or IP address";
    };

    stateDir = mkOption {
      type = types.path;
      default = "/var/lib/zigduck";
      description = "Directory for runtime state files";
    };

    debug = mkOption {
      type = types.bool;
      default = false;
      description = "Enable debug logging (sets DEBUG=1)";
    };

    extraEnv = mkOption {
      type = types.attrsOf types.str;
      default = {};
      description = "Extra environment variables to pass to the service";
    };
    
    # zigduck-dashboard servoce options
    dashboard = {
      enable = mkEnableOption "Zigduck dashboard service";
      host = mkOption {
        type = types.str;
        default = "0.0.0.0";
        description = "Host to bind the webserver to";
      };
      port = mkOption {
        type = types.port;
        default = 13336;
        description = "Port for the dashboard";
      };
      openFirewall = mkOption {
        type = types.bool;
        default = false;
        description = "Whether to open the firewall for the specified port.";
      };      
      passwordFile = mkOption {
        type = types.nullOr types.path;
        default = null;
        description = "Path to password file for API authentication (API_PASSWORD_FILE)";
      };
      secure = mkOption {
        type = types.bool;
        default = true;
        description = "Whether to set the Secure flag on authentication cookies. Set to false when serving over plain HTTP.";
      };
    };


    # zigduck-cli options
    cli = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Whether to install zigduck-cli package.";
      };
    };
    
  };}


