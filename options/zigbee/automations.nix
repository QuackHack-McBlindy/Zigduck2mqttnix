{
  config,
  lib,
  pkgs,
  ...
} : let
  inherit (lib) mkOption types mkEnableOption;
  Types = import ./../types.nix { inherit lib; };
in {
  options.house.zigbee.automations = mkOption {
    type = types.submodule {
      options = {
        # MQTT triggered automations
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

        # Time based automations
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
                  types.str
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
                description = "Schedule configuration (cron string or structured)";
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

        # Presence based automations
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

        # Welcome home automation
        greeting = mkOption {
          type = types.submodule {
            options = {
              enable = mkEnableOption "Enable greeting automation";
              awayDuration = mkOption {
                type = types.int;
                default = 7200;
                description = "Time in seconds to be considered away from home (default 7200)";
              };
              delay = mkOption {
                type = types.int;
                default = 10;
                description = "Delay in seconds before triggering greeting";
              };
              actions = mkOption {
                type = types.listOf Types.automationActionType;
                default = [ "echo 'Welcome home!'" ];
                description = "Action to perform for greeting";
              };
            };
          };
          default = {};
          description = "Greeting automation configuration";
        };

        # Per-room dimmer switch actions
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

        # Room-specific automations
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

        # Global automations
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

  };}
