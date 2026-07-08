# **Zigduck2mqttnix**

[![Sponsors](https://img.shields.io/github/sponsors/QuackHack-McBlindy?logo=githubsponsors&label=Sponsor&style=flat&labelColor=ff1493&logoColor=fff&color=rgba(234,74,170,0.5) "")](https://github.com/sponsors/QuackHack-McBlindy) [![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-Sponsor?style=flat&logo=buymeacoffee&logoColor=fff&labelColor=ff1493&color=ff1493)](https://buymeacoffee.com/quackhackmcblindy)



<br>

Declarative full-stack Zigbee home automation system that's reproducible and deployable.  
Nix for configuration, Rust for responsive async runtime.  
Under the hood: zigbee2mqtt, Mosquitto and adb.   
  
Define once, forget forever.   

 

```markdown
            Nix
             │
             ▼
        zigduck-rs
             │
      ┌──────┴──────┐
      ▼             ▼
    MQTT         REST API
      │             │
      ▼             ▼
 zigbee2mqtt    adb/media
      │             │
      └──────┬──────┘
             ▼
          Devices
```

 

<br> 
 
 
## **Installation**

<details><summary><strong>
❄️ Using flakes
</strong></summary>

Use `Zigduck2mqttnix`:  
  

#### **1: Add zigduck2mqttnix as an input in your flake.nix**

```nix
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    zigduck2mqttnix.url = "github:quackhack-mcblindy/zigduck2mqttnix";
  };
```


#### **2: Import the zigduck2mqttnix module into your configuration**  
  

```nix
  imports = [ zigduck2mqttnix.nixosModules.zigduck2mqttnix ];
```


#### **3: Enable the services**  

```nix
    environment.systemPackages = [ 
      self.inputs.zigduck2mqttnix.packages.x86_64-linux.zigduck-rs
      self.inputs.zigduck2mqttnix.packages.x86_64-linux.zigduck-cli
      self.inputs.zigduck2mqttnix.packages.x86_64-linux.zigduck-api
    ];
    services.zigduck = {
      enable = true;
      api.enable = true;
      api.port = 13335;
      api.passwordFile = config.sops.secrets.api.path;
      broker = "192.168.1.110";
      cli.broker = "192.168.1.110";      
      extraEnv.PATH = 
        "/run/current-system/sw/bin:"
        + "/optional/wrappers";
      };              
    };

};
```



</details>

<br>

## **Configuration**


<details><summary><strong>
Zigbee configuration
</strong></summary>

**Example configuration:**

```nix
  house = {
    zigbee = {
      # without this network key there is no reproducibility
      networkKeyFile = config.sops.secrets.z2m_network_key.path;
      mosquitto = {
        host = "192.168.1.110";
        username = "duckmqtt";
        passwordFile = config.sops.secrets.mosquitto.path;
      };
      coordinator = {
        vendorId =  "10c3";
        productId = "ea61";
        symlink = "zigbee"; # device symlink
      };
      # optional philips hue  bridge etc
      hueSyncBox = { 
        enable = true;
        bridge = { 
          ip = "192.168.1.33";
          # api token:
          # curl -X POST http://192.168.1.33/api -d '{"devicetype":"house#nixos"}'
          passwordFile = config.sops.secrets.hueBridgeAPI.path;
        }; 
        syncBox = {
          ip = "192.168.1.34";
          passwordFile = config.sops.secrets.hueBridgeAPI.path;
          tv = "shield";
        };
      }; 
    };
```     

<br>
</details>

<details><summary><strong>
Rooms
</strong></summary>

**Example configuration:**

```nix
  house = {
    rooms = {
      bedroom.icon    = "mdi:bed";
      hallway.icon    = "mdi:door";
      kitchen.icon    = "mdi:food-fork-drink";
      livingroom.icon = "mdi:sofa";
      wc.icon         = "mdi:toilet";
      tv-area.icon    = "mdi:television";
      other.icon      = "mdi:misc";
    };
```

</details>

<details><summary><strong>
Lights /  Devices 
</strong></summary>

**Example configuration:**  

```nix
  house = {
    zigbee = { 
      devices = {
        "0x0016830103ba7e95" = { # 64bit IEEE address (this is the unique device ID)  
          friendly_name = "Dimmer Switch Kitchen"; # simple human readable friendly name
          room = "kitchen"; # bind to group
          type = "dimmer"; # device type
          endpoint = 1; # zigbee endpoint
          batteryType = "CR2450"; # optional
        }; 
        "0x0017880402750848a" = { 
          friendly_name = "Spotlight kök 1";
          room = "kitchen";
          type = "light";
          endpoint = 11;
        };
      };
    };  
```
      
<br>

</details>




<details><summary><strong>
Dimmers /  Motion 
</strong></summary>

**Example configuraiton:**  

```nix    
    zigbee
      dimmer = {
        message = "action";
        doubleClickTimeout = 500;
        # optional as these defaults match most dimmers
        #actions = {
        #  onPress = "on_press_release";
        #  onHold = "on_hold_release";
        #};  
      };
      
      motion = {
        enable = true;
        trigger.lights = {
          after = 14;
          before = 9;
          duration = 900;
        };  
      };
```

<br>

</details>

<details><summary><strong>
Scenes
</strong></summary>


**Example configuraiton:**  

```nix
  house.zigbee = {
    scenes = {
      "Scene name" = {
        # device friendly_name
        "Spotlight kök 1" = {
          state = "ON";
          brightness = 200;
          color = { hex = "#00FF00"; };
        };
        # ... more lights
```

<br>

</details>


<details><summary><strong>
Automations
</strong></summary>

**Example configuraiton:**  

```nix
  house = {
    zigbee = {   
      # there are 6 different automation types    
      automations = {  
        # + a greeting automation
        greeting = {
          enable = true;
          awayDuration = 7200;
          delay = 10;
          actions = [
            {
              type = "shell";
              command = ''
                tts "greeting message" 
              '';
            }
          ];
        };
        

        # 1. MQTT triggered automations
        mqtt_triggered = {
          temperature = {
            enable = true;
            description = "Updating temperature data on dashboard";
            topic = "zigbee2mqtt/Motion Sensor Hall";
            actions = [{ type = "shell"; command = Mqtt2jsonHistory "temperature" "temperature.json"; }];
          };
          
        # 2. room action automations
        room_actions = {
          hallway = { 
            door_opened = [];
            door_closed = [];
          };
          
          kitchen = { 
            motion_not_detected = [
              {
                type = "shell";
                command = 
                '';
              }
              {
                type = "scene";
                scene = "kitchenFadeOff";
              }
            ];  

            motion_detected = [
              { 
                type = "scene";
                scene = "kitchenInstant";
              }
            ];
          };
        };
          
        # 3. global actions automations  
        global_actions = {
          leak_detected = [
            {
              type = "shell";
              command = "yo notify '🚨 WATER LEAK DETECTED!'";
            }
          ];
          smoke_detected = [
            {
              type = "shell";
              command = "yo notify '🔥 SMOKE DETECTED!'";
            }
          ];
        };

        # 4. [optional] dimmer actions automations (default configured per room)
        dimmer_actions = {          
          bedroom = {
            off_hold_release = {
              enable = true;
              description = "Turn off all configured light devices";
              extra_actions = [];
              override_actions = [
                {
                  type = "scene";
                  scene = "dark";
                }
                {
                  type = "mqtt";
                  topic = "zigbee2mqtt/Fläkt/set";
                  message = ''{"state":"OFF"}'';
                }
              ];
            };   
          };              
        };
        
        # 5. time based automations
        time_based = {};
        
        # 6. presence based automations
        presence_based = {};        
      };  

```

<br>

</details>


<details><summary><strong>
Media (optional)
</strong></summary>

**Example configuraiton:**  

```nix
  house = {
    media.root = "/Pool";
    media = {
      movies = "/Pool/Movies";
      tv = "/Pool/TV"; 
      music = "/Pool/Music"; 
      musicVideos = "/Pool/Music_Videos";
      otherVideos = "/Pool/Other_Videos"; 
      podcasts = "/Pool/Podcasts";
    };
  };
```

<br>

</details>

<details><summary><strong>
Commandline 
</strong></summary>

```
Usage: zigduck-cli [OPTIONS]

Options:
  -b, --broker <BROKER>
          MQTT broker host [env: MQTT_BROKER=] [default: 127.0.0.1]
  -u, --user <USER>
          MQTT username [env: MQTT_USER=] [default: mqtt]
      --password-file <PASSWORD_FILE>
          MQTT password file [env: MQTT_PASSWORD_FILE=]
      --password <PASSWORD>
          MQTT password [env: MQTT_PASSWORD=]
  -v, --verbose...
          Verbosity level
      --devices-config <DEVICES_CONFIG>
          Path to devices configuration [env: DEVICES_CONFIG=]
      --scenes-config <SCENES_CONFIG>
          Path to scenes configuration [env: SCENES_CONFIG=]
      --hue-bridge-ip <HUE_BRIDGE_IP>
          Hue Bridge IP [env: HUE_BRIDGE_IP=]
      --hue-api-key <HUE_API_KEY>
          Hue Bridge API key [env: HUE_API_KEY=]
      --hue-key-file <HUE_KEY_FILE>
          Hue Bridge API key file [env: HUE_KEY_FILE=]
      --device <DEVICE>
          Device name (friendly name)
      --room <ROOM>
          Room name
      --scene <SCENE>
          Scene name
      --list [<LIST>]
          List devices, rooms, scenes, lights, or sensors [possible values: devices, rooms, scenes, lights, sensors]
      --pair [<PAIR>]
          Pairing duration in seconds (default: 120)
      --all-lights [<ALL_LIGHTS>]
          Control all lights (optional true/false)
      --cheap-mode <CHEAP_MODE>
          Room name for cheap mode
      --json-cmd
          Send raw JSON to a device
      --state <STATE>
          Device state: on/off/toggle/max/dark
      --brightness <BRIGHTNESS>
          Brightness percentage (1-100)
      --color <COLOR>
          Color name or hex code
      --temperature <TEMPERATURE>
          Color temperature (153-500)
      --transition <TRANSITION>
          Transition time in seconds
      --payload <PAYLOAD>
          Raw JSON payload
      --backend <BACKEND>
          Backend type (auto/zigbee/hue) [default: auto] [possible values: auto, zigbee, hue]
      --json-output
          Output list as JSON
      --watch
          Watch for new devices during pairing
      --random
          Pick a random scene
      --scene-room <SCENE_ROOM>
          Restrict scene to a specific room
      --delay <DELAY>
          Delay in seconds for cheap mode [default: 300]
  -h, --help
          Print help (see more with '--help')
  -V, --version
          Print version

```

<br>

</details>


<details><summary><strong>
API
</strong></summary>


**Endpoints:**  


| Endpoint | Method | Description | Parameters | Auth Required |
|----------|--------|-------------|------------|---------------|
| `/` | GET | Service information and list of available endpoints | None | Yes |
| `/transcode-video` | GET | Streams transcoded video from a given URL (MP4 with chunked transfer) | `url` (URL to transcode) | Yes |
| `/browse` | GET | Browse media directory using `ls` (legacy) | `path` (directory path relative to `/Pool`) | Yes |
| `/browsev2` | GET | Browse media directory using `find` with extended info | `path` (directory path relative to `/Pool`) | Yes |
| `/add` | GET | Add a file to the VLC playlist | `path` (file path) | Yes |
| `/add_folder` | GET | Add a folder (recursively) to the VLC playlist | `path` (folder path) | Yes |
| `/timers` | GET | List all timers | None | Yes |
| `/alarms` | GET | List all alarms | None | Yes |
| `/shopping` (or `/shopping-list`) | GET | List shopping items | None | Yes |
| `/reminders` (or `/remmind`) | GET | List reminders | None | Yes |
| `/media/power/on` | GET | Wake up media device via ADB | `device` (IP, default `192.168.1.224`) | Yes |
| `/media/power/off` | GET | Put media device to sleep via ADB | `device` (IP, default `192.168.1.224`) | Yes |
| `/media/next` | GET | Send "next track" command via ADB | `device` (IP, default `192.168.1.224`) | Yes |
| `/media/previous` | GET | Send "previous track" command via ADB | `device` (IP, default `192.168.1.224`) | Yes |
| `/media/play` or `/media/pause` | GET | Toggle play/pause via ADB | `device` (IP, default `192.168.1.224`) | Yes |
| `/media/volume/up` | GET | Increase volume via ADB | `device` (IP, default `192.168.1.224`) | Yes |
| `/media/volume/down` | GET | Decrease volume via ADB | `device` (IP, default `192.168.1.224`) | Yes |
| `/media/playlist` | GET | Start a playlist on media device (ADB intent) | `device` (IP), `url` (optional, defaults to webserver URL) | Yes |
| `/playlist` | GET | Get current VLC playlist (JSON) | None | Yes |
| `/playlist/remove` | GET | Remove an item from the VLC playlist by index | `index` (zero‑based) | Yes |
| `/playlist/clear` | GET | Clear the entire VLC playlist | None | Yes |
| `/playlist/shuffle` | GET | Shuffle the current VLC playlist | None | Yes |
| `/health` | GET | Basic health check (no authentication) | None | No |
| `/health/all` | GET | Aggregate health data from all services (no authentication) | None | No |
| `/state` | GET | Full state of all Zigbee devices (from `/var/lib/zigduck/state.json`) | None | Yes |
| `/state/{device}` | GET | State of a specific device | `{device}` (device name) | Yes |
| `/state/room/{room}` | GET | State of all devices in a given room | `{room}` (room name) | Yes |
| `/device/list` | GET | List all devices (from `devices.json`) | None | Yes |
| `/device/{device}/{command}/{value}...` | GET | Control a device with one or more commands (e.g., `state/on`, `brightness/200`, `color/%23FF5733`, `temperature/300`). Multiple commands can be chained. | `{device}` (name), `{command}` (action), `{value}` (argument) | Yes |
| `/scene/{scene}` | GET | Activate a scene by name (from `scenes.json`) | `{scene}` (scene name) | Yes |
| `/device/rooms` | GET | List devices grouped by room (from `rooms.json`) | None | Yes |
| `/device/types` | GET | List devices grouped by type (from `types.json`) | None | Yes |
| `/tts` | GET | Text‑to‑speech, returns an audio/wav file | `text` (text to speak) | Yes |
| `/do` | GET | Execute a natural language command (e.g., `?cmd=do turn on light`) | `cmd` (command string) | Yes |
| `/upload` | POST | Upload a file (multipart/form‑data) to `/var/lib/zigduck/uploads` | File data in body | Yes |

<br>

</details>

<br>


<details><summary><strong>
Inspiration?
</strong></summary>

<br>

for a full configuration example view:  
*[my home](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/modules/myHouse.nix)*

<br>

</details>

<br>

## **License**

This project is licensed under the terms of the MIT license.  
See the `LICENSE` file in the repository for full details.

Contributions are welcomed.


