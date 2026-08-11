{
  config,
  lib,
  pkgs,
  ...
} : let
  inherit (lib) types mkOption mkEnableOption mkMerge;  
  Types = import ./types.nix { inherit lib; };
  format = pkgs.formats.yaml { };
  configFile = format.generate "zigbee2mqtt.yaml" config.house.zigbee.settings;

  zigbeeDevices = config.house.zigbee.devices;
 
  scenes = config.house.zigbee.scenes;

  jsonFormat = pkgs.formats.json { };

in {
    imports = [ ./zigduck.nix ./assertions.nix ./helpers.nix ];
    
    options.house = {
      https = {
        urlFile = lib.mkOption {
          type = lib.types.path;
          description = ''
            File containing full https url.
            This should be served as webserver. (TLS req?)
            Example: "https://my-domain.com"
            If you don't have a domain, you can use https://www.duckdns.org/ for free.
          '';
          default = "";
        };
      };  
      # set media root & the rest is overrides
      media = with lib; {
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
          
      # dashboard configuraiton
      dashboard = {
        passwordFile = lib.mkOption {
          type = lib.types.path;
          description = "Passwordfile for the dashboard API";
          default = "";
        };
      
        pages = lib.mkOption {
          type = lib.types.attrsOf (lib.types.submodule {
            options = {
              icon = lib.mkOption {
                type = lib.types.str;
                description = "Icon for the tab (FontAwesome class, MDI class, or image URL)";
                default = "fas fa-question";
              };
              code = lib.mkOption {
                type = lib.types.str;
                description = "HTML and JavaScript code for the page";
                default = "";
              };
              title = lib.mkOption {
                type = lib.types.str;
                description = "Title for the page (optional)";
                default = "";
              };
              files = lib.mkOption {
                type = lib.types.attrsOf (lib.types.oneOf [lib.types.path lib.types.str]);
                default = {};
                description = "Files to be symlinked to the http server for this page";
              };
              css = lib.mkOption {
                type = lib.types.str;
                default = "";
                description = "Additional CSS for this page";
              };              
            };
          });
          default = {};
          description = "Custom pages for the dashboard";
        };
        
        statusCards = lib.mkOption {
          type = lib.types.attrsOf Types.statusCardType;
          default = {};
          description = "Configurable status cards for the dashboard";
        };
      };
      

      rooms = mkOption {
        type = types.attrsOf Types.roomType;
        default = {
          bedroom.icon = "mdi:bedroom";
          hallway.icon = "mdi:hallway";
          kitchen.icon = "mdi:sofa";
          livingroom.icon = "mdi:toilet";
          wc.icon = "mdi:toilet";
        };
        description = "A set of rooms in the house with their attributes.";
      };
    
      tv = lib.mkOption {
        type = lib.types.attrsOf (lib.types.submodule {
          options = {
            enable = lib.mkEnableOption "Enable this Android TVOS device";
            room = lib.mkOption {
              type = lib.types.strMatching (lib.concatStringsSep "|" (lib.attrNames config.house.rooms));
              description = "Room where TV is located";
            };
            ip = lib.mkOption {
              type = lib.types.str;
              description = "TV's static IP address";
            };

            apps = lib.mkOption {
              type = lib.types.attrsOf lib.types.str;
              default = {};
              description = "App package names and activities for this TV device";
              example = {
                telenor = "se.telenor.stream/.MainActivity";
                tv4 = "se.tv4.tv4playtab/se.tv4.tv4play.ui.mobile.main.BottomNavigationActivity";
              };
            };
            
            # TV channel definitions
            channels = lib.mkOption {
              type = lib.types.attrsOf (lib.types.submodule {
                options = {
                  name = lib.mkOption {
                    type = lib.types.str;
                    description = "Channel display name";
                  };
                  icon = lib.mkOption {
                    type = types.nullOr types.path;
                    description = "Optional file path for channel icon used for the generated TV-guide web frontend";
                    default = null;
                  };                  
                  id = lib.mkOption {
                    type = lib.types.nullOr lib.types.int;
                    default = null;
                    description = "Channel ID number, when set will send defined value as ADB channel command.";
                  };
                  cmd = lib.mkOption {
                    type = lib.types.str;
                    description = "Sequence of ADB commands to launch channel. Seperated with && (Overrides ID)";
                    default = "";
                  };     
                  stream_url = lib.mkOption {
                    type = lib.types.str;
                    description = "Stream URL to send to device. (Overrides ID)";
                    default = "";
                  };
                  scrape_url = lib.mkOption {
                    type = lib.types.str;
                    description = ''
                      URL from which to scrape TV guide data for this channel.
                      Only used as reference for user provided external script.
                    '';
                    default = "";
                  };      
                };
              });
              description = "TV channel options";
            };
          };
        });
        default = {};
      };
      
      
      zigbee = {      
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
            Your {file}`configuration.yaml` as a Nix attribute set.
            Check the [documentation](https://www.zigbee2mqtt.io/information/configuration.html)
            for possible options.
          '';
        };

      
        # Zigbee network key      
        networkKeyFile = mkOption {
          type = types.path;
          description = ''
            Path to a file containing the Zigbee network key (Zigbee2MQTT's `network_key`).
            Copy your existing one from your Zigbee2mqtt configuraiton.yaml file.
            Having this key saved avoids user having to repair his/her devices.
            Example content:
            ```
            - 86
            - 208
            ...
            ...
            ```
          '';
        };
      };

      zigbee.mosquitto = mkOption {
        type = types.nullOr (types.submodule {
          options = {
            host = mkOption {
              type = types.nullOr types.str;
              default = null;
              description = "IP address of the host running Mosquitto";
              example = "192.168.1.211";
            };  
            username = mkOption {
              type = types.nullOr types.str;
              default = null;
              description = "MQTT username for authentication";
            };  
            passwordFile = mkOption {
              type = types.nullOr types.path;
              default = null;
              description = "Path to file containing MQTT password";
            };
            baseTopic = mkOption {
              type = types.str;
              default = "zigbee2mqtt";
              description = "MQTT base topic used by Zigbee2mqtt and the zigduck runtime.";
            };

            ssl = {
              enable = mkEnableOption "Enable SSL/TLS for MQTT connection";    
              caCertFile = mkOption {
                type = types.nullOr types.path;
                default = null;
                description = "Path to CA certificate file";
              };    
              clientCertFile = mkOption {
                type = types.nullOr types.path;
                default = null;
                description = "Path to client certificate file";
              };    
              clientKeyFile = mkOption {
                type = types.nullOr types.path;
                default = null;
                description = "Path to client private key file";
              };
            };
          };
        });
      };

      
      zigbee.coordinator = mkOption {
        type = types.nullOr (types.submodule {
          options = {
            vendorId = mkOption {
              type = types.str;
              description = "USB vendor ID (hex format)";
            };
            productId = mkOption {
              type = types.str;
              description = "USB product ID (hex format)";
            };
            symlink = mkOption {
              type = types.str;
              description = "Symlink name to create in /dev";
            };
            adapter = mkOption {
              type = types.str;
              description = "Adapter type for the coordinator, required for Zigbee2MQTT version 2.x and above. If you don't kknow leave blank for default.";
              default = "zstack";
            };
          };
        });
        default = {};
        description = "Serial port device mapping by USB IDs";
      };

      # philips hue play hdmi sync box - sync lights with tv
      zigbee.hueSyncBox = mkOption {
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
      };

      # dimmer coniguration      
      zigbee.dimmer = lib.mkOption {
        type = types.submodule {
          options = {
            # which mqtt field contains the action
            message = lib.mkOption {
              type = lib.types.str;
              default = "action";
              description = "MQTT field name containing dimmer action";
            };
            # action mappings
            actions = {
              onPress = lib.mkOption {
                type = lib.types.str;
                default = "on_press_release";
                description = "Action for single press of ON button";
              };
              onHold = lib.mkOption {
                type = lib.types.str;
                default = "on_hold_release";
                description = "Action for holding ON button";
              };
              offPress = lib.mkOption {
                type = lib.types.str;
                default = "off_press_release";
                description = "Action for single press of OFF button";
              };
              offHold = lib.mkOption {
                type = lib.types.str;
                default = "off_hold_release";
                description = "Action for holding OFF button";
              };
              upPress = lib.mkOption {
                type = lib.types.str;
                default = "up_press_release";
                description = "Action for single press of UP button";
              };
              upHold = lib.mkOption {
                type = lib.types.str;
                default = "up_hold_release";
                description = "Action for holding UP button";
              };
              downPress = lib.mkOption {
                type = lib.types.str;
                default = "down_press_release";
                description = "Action for single press of DOWN button";
              };
              downHold = lib.mkOption {
                type = lib.types.str;
                default = "down_hold_release";
                description = "Action for holding DOWN button";
              };
            };
            doubleClickTimeout = mkOption {
              type = types.nullOr types.int;
              default = null;
              description = "Timeout for double‑click detection in milliseconds (defaults to 300).";
            };
          };  
        };    
        default = {
          message = "action";
          actions = {
            onPress = "on_press_release";
            onHold = "on_hold_release";
            offPress = "off_press_release";
            offHold = "off_hold_release";
            upPress = "up_press_release";
            upHold = "up_hold_release";
            downPress = "down_press_release";
            downHold = "down_hold_release";
          };
        };
        description = "Configuration for dimmer switches. Default configuration is for the Philips Hue Dimmer Switch. You can check the message for your specific dimmer at zigbee2MQTT documentation.";
      };


      # zigbee devices configuration
      zigbee.devices = lib.mkOption {
        type = lib.types.attrsOf (lib.types.submodule {
          options = {
            friendly_name = mkOption {
              type = types.str;
              description = "A human-readable device name.";
              example = "Kitchen Dimmer";
            };
            room = lib.mkOption { 
              type = lib.types.strMatching (lib.concatStringsSep "|" (lib.attrNames config.house.rooms));
              description = "The room this device belongs to.";
              example = "kitchen";
            };
            type = lib.mkOption { 
              type = lib.types.enum [ "light" "hue_light" "dimmer" "sensor" "motion" "outlet" "remote" "pusher" "blind" ];
              description = "The type of device (e.g., light, dimmer, sensor, motion, outlet, remote, pusher, blind, hue_light).";
              example = "light";
            };
            icon = lib.mkOption { 
              type = lib.types.str;
              description = "Material Design icon name representing this device.";
              default = "mdi:monitor-shimmer";
              example = "mdi:cancel";
            };
            
            batteryType = mkOption {
              type = types.nullOr (types.enum ["CR2032" "CR2450" "CR02" "AAA" "AA"]);
              default = null;
              description = "Optional type of battery the device uses, if applicable.";
              example = "CR2032";
            };
            
            supports_color = mkOption {
              type = types.bool;
              default = false;
              description = "Whether the light device supports setting color.";
              example = true;
            };
            
            supports_temperature = mkOption {
              type = types.bool;
              default = false;
              description = "Whether the light device supports setting temperature.";
              example = true;
            };
            
            endpoint = lib.mkOption { 
              type = lib.types.int;
              description = "The Zigbee endpoint to control this device.";
              example = 11;
            };
            
            hue_id = lib.mkOption { 
              type = types.nullOr types.int;
              description = "The light_id for the device. Integrates Philips Hue paired devices. Configuring this option will NOT insert the device into the Zigbee2MQTT configuration file.";
              example = 11;
              default = null;
            };            
          };
        });
        default = {};
          description = "Zigbee device definitions keyed by device ID.";
          example = {
            "0x0017880103ca6e95" = {
              friendly_name = "Kitchen Dimmer";
              room = "kitchen";
              type = "dimmer";
              icon = "mdi-toggle-switch";
              endpoint = 1;
              batteryType = "CR3032";
              supports_color = false;
            };
          };    
        };
        
        zigbee.scenes = lib.mkOption {
          type = lib.types.attrsOf (lib.types.attrsOf (lib.types.attrs));
          default = {};
          description = "Scenes for Zigbee devices";
        };
 
        zigbee.motion = lib.mkOption {
          type = lib.types.submodule {
            options = {
              enable = lib.mkEnableOption "Enable Zigbee motion handling" // {
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
        }; 
 
        # automations configuration
        zigbee.automations = mkOption {
          type = types.submodule {
            options = {                   
              # mqtt triggered automations
              mqtt_triggered = mkOption {
                type = types.attrsOf (types.submodule {
                  options = {
                    enable = mkEnableOption "Enable this MQTT-triggered automation";
                    description = mkOption {
                      type = types.str;
                      description = "Description of what this automation does";
                    };
                    topic = mkOption {
                      type = types.str;
                      description = "MQTT topic to subscribe to";
                      example = "zigbee2mqtt/button/action";
                    };
                    message = mkOption {
                      type = types.nullOr types.str;
                      default = null;
                      description = "Specific message value to match (if any)";
                      example = "single";
                    };
                    conditions = mkOption {
                      type = types.listOf (types.submodule {
                        options = {
                          type = mkOption {
                            type = types.enum ["dark_time" "someone_home" "room_occupied"];
                            description = "Condition type";
                          };
                          room = mkOption {
                            type = types.nullOr types.str;
                            default = null;
                            description = "Room for room-specific conditions";
                          };
                          value = mkOption {
                            type = types.nullOr types.bool;
                            default = null;
                            description = "Expected condition value";
                          };
                        };
                      });
                      default = [];
                      description = "Conditions that must be met";
                    };
                    actions = mkOption {
                      type = types.listOf Types.automationActionType;
                      default = [];
                      description = "Actions to perform when MQTT message is received";
                    };
                  };
                });
                default = {};
                description = "MQTT-triggered automations";
                example = {
                  button_single_press = {
                    enable = true;
                    description = "Toggle living room lights on button single press";
                    topic = "zigbee2mqtt/living_room_button/action";
                    message = "single";
                    actions = [
                      {
                        type = "mqtt";
                        topic = "zigbee2mqtt/living_room_lights/set";
                        message = ''{"state":"TOGGLE"}'';
                      }
                    ];
                  };
                  motion_alert = {
                    enable = true;
                    description = "Send notification on motion detected";
                    topic = "zigbee2mqtt/outdoor_motion/occupancy";
                    message = "true";
                    actions = [
                      "echo 'Motion detected outside!' | wall"
                      {
                        type = "shell";
                        command = "${pkgs.libnotify}/bin/notify-send 'Security' 'Motion detected outside'";
                      }
                    ];
                  };
                };
              };
                       
              # time based automations
              time_based = mkOption {
                type = types.attrsOf (types.submodule {
                  options = {
                    enable = mkEnableOption "Enable this time-based automation";
                    description = mkOption {
                      type = types.str;
                      description = "Description of what this automation does";
                    };
                    schedule = mkOption {
                      type = types.oneOf [
                        (types.submodule {
                          options = {
                            start = mkOption {
                              type = types.nullOr types.str;
                              default = null;
                              description = "Start time (HH:MM)";
                            };
                            end = mkOption {
                              type = types.nullOr types.str;
                              default = null;
                              description = "End time (HH:MM)";
                            };
                            days = mkOption {
                              type = types.listOf (types.enum ["mon" "tue" "wed" "thu" "fri" "sat" "sun"]);
                              default = ["mon" "tue" "wed" "thu" "fri" "sat" "sun"];
                              description = "Days of week to run";
                            };
                          };
                        })
                      ];
                      description = "Schedule configuration";
                    };
                    conditions = mkOption {
                      type = types.listOf (types.submodule {
                        options = {
                          type = mkOption {
                            type = types.enum ["dark_time" "someone_home" "room_occupied"];
                            description = "Condition type";
                          };
                          room = mkOption {
                            type = types.nullOr types.str;
                            default = null;
                            description = "Room for room-specific conditions";
                          };
                          value = mkOption {
                            type = types.nullOr types.bool;
                            default = null;
                            description = "Expected condition value";
                          };
                        };
                      });
                      default = [];
                      description = "Conditions that must be met";
                    };
                    actions = mkOption {
                      type = types.listOf Types.automationActionType;
                      default = [];
                      description = "Actions to perform";
                    };
                  };
                });
                default = {};
                description = "Time-based automations";
                example = {
                  morning_wakeup = {
                    enable = true;
                    description = "Gentle morning lights";
                    schedule = {
                      start = "06:30";
                      end = "07:00";
                      days = ["mon" "tue" "wed" "thu" "fri"];
                    };
                    conditions = [
                      { type = "someone_home"; value = true; }
                    ];
                    actions = [
                      {
                        type = "scene";
                        scene = "morning";
                      }
                      "echo 'Good morning!'"
                    ];
                  };
                  bedtime = {
                    enable = true;
                    description = "Prepare for bed";
                    schedule = "0 22 * * *";
                    actions = [
                      {
                        type = "scene";
                        scene = "night";
                      }
                    ];
                  };
                };
              };
        
              # presence based automations
              presence_based = mkOption {
                type = types.attrsOf (types.submodule {
                  options = {
                    enable = mkEnableOption "Enable this presence-based automation";
                    description = mkOption {
                      type = types.str;
                      description = "Description of what this automation does";
                    };
                    motion_sensors = mkOption {
                      type = types.listOf types.str;
                      description = "List of motion sensor friendly names to monitor";
                      example = ["Hallway Motion" "Living Room Motion"];
                    };
                    no_motion_duration = mkOption {
                      type = types.int;
                      default = 300;
                      description = "Seconds without motion before triggering";
                    };
        
                    conditions = mkOption {
                      type = types.listOf (types.submodule {
                        options = {
                          type = mkOption {
                            type = types.enum ["dark_time" "room_occupied" "lights_on"];
                            description = "Condition type";
                          };
                          room = mkOption {
                            type = types.nullOr types.str;
                            default = null;
                            description = "Room for room-specific conditions";
                          };
                          value = mkOption {
                            type = types.nullOr types.bool;
                            default = null;
                            description = "Expected condition value";
                          };
                        };
                      });
                      default = [];
                      description = "Conditions that must be met";
                    };
                    actions = mkOption {
                      type = types.listOf Types.automationActionType;
                      default = [];
                      description = "Actions to perform when no motion is detected";
                    };
                    motion_restored_actions = mkOption {
                      type = types.listOf Types.automationActionType;
                      default = [];
                      description = "Actions to perform when motion is detected again";
                    };
                  };
                });
                default = {};
                description = "Presence/motion-based automations";
              };
  
              # welcome home automation
              greeting = mkOption {
                type = types.submodule {
                  options = {
                    enable = mkEnableOption "Enable greeting automation";
                    awayDuration = mkOption {
                      type = types.int;
                      default = 7200;
                      description = "Time in seconds to be concidered away from home (default 7200)";
                    };                    
                    delay = mkOption {
                      type = types.int;
                      default = 10;
                      description = "Delay in seconds before triggering greeting";
                    };
                    actions = mkOption {
                      type = types.listOf Types.automationActionType;
                      default = "echo 'Welcome home!'";
                      description = "Action to perform for greeting";
                    };
                  };
                };
                default = {};
                description = "Greeting automation configuration";
              };
            
              # per-room dimmer switch actions
              dimmer_actions = mkOption {
                type = types.attrsOf (types.submodule {
                  options = {
                    on_press_release = mkOption {
                      type = types.nullOr Types.dimmerActionType;
                      default = null;
                      description = "Action for on button press and release";
                    };
                    on_hold_release = mkOption {
                      type = types.nullOr Types.dimmerActionType;
                      default = null;
                      description = "Action for on button hold and release";
                    };
                    off_press_release = mkOption {
                      type = types.nullOr Types.dimmerActionType;
                      default = null;
                      description = "Action for off button press and release";
                    };
                    off_hold_release = mkOption {
                      type = types.nullOr Types.dimmerActionType;
                      default = null;
                      description = "Action for off button hold and release";
                    };
                    up_press_release = mkOption {
                      type = types.nullOr Types.dimmerActionType;
                      default = null;
                      description = "Action for up button press and release";
                    };
                    up_hold_release = mkOption {
                      type = types.nullOr Types.dimmerActionType;
                      default = null;
                      description = "Action for up button hold and release";
                    };
                    down_press_release = mkOption {
                      type = types.nullOr Types.dimmerActionType;
                      default = null;
                      description = "Action for down button press and release";
                    };
                    down_hold_release = mkOption {
                      type = types.nullOr Types.dimmerActionType;
                      default = null;
                      description = "Action for down button hold and release";
                    };
                  };
                });
                default = {};
                description = "Per-room configuration for dimmer switch actions";
                example = {
                  kitchen = {
                    on_press_release = {
                      enable = true;
                      description = "Turn on kitchen lights and fan";
                      extra_actions = [
                        {
                          type = "mqtt";
                          topic = "zigbee2mqtt/Fläkt/set";
                          message = ''{"state":"ON"}'';
                        }
                      ];
                    };
                    off_press_release = {
                      enable = true;
                      description = "Turn off kitchen lights only";
                    };
                  };
                  _default = {
                    on_press_release = {
                      enable = true;
                      description = "Default: turn on room lights";
                    };
                    on_hold_release = {
                      enable = true;
                      description = "Default: turn on all lights at maximum brightness";
                    };
                    up_press_release = {
                      enable = true;
                      description = "Default: dim up room lights";
                    };
                    up_hold_release = {
                      enable = true;
                      description = "Default: no default actions";
                    };
                    down_press_release = {
                      enable = true;
                      description = "Default: dim down room lights";
                    };
                    down_hold_release = {
                      enable = true;
                      description = "Default: no default actions";
                    };   
                    off_press_release = {
                      enable = true;
                      description = "Default: turn off room lights";
                    };                      
                    off_hold_release = {
                      enable = true;
                      description = "Default: turn off all lights";
                    };                    
                  };
                };
              };
        
              # room-specific automations
              room_actions = mkOption {
                type = types.attrsOf (types.attrsOf (types.listOf Types.automationActionType));
                default = {};
                description = "Room-specific automation actions";
                example = {
                  kitchen = {
                    motion_detected = [
                      "echo 'Motion in kitchen'"
                      {
                        type = "mqtt";
                        topic = "zigbee2mqtt/Fläkt/set";
                        message = ''{"state":"ON"}'';
                      }
                    ];
                    lights_turned_on = [
                      "echo 'Kitchen lights activated'"
                    ];
                  };
                };
              };

              # global automations
              global_actions = mkOption {
                type = types.attrsOf (types.listOf Types.automationActionType);
                default = {};
                description = "Global automation actions not tied to specific rooms";
                example = {
                  all_lights_on = [
                    "echo 'All lights turned on'"
                    {
                      type = "scene";
                      scene = "max";
                    }
                  ];
                  security_armed = [
                    "echo 'Security system armed'"
                    {
                      type = "mqtt";
                      topic = "zigbee2mqtt/security/state";
                      message = ''{"armed":true}'';
                    }
                  ];
                };
              };
            };
          };
          default = {};
          description = "Modular automation configurations";
        };
      };
  
    }
