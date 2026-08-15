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

  zigbeeDevices = house.zigbee.devices;
  
  scenes = house.zigbee.scenes;
  sceneConfig = pkgs.writeText "scene-config.json" (builtins.toJSON {
    scenes = scenes;
  });

  sceneConfigCli = pkgs.writeText "scene-config-cli.json" (builtins.toJSON (
    lib.mapAttrs (sceneName: sceneDevices: {
      friendly_name = sceneName;
      devices = sceneDevices;
    }) scenes
  ));
        
  byRoom = lib.foldlAttrs (acc: id: dev:
    lib.recursiveUpdate acc {
      ${dev.room} = (acc.${dev.room} or []) ++ [ id ];
    }) {} zigbeeDevices;

  groupConfig = lib.mapAttrs' (room: ids: {
    name = room;
    value = {
      friendly_name = room;
      devices = map (id: 
        let dev = zigbeeDevices.${id};
        in "${id}/${toString dev.endpoint}"
      ) ids;
    };
  }) byRoom;

  format = pkgs.formats.yaml { };
  configFile = format.generate "zigbee2mqtt.yaml" house.zigbee.settings;
 
  tvDevicesJson = pkgs.writeText "tv-devices.json" (builtins.toJSON house.tv);

  ieeeToFriendly = lib.mapAttrs (ieee: dev: dev.friendly_name) zigbeeDevices;
  mappingJSON = builtins.toJSON ieeeToFriendly;
  mappingFile = pkgs.writeText "ieee-to-friendly.json" mappingJSON;

  deviceMeta = builtins.toJSON (
    lib.listToAttrs (
      lib.filter (attr: attr.name != null) (
        lib.mapAttrsToList (ieee: dev: {
          name = dev.friendly_name;
          value = {
            id = dev.friendly_name;
            room = dev.room;
            type = dev.type;
            endpoint = dev.endpoint;
            ieee = ieee;
          
            # CLI
            friendly_name = dev.friendly_name;
            hue_id = dev.hue_id or null;
            supports_color = dev.supports_color or null;
            supports_temperature = dev.supports_temperature or null;
            icon = dev.icon or null;
            battery_type = dev.battery_type or null;
          };
        }) zigbeeDevices
      )
    )
  );
  
  deviceConfig = 
    let
      filteredDevices = lib.filterAttrs (_: dev: dev.type != "hue_light") zigbeeDevices;
    in
    lib.mapAttrs (id: dev: {
      friendly_name = dev.friendly_name;
    }) filteredDevices;


  automationsJSON = builtins.toJSON house.zigbee.automations;
  automationsFile = pkgs.writeText "automations.json" automationsJSON;

  dashboardConfig = lib.filterAttrs (_: card: card.enable) house.dashboard.statusCards;
  dashboardConfigJSON = builtins.toJSON {
      cards = lib.mapAttrs (name: card: {
          enable = card.enable;
          title = card.title;
          icon = card.icon;
          color = card.color;
          on_click_action = card.on_click_action or [];
      }) dashboardConfig;
  };
  dashboardConfigFile = pkgs.writeText "dashboard-config.json" dashboardConfigJSON;
 
  statusCardsConfigJson = pkgs.writeText "status-cards-config.json" (builtins.toJSON {
    cards = lib.mapAttrs (name: card: {
      inherit name;
      title = card.title;
      group = card.group or "default";
      icon = card.icon;
      color = card.color;
      theme = card.theme or "neon";
      fileName = builtins.baseNameOf card.filePath;
      jsonField = card.jsonField;
      format = card.format;
      detailsJsonField = card.detailsJsonField or null;
      detailsFormat = card.detailsFormat or "";
      details = card.details or "";
      defaultDetails = card.defaultDetails or "";
      defaultValue = card.defaultValue or "--";
      chart = card.chart or false;
      historyField = card.historyField or "history";
      on_click_action = card.on_click_action or [];
    }) (lib.filterAttrs (_: card: card.enable) house.dashboard.statusCards);
    enabled = builtins.attrNames (lib.filterAttrs (_: card: card.enable) house.dashboard.statusCards);
  });

  devices-json = pkgs.writeText "devices.json" deviceMeta;
  jsonFormat = pkgs.formats.json { };

  mainConfig = {
    mosquitto = {
      broker = house.zigbee.mosquitto.host;
      user = house.zigbee.mosquitto.username;
      password_file = house.zigbee.mosquitto.passwordFile; 
      base_topic = house.zigbee.mosquitto.baseTopic;
    };
    hue = {
      bridge_ip = house.zigbee.hueSyncBox.bridge.ip;
      password_file = house.zigbee.hueSyncBox.bridge.passwordFile;
    };
    dark_time = {
      enabled = house.zigbee.motion.when.dark.enable;
      after = house.zigbee.motion.trigger.lights.after;
      before = house.zigbee.motion.trigger.lights.before;
      duration = house.zigbee.motion.trigger.lights.duration;
    };
    dimmer = {
      message_key = house.zigbee.dimmer.message;
      actions = {
        on_press = house.zigbee.dimmer.actions.onPress;
        on_hold = house.zigbee.dimmer.actions.onHold;
        off_press = house.zigbee.dimmer.actions.offPress;
        off_hold = house.zigbee.dimmer.actions.offHold;
        up_press = house.zigbee.dimmer.actions.upPress;
        up_hold = house.zigbee.dimmer.actions.upHold;
        down_press = house.zigbee.dimmer.actions.downPress;
        down_hold = house.zigbee.dimmer.actions.downHold;
      };
      double_click_timeout_ms = house.zigbee.dimmer.doubleClickTimeout;
    };
    api = {
      url = "http://${house.zigbee.mosquitto.host}:${toString cfg.dashboard.port}";
      password_file = cfg.dashboard.passwordFile;
    };
    motion = {    
      enabled = house.zigbee.motion.enable;
    };
    no_motion = {
      enabled = house.zigbee.no.motion.trigger.all.lights.off.enable;
      after = house.zigbee.no.motion.trigger.all.lights.off.after;
      exclude = house.zigbee.no.motion.trigger.all.lights.off.exclude;
    };
  };

  zigduckConfigFile = jsonFormat.generate "config.json" mainConfig;

  allTVs = builtins.attrValues house.tv;
  defaultIP = if allTVs == [] then "127.0.0.1" else (builtins.head allTVs).ip;
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

  rooms = builtins.listToAttrs (
    lib.mapAttrsToList (name: value: { inherit name; value = value.ip; }) house.tv
  );

  tvDefaultsAttrSet = {
    device_ip   = defaultIP;
    inherit rooms;
    inherit directories;
    webserver_file = if house.https.urlFile != null
                             then house.https.urlFile
                             else null;    
    playlist_file  = house.media.root + "/playlist.m3u";
    max_items      = 200;
    shuffle        = true;
    youtube_api_key_file = if house.media.youtubePasswordFile != null
                             then house.media.youtubePasswordFile
                             else null;
    mqtt_password_file   = if house.zigbee.mosquitto != null
                             then house.zigbee.mosquitto.passwordFile
                             else null;
  };

  tvDefaultsJsonFile = pkgs.writeText "tv-ctl-defaults.json"
    (builtins.toJSON tvDefaultsAttrSet);

  dashboardConfigFiles = pkgs.writeText "dashboard-config.json" (builtins.toJSON {
    dashboard_static_root = "${cfg.stateDir}/dashboard";
    port = cfg.dashboard.port;
    secure_cookies = cfg.dashboard.secure; 
    state_file = "${cfg.stateDir}/state.json";
    alarms_file = "${cfg.stateDir}/alarms.json";
    health_dir = "${cfg.stateDir}/health";
    uploads_dir = "${cfg.stateDir}/uploads";
    devices_file = "/etc/zigduck/devices.json";
    scenes_file = "/etc/zigduck/scenes.json";
    rooms_file = "/etc/zigduck/rooms.json";
    types_file = "/etc/zigduck/types.json";
    tv_defaults_file = "/etc/zigduck/tv-defaults.json";
    media_root = house.media.root;
    playlist_file = house.media.root + "/playlist.m3u";
    default_tv_ip = defaultIP;
    webserver_secret_file = if house.https.urlFile != null then house.https.urlFile else "";
  });

in {
  imports = [ ./dashboard.nix ];

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
        description = "Whether to install the zg wrapper with default settings.";
      };
    };
  };


  
  config = mkMerge [
    (mkIf cfg.enable {
      environment.systemPackages = [ 
        pkgs.mosquitto
        pkgs.zigbee2mqtt      
      ];
  
      networking.firewall.allowedTCPPorts =
        (map (l: l.port) config.services.mosquitto.listeners)
        ++ [ 
          house.zigbee.settings.frontend.port
        ];
    
      house.zigbee = {
        enable = true;
        dataDir = "/var/lib/zigbee";
        settings = {
          homeassistant = lib.mkDefault false;
          mqtt = {
            server = "mqtt://localhost:1883";
            user = house.zigbee.mosquitto.username;
            password = house.zigbee.mosquitto.passwordFile;
            base_topic = house.zigbee.mosquitto.baseTopic;
          };
          serial = {
            port = "/dev/" + house.zigbee.coordinator.symlink;
            adapter = house.zigbee.coordinator.adapter;
          };
          frontend = { 
            enabled = true;
            host = "0.0.0.0";   
            port = 8099; 
          };
          advanced = {
            homeassistant_legacy_entity_attributes = false;
            homeassistant_legacy_triggers = false;
            legacy_api = false;
            legacy_availability_payload = false;
            transmit_power = 9;
            channel = 15;
            last_seen = "ISO_8601_local";
            pan_id = 60410;
          };
          device_options = { legacy = false; };
          availability = false;
          permit_join = false;
          devices = deviceConfig;
          groups = groupConfig // {
            all_lights = {
              friendly_name = "all";
              devices = lib.concatMap (id: 
                let dev = zigbeeDevices.${id};
                in if dev.type == "light" then ["${id}/${toString dev.endpoint}"] else []
              ) (lib.attrNames zigbeeDevices);
            };
          };
        }; 
      };
      
      systemd.services.zigbee2mqtt = {
        wantedBy = [ "multi-user.target" ];
        after = [ "sops-nix.service" "network.target" "systemd-tmpfiles-setup.service" ];
        wants = [ "systemd-tmpfiles-setup.service" ];
        environment.ZIGBEE2MQTT_DATA = house.zigbee.dataDir;
        preStart = ''
          mkdir -p ${house.zigbee.dataDir}
          cp --no-preserve=mode ${configFile} ${house.zigbee.dataDir}/configuration.yaml
          mosquitto_password=$(cat ${house.zigbee.mosquitto.passwordFile})
          network_key=$(cat ${house.zigbee.networkKeyFile})
          sed -i "s|/run/secrets/mosquitto|$mosquitto_password|" ${house.zigbee.dataDir}/configuration.yaml
          TMPFILE="${house.zigbee.dataDir}/config.yaml"
          CFGFILE="${house.zigbee.dataDir}/configuration.yaml"          
          ${pkgs.gawk}/bin/awk -v keyfile="${house.zigbee.networkKeyFile}" '
            /(^|[[:space:]])network_key:/ { found = 1 }
            { lines[NR] = $0 }
            END {
              if (found) {
                for (i = 1; i <= NR; i++) print lines[i]
              } else {
                advanced_inserted = 0
                for (i = 1; i <= NR; i++) {
                  if (!advanced_inserted && lines[i] ~ /^[[:space:]]*advanced:[[:space:]]*$/) {
                    print lines[i]
                    print "  network_key:"
                    while ((getline line < keyfile) > 0) {
                      print "    " line
                    }
                    close(keyfile)
                    advanced_inserted = 1
                  } else {
                    print lines[i]
                  }
                }
              }
            }
          ' "$CFGFILE" > "$TMPFILE"   
          mv "$TMPFILE" "$CFGFILE"
        '';
  
        serviceConfig = {
          ExecStart = "${pkgs.zigbee2mqtt}/bin/zigbee2mqtt";
          User = "zigbee2mqtt";
          Group = "zigbee2mqtt";
          WorkingDirectory = house.zigbee.dataDir;
          CapabilityBoundingSet = "";
          DeviceAllow = lib.optionals (lib.hasPrefix "/" house.zigbee.settings.serial.port) [
            house.zigbee.settings.serial.port
          ];
          DevicePolicy = "closed";
          LockPersonality = true;
          MemoryDenyWriteExecute = false;
          NoNewPrivileges = true;
          PrivateDevices = false;
          PrivateUsers = true;
          PrivateTmp = true;
          ProtectClock = true;
          ProtectControlGroups = true;
          ProtectHome = true;
          ProtectHostname = true;
          ProtectKernelLogs = true;
          ProtectKernelModules = true;
          ProtectKernelTunables = true;
          ProtectProc = "invisible";
          ProcSubset = "pid";
          ProtectSystem = "strict";
          ReadWritePaths = house.zigbee.dataDir;
          RemoveIPC = true;
          RestrictAddressFamilies = [ "AF_INET" "AF_INET6" "AF_NETLINK" ];
          RestrictNamespaces = true;
          RestrictRealtime = true;
          RestrictSUIDSGID = true;
          SupplementaryGroups = [ "dialout" ];
          SystemCallArchitectures = "native";
          SystemCallFilter = [ "@system-service @pkey" "~@privileged @resources" "@chown" ];
          UMask = "0077";
        };
      };
  
      services.mosquitto = {
        enable = true;
        listeners = [
          {
            acl = [ "pattern readwrite #" ];
            port = 1883;
            omitPasswordAuth = false;
            users.${house.zigbee.mosquitto.username}.passwordFile = house.zigbee.mosquitto.passwordFile;
            settings.allow_anonymous = false;
          }   
          {
            acl = [ "pattern readwrite #" ];
            port = 9001;
            settings.protocol = "websockets";
            omitPasswordAuth = false;
            users.${house.zigbee.mosquitto.username}.passwordFile = house.zigbee.mosquitto.passwordFile;
            settings.allow_anonymous = false;
            settings.require_certificate = false;
          } 
        ];
      };
  
      systemd.services.zigduck = {
        description = "Zigduck Home Automation Service";
        after = [ "network.target" "mosquitto.service" ];
        wants = [ "mosquitto.service" ];
        wantedBy = [ "multi-user.target" ];
  
        serviceConfig = {
          Type = "simple";
          User = "zigduck";
          Group = "zigduck";
          StateDirectory = "zigduck";
          StateDirectoryMode = "0750";
          WorkingDirectory = cfg.stateDir;
          ExecStart = "${zigduckPkgs.zigduck-rs}/bin/zigduck-rs";
          
          Restart = "on-failure";
          RestartSec = "45s";

          Environment = let
            env = {
              MQTT_BROKER = cfg.broker;
              MQTT_USER = house.zigbee.mosquitto.username;
              MQTT_PASSWORD_FILE = house.zigbee.mosquitto.passwordFile;
              ZIGDUCK_CONFIG = "/etc/zigduck/config.json";
              STATE_DIR = cfg.stateDir;
              DT_LOG_LEVEL = "INFO";
              DT_LOG_FILE = cfg.stateDir + "/zigduck.log";
              PATH = "/run/current-system/sw/bin:/run/wrappers/bin:/nix/var/nix/profiles/default/bin:/nix/var/nix/profiles/default/sbin:/run/current-system/sw/sbin";
            } // optionalAttrs cfg.debug { DEBUG = "1"; } // cfg.extraEnv;
          in mapAttrsToList (name: value: "${name}=${value}") env;
        };
      };
      
      services.udev.extraRules = let
        port = house.zigbee.coordinator;
      in
        ''
          SUBSYSTEM=="tty", ATTRS{idVendor}=="${port.vendorId}", ATTRS{idProduct}=="${port.productId}", SYMLINK+="${port.symlink}"
        '';
    })


    (mkIf cfg.dashboard.enable {
      systemd.services.zigduck-dashboard = {
        description = "Zigduck dashboard Service";
        after = [ "network.target" "zigduck.service" ];
        wantedBy = [ "multi-user.target" ];

        serviceConfig = {
          Type = "simple";
          User = "zigduck";
          Group = "zigduck";
          StateDirectory = "zigduck";
          StateDirectoryMode = "0750";
          WorkingDirectory = cfg.stateDir;
          ExecStart = "${zigduckPkgs.zigduck-rs}/bin/zigduck-dashboard ${cfg.dashboard.host} ${toString cfg.dashboard.port}";
          Restart = "on-failure";
          RestartSec = "45s";

          Environment = let
            env = {
              MQTT_BROKER = cfg.broker;
              MQTT_USER = house.zigbee.mosquitto.username;
              MQTT_PASSWORD_FILE = house.zigbee.mosquitto.passwordFile;
              ZIGDUCK_CONFIG_FILE = "/etc/zigduck/dashboard-config.json";
              ZIGDUCK_DASHBOARD_SECURE_COOKIES = toString cfg.dashboard.secure;
              STATE_DIR = cfg.stateDir;
              DT_LOG_LEVEL = "INFO";
              DT_LOG_FILE = cfg.stateDir + "/zigduck.log";
              PATH = "/run/current-system/sw/bin:/run/wrappers/bin:/nix/var/nix/profiles/default/bin:/nix/var/nix/profiles/default/sbin:/run/current-system/sw/sbin";
              HOME = cfg.stateDir;
            } // optionalAttrs cfg.debug { DEBUG = "1"; }
              // optionalAttrs (cfg.dashboard.passwordFile != null) { API_PASSWORD_FILE = cfg.dashboard.passwordFile; }
              // cfg.extraEnv;
          in mapAttrsToList (name: value: "${name}=${value}") env;
        };
      };
    })


    (mkIf (cfg.enable || cfg.cli.enable) {
      environment.systemPackages = [
        zigduckPkgs.zigduck-cli
        zigduckPkgs.tv
        self.inputs.yo.packages.x86_64-linux.yo-rs
      ];
      
      environment.etc."zigduck/config.json".source = zigduckConfigFile;
      environment.etc."zigduck/devices.json".source = devices-json;
      environment.etc."zigduck/automations.json".source = automationsFile;
      environment.etc."zigduck/scenes.json".source = sceneConfig;
      environment.etc."zigduck/scenesCLI.json".source = sceneConfigCli;
      environment.etc."zigduck/dashboard.json".source = dashboardConfigFile;
      environment.etc."zigduck/dashboard-config.json".source = dashboardConfigFiles;
      environment.etc."zigduck/status-cards-config.json".source = statusCardsConfigJson;
      environment.etc."zigduck/tv-defaults.json".source = tvDefaultsJsonFile;
      
      systemd.tmpfiles.rules = [
        "d ${cfg.stateDir} 0755 zigduck zigduck - -"
        #"d ${cfg.stateDir}/timers 0755 zigduck zigduck - -"
        "f ${cfg.stateDir}/state.json 0644 zigduck zigduck - -"
        "d ${house.zigbee.dataDir} 0755 zigbee2mqtt zigbee2mqtt -"
       "L+ ${cfg.stateDir}/devices.json - - - - /etc/zigduck/devices.json"
       "L+ ${cfg.stateDir}/scenes.json - - - - /etc/zigduck/scenes.json"
       "L+ ${cfg.stateDir}/automations.json - - - - /etc/zigduck/automations.json"
      ];
      
      users.users.zigbee2mqtt = {
        isSystemUser = true;
        group = "zigbee2mqtt";
        home = house.zigbee.dataDir;
        createHome = true;
      }; 
      users.users.zigduck = {
        isSystemUser = true;
        group = "zigduck";
        home = cfg.stateDir;
        createHome = true;
      };

      users.groups.zigbee2mqtt = {};  
      users.groups.zigduck = { };
    })
    
  ];}
