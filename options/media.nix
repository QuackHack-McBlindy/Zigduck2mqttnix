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
          type = types.path;
          default = "/Media";
          example = "/Media";
          description = ''
            Absolute path to the root media directory.
            All other media directories should be subdirectories of this path.
            The URL configured at `config.house.https.urlFile` should point to this directory as the root of a file server.
          '';
        };

        playlistFile = mkOption {
          type = types.path;
          default = config.house.media.root + "/playlist.m3u";
          example = "/Media/playlist.m3u";
          description = ''
            File path where the generated playlist will be written.
            This file is overwritten each time the TV controller creates a new playlist.
            It must be writable by the user executing the tv command.
            Should be inside a directory exposed by the web server.
          '';  
        };
        
        favouritesFile = mkOption {
          type = types.path;
          default = config.house.media.root + "/favourites.m3u";
          example = "/Media/favourites.m3u";
          description = ''
            File path where the playlist for starred media will be written.
            This is a Persistent playlist for the users favourite tracks.
            It must be writable by the user executing the tv command.
            Should be inside a directory exposed by the web server.
          '';  
        };

        movies = mkOption {
          type = types.path;
          default = config.house.media.root + "/Movies";
          example = "/Media/Movies";
          description = ''
            Directory containing movie folders.
            Each movie should be in its own subdirectory, e.g.:
              Movies/Some Movie (2020)/movie.mkv
          '';
        };

        tv = mkOption {
          type = types.path;
          default = config.house.media.root + "/TV";
          example = "/Media/TV";
          description = ''
            Directory containing TV show folders.
            Each show should be in its own subdirectory.
            Inside each show folder, seasons are expected in folders like:
              "Season 1", "Season 01", "S01", "s01"
            Episode files go inside those season folders.
          '';
        };

        music = mkOption {
          type = types.path;
          default = config.house.media.root + "/Music";
          example = "/Media/Music";
          description = ''
            Directory containing music. Can be flat or hierarchical.

            For artist/album search (typ = "music"), each artist or album
            should be in its own subdirectory.

            For song search (typ = "song"), files are searched recursively
            by filename with extensions: mp3, flac, m4a, wav.

            For jukebox (typ = "jukebox"), all files under this directory
            are shuffled and played.
          '';
        };

        musicVideos = mkOption {
          type = types.path;
          default = config.house.media.root + "/Music_Videos";
          example = "/Media/MusicVideos";
          description = ''
            Directory containing music video folders.
            Each music video should be in its own subdirectory.
          '';
        };

        otherVideos = mkOption {
          type = types.path;
          default = config.house.media.root + "/Other_Videos";
          example = "/Media/Videos";
          description = ''
            Directory containing miscellaneous video files.
            Videos are searched recursively by filename with extensions:
              mp4, mkv, avi, mov
            No folder structure requirement beyond being under this path.
          '';
        };

        podcasts = mkOption {
          type = types.path;
          default = config.house.media.root + "/Podcasts";
          example = "/Media/Podcasts";
          description = ''
            Directory containing podcast folders.
            Each podcast should be in its own subdirectory.
            Episodes are files inside those folders.
          '';
        };

        audiobooks = mkOption {
          type = types.path;
          default = config.house.media.root + "/Audiobooks";
          example = "/Media/Audiobooks";
          description = ''
            Directory containing audiobook folders.
            Each audiobook should be in its own subdirectory.
          '';
        };

        youtubePasswordFile = mkOption {
          type = types.nullOr types.path;
          default = null;
          description = ''
            Path to a file containing a YouTube Data API v3 key.
            The file should contain only the API key string (no extra whitespace).

            To obtain a key:
              1. Go to https://console.cloud.google.com/apis/credentials
              2. Create a new API key.
              4. Save the key to a file.

            If set to null, YouTube playback is disabled.
          '';
          example = "/run/secrets/youtube_api_key";
        };
      };
    };
    default = {};
    description = "Media library configuration";

  };}
