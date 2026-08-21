{
  config,
  lib,
  pkgs,
  ...
} : let
  inherit (lib) mkOption types mkEnableOption;
in {

  options.house.https = mkOption {
    type = types.submodule {
      options = {
        urlFile = mkOption {
          type = types.nullOr types.path;
          default = null;
          example = "/run/secrets/url";
          description = ''
            Path to a file containing the public HTTPS URL used to access the
            media library.

            The URL must use HTTPS and should point to the web server that serves
            `house.media.root`. For example:

              https://media.my-domain.org

            This is required by some media clients, such as Android TV, when
            accessing external `.m3u` playlists.

            Domain does not need to be publicly accessable, as long as the certificate is valid.

            You need a domain name with a valid TLS certificate. A free dynamic
            DNS provider such as DuckDNS can be used if you do not already have
            a domain.
          '';
        };
      };
    };
    default = {};
    description = "HTTPS configuration for the media library";

  };}
