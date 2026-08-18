{
  config,
  lib,
  pkgs,
  ...
} : let
  inherit (lib) mkOption types;
  Types = import ./types.nix { inherit lib; };
in {

  options.house.dashboard = mkOption {
    type = types.submodule {
      options = {
        pages = mkOption {
          type = types.attrsOf (types.submodule {
            options = {
              icon = mkOption {
                type = types.str;
                description = "Icon for the tab (FontAwesome class, MDI class, or image URL)";
                default = "fas fa-question";
              };
              code = mkOption {
                type = types.str;
                description = "HTML and JavaScript code for the page";
                default = "";
              };
              title = mkOption {
                type = types.str;
                description = "Title for the page (optional)";
                default = "";
              };
              files = mkOption {
                type = types.attrsOf (types.oneOf [types.path types.str]);
                default = {};
                description = "Files to be symlinked to the http server for this page";
              };
              css = mkOption {
                type = types.str;
                default = "";
                description = "Additional CSS for this page";
              };
            };
          });
          default = {};
          description = "Custom pages for the dashboard";
        };

        statusCards = mkOption {
          type = types.attrsOf Types.statusCardType;
          default = {};
          description = "Configurable status cards for the dashboard";
        };
      };
    };
    default = {};
    description = "Dashboard configuration";

  };}    
