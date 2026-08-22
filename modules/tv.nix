{ 
  self,
  config,
  lib,
  pkgs,
  ...
} : with lib;
let
  cfg = config.services.zigduck;  
  house = config.house;
  zigduckPkgs = self.inputs.zigduck2mqttnix.packages.${pkgs.system};

  format = pkgs.formats.yaml { };
  configFile = format.generate "zigbee2mqtt.yaml" house.zigbee.settings;

  tvDefaultsJsonFile = let
    rooms = lib.mapAttrs (_: tv: tv.ip) house.tv;
  
    enabledTVs = lib.filterAttrs (_: tv: tv.enable) config.house.tv;
  
    tvEntries = lib.mapAttrs (name: tv: {
      ip = tv.ip;
      room = tv.room;
      is_default = tv.isDefault or false;
      keymap = tv.keymap;
      apps = tv.apps or {};
      channels = tv.channels or {};
    }) enabledTVs;
  
    defaultTVName = let
      defaults = lib.filterAttrs (_: tv: tv.isDefault) enabledTVs;
    in
      if defaults != {} then lib.head (lib.attrNames defaults)
      else if enabledTVs != {} then lib.head (lib.attrNames enabledTVs)
      else throw "No enabled TVs defined";
  
    directories = {
      root        = house.media.root;
      tv          = house.media.tv;
      movie       = house.media.movies;
      music       = house.media.music;
      podcast     = house.media.podcasts;
      musicvideo  = house.media.musicVideos;
      othervideo  = house.media.otherVideos;
      audiobook   = house.media.audiobooks;
    };
  
    tvDefaultsAttrSet = {
      device_ip = config.house.tv.${defaultTVName}.ip;
      inherit rooms directories;
      tvs = tvEntries;
      webserver_file = if house.https.urlFile != null
                       then house.https.urlFile
                       else null;
      playlist_file  = house.media.playlistFile;
      favourites_file  = house.media.favouritesFile;
      max_items      = 200;
      shuffle        = true;
      youtube_api_key_file = if house.media.youtubePasswordFile != null
                             then house.media.youtubePasswordFile
                             else null;
      mqtt_password_file   = if house.zigbee.mosquitto != null
                             then house.zigbee.mosquitto.passwordFile
                             else null;
    };
  in
    pkgs.writeText "tv-ctl-defaults.json" (builtins.toJSON tvDefaultsAttrSet);

in {
  config = mkMerge [
    (mkIf (cfg.enable || cfg.cli.enable) {
      environment.systemPackages = [
        zigduckPkgs.tv
      ];
      
      environment.etc."zigduck/tv-defaults.json".source = tvDefaultsJsonFile;

    })

  ];}
