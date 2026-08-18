{
  config,
  lib,
  pkgs,
  ...
} : let
  inherit (lib) mkOption types;
in {

  options.house.media = mkOption {
    type = types.submodule {
      options = {
        root = mkOption {
          type = types.nullOr types.path;
          default = null;
          description = "Root directory for all media";
        };

        playlistFile = mkOption {
          type = types.nullOr types.path;
          default = config.house.media.root + "/playlist.m3u";
          description = "Filepath for default playlist";
        };

        movies = mkOption {
          type = types.path;
          default = config.house.media.root + "/Movies";
          description = "Movies directory";
        };

        tv = mkOption {
          type = types.path;
          default = config.house.media.root + "/TV";
          description = "TV shows directory";
        };

        music = mkOption {
          type = types.path;
          default = config.house.media.root + "/Music";
          description = "Music directory";
        };

        musicVideos = mkOption {
          type = types.path;
          default = config.house.media.root + "/Music_Videos";
          description = "Music videos directory";
        };

        otherVideos = mkOption {
          type = types.path;
          default = config.house.media.root + "/Other_Videos";
          description = "Other videos directory";
        };

        podcasts = mkOption {
          type = types.path;
          default = config.house.media.root + "/Podcasts";
          description = "Podcasts directory";
        };

        audiobooks = mkOption {
          type = types.path;
          default = config.house.media.root + "/Audiobooks";
          description = "Audiobook directory";
        };

        youtubePasswordFile = mkOption {
          type = types.path;
          description = ''
            Path to a file containing your YouTube Data API v3 key.
            Obtain a key from https://console.cloud.google.com/apis/credentials.
            The file should contain only the API key string (no extra whitespace).
          '';
        };
      };
    };
    default = {};
    description = "Media library configuration";

  };}    
