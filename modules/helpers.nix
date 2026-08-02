{
  config,
  lib,
  pkgs,
  ...
} : let

  zigbeeDevices = config.house.zigbee.devices;
  scenes = config.house.zigbee.scenes;


  deviceMeta = builtins.toJSON (
    lib.listToAttrs (
      lib.filter (attr: attr.name != null) (
        lib.mapAttrsToList (_: dev: {
          name = dev.friendly_name;
          value = {
            room = dev.room;
            type = dev.type;
            id = dev.friendly_name;
            endpoint = dev.endpoint;
          };
        }) zigbeeDevices
      )
    )
  );


  normalizedDeviceMap = lib.mapAttrs' (id: device:
    lib.nameValuePair (lib.toLower device.friendly_name) device.friendly_name
  ) zigbeeDevices;

  deviceList = builtins.attrNames normalizedDeviceMap;

  makeCommand = device: settings:
    let
      json = builtins.toJSON settings;
    in
      ''
      yo mqtt_pub --topic "zigbee2mqtt/${device}/set" .-message '${json}'
      '';

  sceneCommands = lib.mapAttrs
    (sceneName: sceneDevices:
      lib.mapAttrs (device: settings: makeCommand device settings) sceneDevices
    ) scenes;  


  colorToHex = color:
    if color ? hex then color.hex
    else if color ? xy then
      let
        x = lib.elemAt color.xy 0;
        y = lib.elemAt color.xy 1;
        # xy > rgb approx (for sRGB gamut)
        r = lib.clamp 0 255 (lib.toInt ((3.2406 * x - 1.5372 * y - 0.4986 * (1 - x - y)) * 255));
        g = lib.clamp 0 255 (lib.toInt ((-0.9689 * x + 1.8758 * y + 0.0415 * (1 - x - y)) * 255));
        b = lib.clamp 0 255 (lib.toInt ((0.0557 * x - 0.2040 * y + 1.0570 * (1 - x - y)) * 255));
      in
      "#" + 
      (lib.fixedWidthString 2 "0" (lib.toHexString r)) +
      (lib.fixedWidthString 2 "0" (lib.toHexString g)) +
      (lib.fixedWidthString 2 "0" (lib.toHexString b))
    else if color ? hue && color ? saturation then
      let
        # convert hue (0-65535) and sat (0-254) to degrees/%
        hue_deg = (color.hue * 360) / 65535;
        sat_pct = color.saturation / 254.0;
        # hsv > rgb conversion
        c = sat_pct;
        h_prime = hue_deg / 60.0;
        x = c * (1 - lib.abs((builtins.mod h_prime 2) - 1));
        m = 1 - c;
      
        # rgb based on hue sector
        rgb1 = 
          if h_prime < 1 then [c x 0]
          else if h_prime < 2 then [x c 0]
          else if h_prime < 3 then [0 c x]
          else if h_prime < 4 then [0 x c]
          else if h_prime < 5 then [x 0 c]
          else [c 0 x];
      
        r1 = lib.elemAt rgb1 0;
        g1 = lib.elemAt rgb1 1;
        b1 = lib.elemAt rgb1 2;
      
        r = lib.clamp 0 255 (lib.toInt ((r1 + m) * 255));
        g = lib.clamp 0 255 (lib.toInt ((g1 + m) * 255));
        b = lib.clamp 0 255 (lib.toInt ((b1 + m) * 255));
      in
      "#" + 
      (lib.fixedWidthString 2 "0" (lib.toHexString r)) +
      (lib.fixedWidthString 2 "0" (lib.toHexString g)) +
      (lib.fixedWidthString 2 "0" (lib.toHexString b))
    else "#ffffff";

  sceneLight = {state, brightness ? null, hex ? null, temp ? null, hue ? null, sat ? null, xy ? null, ct ? null, effect ? "none", alert ? "none", transition ? null}:
    let
      colorValue = if hex != null then { inherit hex; } 
        else if xy != null then { inherit xy; }
        else if hue != null && sat != null then { inherit hue sat; }
        else if ct != null then { inherit ct; }
        else if temp != null then { ct = temp; }
        else null;
    in
    {
      inherit state;
    } // (if brightness != null then { inherit brightness; } else {})
      // (if colorValue != null then { color = colorValue; } else {})
      // (if effect != null && effect != "none" then { inherit effect; } else {})
      // (if alert != null && alert != "none" then { inherit alert; } else {})
      // (if transition != null then { inherit transition; } else {});



  cmdHelpers = ''
    say_duck() {
      echo -e "\e[3m\e[38;2;0;150;150m🦆 duck say \e[1m\e[38;2;255;255;0m⮞\e[0m\e[3m\e[38;2;0;150;150m $1\e[0m"
    }  

    mqtt_pub() {
      ${pkgs.mosquitto}/bin/mosquitto_pub -h "$MQTT_BROKER" -u "$MQTT_USER" -P "$MQTT_PASSWORD" "$@"
    }

    color2hex() {
      local color="$1"
      declare -A color_ranges=(
        ["red"]="255,0,0:165,0,0"
        ["green"]="0,255,0:0,100,0"
        ["blue"]="0,0,255:0,0,165"
        ["yellow"]="255,255,0:200,200,0"
        ["orange"]="255,165,0:205,100,0"
        ["purple"]="128,0,128:80,0,80"
        ["pink"]="255,192,203:220,150,160"
        ["white"]="255,255,255:240,240,240"
        ["black"]="10,10,10:0,0,0"
        ["gray"]="160,160,160:80,80,80"
        ["brown"]="165,42,42:120,30,30"
        ["cyan"]="0,255,255:0,200,200"
        ["magenta"]="255,0,255:180,0,180"
      )
      local r g b
      if [[ -z "$color" || "$color" == "random" || -z "''${color_ranges[''$color]}" ]]; then
        r=$(( RANDOM % 256 ))
        g=$(( RANDOM % 256 ))
        b=$(( RANDOM % 256 ))
      else
        IFS=':' read -r min_range max_range <<< "''${color_ranges[$color]}"
        IFS=',' read -r min_r min_g min_b <<< "$min_range"
        IFS=',' read -r max_r max_g max_b <<< "$max_range"
        r=$(( min_r + RANDOM % (max_r - min_r + 1) ))
        g=$(( min_g + RANDOM % (max_g - min_g + 1) ))
        b=$(( min_b + RANDOM % (max_b - min_b + 1) ))
      fi
      printf "%02x%02x%02x\n" "$r" "$g" "$b"
    }
    
    color2xy() {
      local color="$1"
      declare -A color_ranges=(
        ["red"]="0.675,0.322:0.692,0.308"
        ["green"]="0.17,0.7:0.214,0.709"
        ["blue"]="0.14,0.08:0.153,0.048"
        ["yellow"]="0.452,0.47:0.507,0.472"
        ["orange"]="0.6,0.38:0.62,0.37"
        ["purple"]="0.28,0.13:0.265,0.11"
        ["pink"]="0.35,0.28:0.38,0.3"
        ["white"]="0.3227,0.329:0.313,0.337"
        ["black"]="0.15,0.08:0.12,0.06"
        ["gray"]="0.3227,0.329:0.3,0.31"
        ["brown"]="0.6,0.34:0.62,0.32"
        ["cyan"]="0.16,0.23:0.18,0.25"
        ["magenta"]="0.38,0.18:0.4,0.2"
        ["coolwhite"]="0.31,0.32:0.29,0.3"
        ["warmwhite"]="0.44,0.41:0.46,0.43"
        ["neutralwhite"]="0.35,0.35:0.37,0.37"
      )
      local x y

      if [[ -z "$color" || "$color" == "random" || -z "''${color_ranges[$color]}" ]]; then
        x=$(LC_ALL=C awk -v seed=$RANDOM 'BEGIN {srand(seed); printf "%.4f\n", 0.1 + rand() * 0.6}')
        y=$(LC_ALL=C awk -v seed=$RANDOM 'BEGIN {srand(seed); printf "%.4f\n", 0.05 + rand() * 0.6}')
      else
        IFS=':' read -r min_range max_range <<< "''${color_ranges[$color]}"
        IFS=',' read -r min_x min_y <<< "$min_range"
        IFS=',' read -r max_x max_y <<< "$max_range"
        
        x=$(LC_ALL=C awk -v min="$min_x" -v max="$max_x" -v seed=$RANDOM 'BEGIN {srand(seed); printf "%.4f\n", min + rand() * (max - min)}')
        y=$(LC_ALL=C awk -v min="$min_y" -v max="$max_y" -v seed=$RANDOM 'BEGIN {srand(seed); printf "%.4f\n", min + rand() * (max - min)}')
      fi
      
      LC_ALL=C awk -v x="$x" -v y="$y" 'BEGIN {printf "[%.4f,%.4f]\n", x, y}'
    }    
    
    hex_to_xy() {
      local hex="$1"
      local r g b 
      hex=$(echo "$hex" | sed 's/^#//')
      [[ ''${#hex} -eq 6 ]] || { echo "0.5 0.4"; return 1; }
  
      r=$((16#''${hex:0:2}))
      g=$((16#''${hex:2:2}))
      b=$((16#''${hex:4:2}))
  
      local r_cor g_cor b_cor
      r_cor=$(echo "scale=4; $r / 255" | bc -l)
      g_cor=$(echo "scale=4; $g / 255" | bc -l)
      b_cor=$(echo "scale=4; $b / 255" | bc -l)
  
      r_cor=$(echo "scale=4; if ($r_cor > 0.04045) { e(2.4 * l($r_cor / 1.055 + 0.055)) } else { $r_cor / 12.92 }" | bc -l)
      g_cor=$(echo "scale=4; if ($g_cor > 0.04045) { e(2.4 * l($g_cor / 1.055 + 0.055)) } else { $g_cor / 12.92 }" | bc -l)
      b_cor=$(echo "scale=4; if ($b_cor > 0.04045) { e(2.4 * l($b_cor / 1.055 + 0.055)) } else { $b_cor / 12.92 }" | bc -l)
  
      local x y z
      x=$(echo "scale=4; ($r_cor * 0.649926 + $g_cor * 0.103455 + $b_cor * 0.197109)" | bc -l)
      y=$(echo "scale=4; ($r_cor * 0.234327 + $g_cor * 0.743075 + $b_cor * 0.022598)" | bc -l)
      z=$(echo "scale=4; ($r_cor * 0.000000 + $g_cor * 0.053077 + $b_cor * 1.035763)" | bc -l)
  
      local total
      total=$(echo "scale=4; $x + $y + $z" | bc -l)  
      if [[ $(echo "$total == 0" | bc -l) -eq 1 ]]; then
        echo "0.5 0.4"
      else
        local x_y y_y
        x_y=$(echo "scale=4; $x / $total" | bc -l)
        y_y=$(echo "scale=4; $y / $total" | bc -l)
        echo "$x_y $y_y"
      fi
    }
  '';


in {
  config = {
    environment.systemPackages = [
          (pkgs.writeScriptBin "scene-roll" ''
            ${cmdHelpers}
            ${lib.concatStringsSep "\n" (lib.flatten (lib.mapAttrsToList (_: cmds: lib.mapAttrsToList (_: cmd: cmd) cmds) sceneCommands))}
          '')
          

          (pkgs.writeScriptBin "scene" ''
            ${cmdHelpers}
            MQTT_BROKER="${config.house.zigbee.mosquitto.host}"
            MQTT_USER="${config.house.zigbee.mosquitto.username}"
            MQTT_PASSWORD=$(cat "${config.house.zigbee.mosquitto.passwordFile}") # ⮜ 🦆 says password file
            SCENE="$1"      
            # 🦆 says ⮞ convert to lowercase
            SCENE_LOWER=$(echo "$SCENE" | tr '[:upper:]' '[:lower:]')
      
            # 🦆 says ⮞ no scene == random scene
            if [ -z "$SCENE" ]; then
              SCENE=$(shuf -n 1 -e ${lib.concatStringsSep " " (lib.map (name: "\"${name}\"") (lib.attrNames sceneCommands))})
              SCENE_LOWER=$(echo "$SCENE" | tr '[:upper:]' '[:lower:]')
            fi
      
            # 🦆 says ⮞ create lowercase scene names
            case "$SCENE_LOWER" in
            ${
              lib.concatStringsSep "\n" (
                lib.mapAttrsToList (sceneName: cmds:
                  let
                    commandLines = lib.concatStringsSep "\n    " (
                      lib.mapAttrsToList (_: cmd: cmd) cmds
                    );
                    lowercaseName = lib.toLower sceneName;
                  in
                    "\"${lowercaseName}\")\n    ${commandLines}\n    ;;"
                ) sceneCommands
              )
            }
            *)
              say_duck "fuck ❌"
              exit 1
              ;;
            esac
          '')  
          

          (pkgs.writeScriptBin "zig" ''
            ${cmdHelpers}
            set -euo pipefail
            # 🦆 says ⮞ create case insensitive map of device friendly_name
            declare -A device_map=(
              ${lib.concatStringsSep "\n" (lib.mapAttrsToList (k: v: "['${lib.toLower k}']='${v}'") normalizedDeviceMap)}
            )
            available_devices=(
              ${toString deviceList}
            )    
            DEVICE="$1" # 🦆 says ⮞ device to control      
            STATE="''${2:-}" # 🦆 says ⮞ state change        
            BRIGHTNESS="''${3:-100}"
            COLOR="''${4:-}"
            TEMP="''${5:-}"
            ZIGBEE_DEVICES='${deviceMeta}'
            MQTT_BROKER="${config.house.zigbee.mosquitto.host}"

            MQTT_USER="${config.house.zigbee.mosquitto.username}"
            MQTT_PASSWORD=$(cat "${config.house.zigbee.mosquitto.passwordFile}") # ⮜ 🦆 says password file
            # 🦆 says ⮞ Zigbee coordinator backup
            if [[ "$DEVICE" == "backup" ]]; then
              mqtt_pub -t "zigbee2mqtt/backup/request" -m '{"action":"backup"}'
              say_duck "Zigbee coordinator backup requested! - processing on server..."
              exit 0
            fi         
            # 🦆 says ⮞ validate device
            input_lower=$(echo "$DEVICE" | tr '[:upper:]' '[:lower:]')
            exact_name=''${device_map["$input_lower"]}
            if [[ -z "$exact_name" ]]; then
              say_duck "fuck ❌ device not found: $DEVICE" >&2
              say_duck "Available devices: ${toString (builtins.attrNames zigbeeDevices)}" >&2
              exit 1
            fi
            # 🦆 says ⮞ if COLOR da lamp prob want hex yo
            if [[ -n "$COLOR" ]]; then
              COLOR=$(color2hex "$COLOR") || {
                say_duck "fuck ❌ Invalid color: $COLOR" >&2
                exit 1
              }
            fi
            # 🦆 says ⮞ turn off the device
            if [[ "$STATE" == "off" ]]; then
              mqtt_pub -t "zigbee2mqtt/$exact_name/set" -m '{"state":"OFF"}'
              say_duck " turned off $DEVICE"
              exit 0
            fi    
            # 🦆 says ⮞ turn down the device brightness
            if [[ "$STATE" == "down" ]]; then
              say_duck "🔻 Decreasing $light_id in $clean_room"
              mqtt_pub -t "zigbee2mqtt/$exact_name/set" -m '{"brightness_step":-50,"transition":3.5}'
              exit 0
            fi      
            # 🦆 says ⮞ turn up the device brightness
            if [[ "$STATE" == "up" ]]; then
              say_duck "🔺 Increasing brightness on $light_id in $clean_room"
              mqtt_pub -t "zigbee2mqtt/$exact_name/set" -m '{"brightness_step":50,"transition":3.5}'
              exit 0
            fi      
                        
            # 🦆 says ⮞ construct payload
            PAYLOAD="{\"state\":\"ON\""
            [[ -n "$BRIGHTNESS" ]] && PAYLOAD+=", \"brightness\":$BRIGHTNESS"
            [[ -n "$COLOR" ]] && PAYLOAD+=", \"color\":{\"hex\":\"$COLOR\"}"
            PAYLOAD+="}"
            # 🦆 says ⮞ publish payload
            mqtt_pub -t "zigbee2mqtt/$exact_name/set" -m "$PAYLOAD"
            say_duck "$PAYLOAD" 
     
     
     
            # 🦆TODO⮞ BRIDGED PAYLOAD 
            PAYLOAD="{\"state\":\"true\""
            [[ -n "$BRIGHTNESS" ]] && PAYLOAD+=", \"bri\":$BRIGHTNESS"
            [[ -n "$COLOR" ]] && PAYLOAD+=", \"color\":{\"hex\":\"$COLOR\"}"
            PAYLOAD+="}"
            # 🦆 says ⮞ publish payload
            mqtt_pub -t "zigbee2mqtt/$exact_name/set" -m "$PAYLOAD"
            say_duck "$PAYLOAD" 
            
               
     
     
     
            
            
          '')
          

          ( pkgs.writeScriptBin "hue" ''
            ${cmdHelpers}
            # set -euo pipefail
          
            # 🦆 says ⮞ configuration loaded at build time
            if [ "${if config.house.zigbee.hueSyncBox != null && config.house.zigbee.hueSyncBox.enable then "1" else "0"}" = "1" ]; then
              HUE_BRIDGE_IP="${config.house.zigbee.hueSyncBox.bridge.ip}"
              HUE_BRIDGE_API_KEY="$(cat "${config.house.zigbee.hueSyncBox.bridge.passwordFile}" 2>/dev/null || echo "")"
              HUE_SYNC_BOX_IP="${config.house.zigbee.hueSyncBox.syncBox.ip}"
              HUE_SYNC_BOX_API_KEY="$(cat "${config.house.zigbee.hueSyncBox.syncBox.passwordFile}" 2>/dev/null || echo "")"
              HUE_INSECURE="${toString config.house.zigbee.hueSyncBox.insecure}"
              HUE_SKIP_CERT_CHECK="${toString config.house.zigbee.hueSyncBox.skipCertCheck}"
              
              # 🦆 says ⮞ build-time device mapping (keyed by friendly_name)
              HUE_DEVICE_MAP='${builtins.toJSON (
                let
                  zigbeeConfig = config.house.zigbee;
                  # 🦆 says ⮞ get all devices with hue_id
                  hueDevices = lib.attrsets.filterAttrs (name: device: device.hue_id != null) zigbeeConfig.devices;
                  # 🦆 says ⮞ create mapping from friendly_name to hue_id
                  hueDeviceMapping = builtins.listToAttrs (
                    builtins.filter (x: x != null) (
                      builtins.map (device:
                        let
                          deviceInfo = builtins.getAttr device hueDevices;
                        in
                          if deviceInfo.hue_id != null then
                            {
                              name = deviceInfo.friendly_name;
                              value = {
                                hue_id = deviceInfo.hue_id;
                                zigbee_key = device;
                                room = deviceInfo.room;
                                type = deviceInfo.type;
                              };
                            }
                          else null
                      ) (builtins.attrNames hueDevices)
                    )
                  );
                in
                  hueDeviceMapping
              )}'
              
              
              # 🦆 says ⮞ Nix scenes
              HUE_NIX_SCENES='${builtins.toJSON config.house.zigbee.scenes}'
            else
              HUE_BRIDGE_IP=""
              HUE_BRIDGE_API_KEY=""
              HUE_SYNC_BOX_IP=""
              HUE_SYNC_BOX_API_KEY=""
              HUE_INSECURE="false"
              HUE_SKIP_CERT_CHECK="false"
              HUE_DEVICE_MAP='{}'
              HUE_NIX_SCENES='{}'
            fi
          
            # 🦆 says ⮞ fetch hue states and update global state.json
            update_state_file() {
              STATE_FILE="/var/lib/zigduck/state.json"
              HUE_JSON="$(hue bridge lights)"
              NOW_ISO="$(date --iso-8601=seconds)"
              NOW_EPOCH="$(date +%s)"
              
              jq \
                --argjson hue "$HUE_JSON" \
                --arg now_iso "$NOW_ISO" \
                --arg now_epoch "$NOW_EPOCH" '
                reduce ($hue | keys[]) as $id (
                  .;
                  (
                    $hue[$id] as $l
                    | $l.name as $name
                    | .[$name] = (
                        (.[$name] // {})
                        + {
                            state: (if $l.state.on then "ON" else "OFF" end),
                            brightness: ($l.state.bri | tostring),
                            color: (
                              if ($l.state.xy | length) == 2
                              then "{\"x\":" + ($l.state.xy[0]|tostring)
                                   + ",\"y\":" + ($l.state.xy[1]|tostring) + "}"
                              else .[$name].color
                              end
                            ),
                            last_seen: $now_iso,
                            last_updated: $now_epoch
                          }
                      )
                  )
                )
              ' "$STATE_FILE" > "''${STATE_FILE}.tmp"      
              mv "''${STATE_FILE}.tmp" "$STATE_FILE" 
            }

            # 🦆 says ⮞ helpers
            load_device_map() {
              echo "$HUE_DEVICE_MAP" | ${pkgs.jq}/bin/jq '.'
            }
            
          
            load_nix_scenes() {
              echo "$HUE_NIX_SCENES" | ${pkgs.jq}/bin/jq '.'
            }
          
            get_hue_id() {
              local friendly_name="$1"
              local device_map
              device_map=$(load_device_map)
              echo "$device_map" | ${pkgs.jq}/bin/jq -r --arg name "$friendly_name" '
                if .[$name] then .[$name].hue_id else null end
              '
            }
          
            get_device_info() {
              local friendly_name="$1"
              local device_map
              device_map=$(load_device_map)
              echo "$device_map" | ${pkgs.jq}/bin/jq -r --arg name "$friendly_name" '
                if .[$name] then .[$name] else null end
              '
            }
          
            list_hue_devices() {
              local device_map
              device_map=$(load_device_map)
              echo "$device_map" | ${pkgs.jq}/bin/jq -r '
                to_entries[] | 
                "\(.key) (hue_id: \(.value.hue_id), room: \(.value.room), type: \(.value.type))"
              '
            }
          
            list_nix_scenes() {
              local nix_scenes
              nix_scenes=$(load_nix_scenes)
              echo "$nix_scenes" | ${pkgs.jq}/bin/jq -r '
                to_entries[] | 
                .key
              '
            }
          
            get_nix_scene_info() {
              local scene_name="$1"
              local nix_scenes
              nix_scenes=$(load_nix_scenes)
              echo "$nix_scenes" | ${pkgs.jq}/bin/jq -r --arg scene "$scene_name" '
                if .[$scene] then .[$scene] else null end
              '
            }
          
            hue_api() {
              local target="$1" method="$2" endpoint="$3" data="$4"
              local ip key base curl_opts=""  
              case "$target" in
                bridge)
                  ip="$HUE_BRIDGE_IP"
                  key="$HUE_BRIDGE_API_KEY"
                  base="http://$ip/api/$key"
                  [ "$HUE_INSECURE" = "true" ] && curl_opts="-k"
                  ;;
                sync)
                  ip="$HUE_SYNC_BOX_IP"
                  key="$HUE_SYNC_BOX_API_KEY"
                  base="https://$ip/api/v1/$key"
                  [ "$HUE_SKIP_CERT_CHECK" = "true" ] && curl_opts="-k"
                  ;;
                *)
                  say_duck "fuck ❌ Invalid target: $target"
                  say_duck "Use: \"bridge\" or \"sync\""
                  exit 1
                  ;;
              esac      
              [[ -z "$ip" || -z "$key" ]] && {
                say_duck "fuck ❌ $target not configured or API key missing"
                exit 1
              }
              if [[ -n "$data" ]]; then
                curl $curl_opts -X "$method" "$base$endpoint" \
                  -H "Content-Type: application/json" \
                  -d "$data" 2>/dev/null || { say_duck "fuck ❌ $target API call failed"; exit 1; }
              else
                curl $curl_opts -X "$method" "$base$endpoint" 2>/dev/null || { say_duck "$target API call failed"; exit 1; }
              fi
            }
          
            # 🦆 says ⮞ ACTIVATE NIX SCENE ON HUE DEVICES
            apply_nix_scene() {
              local scene_name="$1"
              local nix_scenes
              nix_scenes=$(load_nix_scenes)
              local scene_def
              scene_def=$(echo "$nix_scenes" | ${pkgs.jq}/bin/jq -r --arg scene "$scene_name" '.[$scene]')
              
              if [ -z "$scene_def" ] || [ "$scene_def" = "null" ]; then
                say_duck "fuck ❌ No Nix scene found: $scene_name"
                say_duck "Available Nix scenes:"
                list_nix_scenes | sed 's/^/  /'
                exit 1
              fi
              
              say_duck "Applying Nix scene: $scene_name"
              local applied_count=0
              local skipped_count=0
              
              local device_names
              device_names=$(echo "$scene_def" | ${pkgs.jq}/bin/jq -r 'keys[]')
              
              while IFS= read -r friendly_name; do
                local hue_id
                hue_id=$(get_hue_id "$friendly_name")
                
                if [ -z "$hue_id" ] || [ "$hue_id" = "null" ]; then
                  say_duck "⚠️ Skipping $friendly_name: no hue_id"
                  skipped_count=$((skipped_count + 1))
                  continue
                fi
                
                local device_state
                device_state=$(echo "$scene_def" | ${pkgs.jq}/bin/jq -c --arg name "$friendly_name" '.[$name]')
                
                # 🦆says⮞ STATE BUILD
                local state
                state=$(echo "$device_state" | ${pkgs.jq}/bin/jq -r '.state // "ON"')
                
                local update_json="{\"on\":"
                if [ "$state" = "ON" ]; then
                  update_json="''${update_json}true"
  
                  # 🦆says⮞ brightness
                  if [ "$brightness" != "null" ] && [ "$brightness" != "" ]; then
                    update_json="''${update_json}, \"bri\":$brightness"
                  fi
  
                  # 🦆says⮞  color (supports all Hue formats)
                  local xy_json hue_val sat_val ct_val
                  xy_json=$(echo "$device_state" | ${pkgs.jq}/bin/jq -r '.color.xy')
                  hue_val=$(echo "$device_state" | ${pkgs.jq}/bin/jq -r '.color.hue')
                  sat_val=$(echo "$device_state" | ${pkgs.jq}/bin/jq -r '.color.saturation')
                  ct_val=$(echo "$device_state" | ${pkgs.jq}/bin/jq -r '.color.ct // .color.temp')
  
                  if [ "$xy_json" != "null" ] && [ "$xy_json" != "" ]; then
                    # 🦆says⮞  xy color
                    update_json="''${update_json}, \"xy\":$xy_json"
                  elif [ "$hue_val" != "null" ] && [ "$sat_val" != "null" ] && [ "$hue_val" != "" ] && [ "$sat_val" != "" ]; then
                    # 🦆says⮞  hue/sat
                    update_json="''${update_json}, \"hue\":$hue_val, \"sat\":$sat_val"
                  elif [ "$ct_val" != "null" ] && [ "$ct_val" != "" ]; then
                    # 🦆says⮞ color temp
                    update_json="''${update_json}, \"ct\":$ct_val"
                  else
                    # 🦆says⮞ fallback2hex
                    hex_value=$(echo "$device_state" | ${pkgs.jq}/bin/jq -r '.color.hex // .color')
                    if [ "$hex_value" != "null" ] && [ "$hex_value" != "" ]; then
                      local xy_coords
                      xy_coords=$(hex_to_xy "$hex_value") || {
                        say_duck "⚠️ Skipping color for $friendly_name: invalid hex '$hex_value'"
                      }
                      if [ -n "$xy_coords" ]; then
                        local x y
                        x=$(echo "$xy_coords" | cut -d' ' -f1)
                        y=$(echo "$xy_coords" | cut -d' ' -f2)
                        update_json="''${update_json}, \"xy\":[$x,$y]"
                      fi
                    fi
                  fi
  
                  # 🦆says⮞ effect
                  local effect_val
                  effect_val=$(echo "$device_state" | ${pkgs.jq}/bin/jq -r '.effect')
                  if [ "$effect_val" != "null" ] && [ "$effect_val" != "" ] && [ "$effect_val" != "none" ]; then
                    update_json="''${update_json}, \"effect\":\"$effect_val\""
                  fi
  
                  # 🦆says⮞ alert
                  local alert_val
                  alert_val=$(echo "$device_state" | ${pkgs.jq}/bin/jq -r '.alert')
                  if [ "$alert_val" != "null" ] && [ "$alert_val" != "" ] && [ "$alert_val" != "none" ]; then
                    update_json="''${update_json}, \"alert\":\"$alert_val\""
                  fi
                else
                  update_json="''${update_json}false"
                fi
                
                update_json="''${update_json}}"
                
                hue_api bridge PUT "/lights/$hue_id/state" "$update_json" > /dev/null 2>&1
                if [ $? -eq 0 ]; then
                  say_duck "$friendly_name (hue_id: $hue_id): $state"
                  applied_count=$((applied_count + 1))
                else
                  say_duck "fuck  ❌ Failed to update $friendly_name"
                fi
                
                # 🦆says⮞tiny delay - safety first!
                sleep 0.1
              done <<< "$device_names"
              
              say_duck "Scene '$scene_name' applied! ($applied_count hue devices, $skipped_count non-hue devices skipped)"
              update_state_file
            }
          
            # 🦆 says ⮞ routing
            case "$1" in
              # 🦆 says ⮞ bridge
              bridge)
                case "$2" in
                  devices|list)
                    echo "Hue devices configured in zigbee:"
                    list_hue_devices
                    ;;
                  scenes)
                    echo "Bridge scenes from Hue:"
                    hue_api bridge GET "/scenes" "" | ${pkgs.jq}/bin/jq '.'
                    ;;
                  nix-scenes)
                    echo "Available Nix scenes:"
                    list_nix_scenes
                    ;;
                  nix-scene-info)
                    scene_name="$3"
                    echo "Nix scene info for: $scene_name"
                    get_nix_scene_info "$scene_name" | ${pkgs.jq}/bin/jq '.'
                    ;;
                  groups)
                    hue_api bridge GET "/groups" "" | ${pkgs.jq}/bin/jq '.'
                    ;;
                  sync-state)
                    update_state_file
                    ;;
                  lights)
                    hue_api bridge GET "/lights" "" | ${pkgs.jq}/bin/jq '.'
                    ;;
                  light)
                    friendly_name="$3"
                    action="$4"
                    value="''${5:-}"
                    
                    # 🦆 says ⮞ get hue_id from friendly_name
                    hue_id=$(get_hue_id "$friendly_name")
                    if [ -z "$hue_id" ] || [ "$hue_id" = "null" ]; then
                      say_duck "fuck ❌ No hue_id found for device: $friendly_name"
                      say_duck "Available devices:"
                      list_hue_devices | sed 's/^/  /'
                      exit 1
                    fi
                    
                    case "$action" in
                      on)
                        hue_api bridge PUT "/lights/$hue_id/state" '{"on":true}'
                        say_duck "Turned on $friendly_name (hue_id: $hue_id)"
                        ;;
                      off)
                        hue_api bridge PUT "/lights/$hue_id/state" '{"on":false}'
                        say_duck "Turned off $friendly_name (hue_id: $hue_id)"
                        ;;
                      toggle)
                        current_state=$(hue_api bridge GET "/lights/$hue_id" "")
                        is_on=$(echo "$current_state" | ${pkgs.jq}/bin/jq -r '.state.on')
                        new_state=$([ "$is_on" = "true" ] && echo "false" || echo "true")
                        hue_api bridge PUT "/lights/$hue_id/state" "{\"on\":$new_state}"
                        say_duck "Toggled $friendly_name (hue_id: $hue_id) → $([ "$new_state" = "true" ] && echo "ON" || echo "OFF")"
                        ;;
                      brightness|bri)
                        [[ "$value" =~ ^[0-9]+$ && "$value" -ge 0 && "$value" -le 254 ]] || {
                          say_duck "fuck ❌ Brightness must be 0-254"
                          exit 1
                        }
                        hue_api bridge PUT "/lights/$hue_id/state" "{\"bri\":$value}"
                        say_duck "Set $friendly_name brightness to $value"
                        ;;
                      color)
                        xy_json=$(color2xy "$value") || {
                          say_duck "fuck ❌ Invalid color: $value"
                          exit 1
                        }
                        hue_api bridge PUT "/lights/$hue_id/state" "{\"xy\":$xy_json}"
                        say_duck "Set $friendly_name color to $value (xy: $xy_json)"
                        ;;
                      state)
                        # 🦆 says ⮞ advanced! set multiple properties
                        if [[ -n "$value" ]]; then
                          if echo "$value" | ${pkgs.jq}/bin/jq . >/dev/null 2>&1; then
                            hue_api bridge PUT "/lights/$hue_id/state" "$value"
                            say_duck "Updated $friendly_name state"
                          else
                            say_duck "fuck ❌ Invalid JSON in state payload"
                            exit 1
                          fi
                        else
                          say_duck "fuck ❌ State requires JSON payload"
                          say_duck "Example: hue bridge light \"TV Play Strip\" state '{\"on\":true, \"bri\":200, \"xy\":[0.1709,0.3693]}'"
                          exit 1
                        fi
                        ;;
                      info|status)
                        echo "Device info for $friendly_name:"
                        get_device_info "$friendly_name" | ${pkgs.jq}/bin/jq '.'
                        echo "Current state from bridge:"
                        hue_api bridge GET "/lights/$hue_id" "" | ${pkgs.jq}/bin/jq '.'
                        ;;
                      *)
                        say_duck "fuck ❌ Unknown light action: $action"
                        say_duck "Available actions: on, off, toggle, brightness, color, state, info"
                        exit 1
                        ;;
                    esac
                    ;;
                  scene)
                    # 🦆 says ⮞ activate Hue scenes (by id)
                    scene_id="$3"
                    hue_api bridge PUT "/groups/0/action" "{\"scene\":\"$scene_id\"}"
                    say_duck "Activated bridge scene: $scene_id"
                    ;;
                  apply-scene|nix-scene)
                    # 🦆 says ⮞ Nix configured scenes
                    scene_name="$3"
                    apply_nix_scene "$scene_name"
                    ;;
                  group)
                    group_id="$3"
                    action="$4"
                    case "$action" in
                      on)
                        hue_api bridge PUT "/groups/$group_id/action" '{"on":true}'
                        ;;
                      off)
                        hue_api bridge PUT "/groups/$group_id/action" '{"on":false}'
                        ;;
                      brightness|bri)
                        value="$5"
                        [[ "$value" =~ ^[0-9]+$ && "$value" -ge 0 && "$value" -le 254 ]] || {
                          say_duck "fuck ❌ Brightness must be 0-254"
                          exit 1
                        }
                        hue_api bridge PUT "/groups/$group_id/action" "{\"bri\":$value}"
                        ;;
                      *)
                        say_duck "fuck ❌ Unknown group action: $action"
                        exit 1
                        ;;
                    esac
                    ;;
                  help)
                    cat <<EOF
          🦆 Hue Bridge Commands:
            devices, list         List configured hue devices
            scenes                List scenes from Hue bridge
            nix-scenes            List available Nix scenes
            nix-scene-info <name> Show details of a Nix scene
            groups                List all groups from bridge
            light <name> <action> Control a light by friendly name
                Actions: on, off, toggle, brightness <0-254>, color <name/hex>, state <json>, info
            scene <id>            Activate a bridge scene (by ID only)
            apply-scene <name>    Apply a Nix-defined scene to hue devices
            group <id> <action>   Control a group
                Actions: on, off, brightness <0-254>
          
          Examples:
            hue bridge devices
            hue bridge nix-scenes
            hue bridge apply-scene backlit
            hue bridge apply-scene dark
            hue bridge light "TV Play 1" on
            hue bridge light "TV Play Strip" brightness 150
            hue bridge nix-scene-info backlit
            hue bridge light "TV Play Strip" state '{"on":true, "bri":200, "xy":[0.1709,0.3693]}'
          EOF
                    ;;
                  *)
                    say_duck "fuck ❌ Unknown bridge command: $2"
                    say_duck "Use: help, devices, scenes, nix-scenes, groups, light, scene, apply-scene, group"
                    exit 1
                    ;;
                esac
                ;;            
              
              # 🦆 says ⮞ syncBox
              sync)
                case "$2" in
                  on)
                    hue_api sync PUT "/sync" '{"syncActive":true}'
                    ;;
                  off)
                    hue_api sync PUT "/sync" '{"syncActive":false}'
                    ;;
                  toggle)
                    status=$(hue_api sync GET "" "")
                    is_active=$(echo "$status" | ${pkgs.jq}/bin/jq -r '.execution.syncActive')
                    new_state=$([ "$is_active" = "true" ] && echo "false" || echo "true")
                    hue_api sync PUT "/sync" "{\"syncActive\":$new_state}"
                    ;;
                  status)
                    hue_api sync GET "" "" | ${pkgs.jq}/bin/jq '.'
                    ;;
                  mode)
                    mode="$3"
                    case "$mode" in
                      video|music|game)
                        hue_api sync PUT "/sync" "{\"mode\":\"$mode\"}"
                        ;;
                      *)
                        say_duck "fuck ❌ Invalid mode: $mode"
                        exit 1
                        ;;
                    esac
                    ;;
                  intensity)
                    intensity="$3"
                    case "$intensity" in
                      subtle|moderate|high|intense)
                        hue_api sync PUT "/sync" "{\"intensity\":\"$intensity\"}"
                        ;;
                      *)
                        say_duck "fuck ❌ Invalid intensity: $intensity"
                        exit 1
                        ;;
                    esac
                    ;;
                  entertainment-area)
                    area_id="$3"
                    hue_api sync PUT "/sync" "{\"entertainmentConfiguration\":\"$area_id\"}"
                    ;;
                  hdmi-input)
                    input="$3"
                    [[ "$input" =~ ^[1-4]$ ]] || {
                      say_duck "fuck ❌ Invalid HDMI input: $input"
                      exit 1
                    }
                    hue_api sync PUT "/sync" "{\"hdmiSource\":\"input$input\"}"
                    ;;
                  help)
                    cat <<EOF
          🦆 Hue Sync Box Commands:
            on                    Turn sync on
            off                   Turn sync off
            toggle                Toggle sync state
            status                Get sync box status
            mode <mode>           Set sync mode (video|music|game)
            intensity <level>     Set intensity (subtle|moderate|high|intense)
            entertainment-area <id> Set entertainment area
            hdmi-input <1-4>      Set HDMI input source
          EOF
                    ;;
                  *)
                    say_duck "fuck ❌ Unknown sync command: $2"
                    say_duck "Use: help, on, off, toggle, status, mode, intensity, entertainment-area, hdmi-input"
                    exit 1
                    ;;
                esac
                ;;
              
              # 🦆 says ⮞ help
              help|--help|-h)
                cat <<EOF
          🦆 Philips Hue Control Script
          
          Usage:
            hue bridge <command> [args...]    Control Hue Bridge
            hue sync <command> [args...]      Control Hue Sync Box
            hue help                         Show this help
          
          Quick Examples:
            hue bridge nix-scenes             # List all Nix scenes
            hue bridge apply-scene backlit    # Apply Nix "backlit" scene to hue devices
            hue bridge apply-scene dark       # Apply Nix "dark" scene to hue devices
            hue bridge devices                # List all hue devices
            hue bridge light "TV Play 1" on  # Turn on a light by name
            hue sync on                       # Turn on sync box
            hue sync mode video              # Set sync mode to video
          
          Use 'hue bridge help' or 'hue sync help' for more detailed help.
          EOF
                ;;         
              *)
                say_duck "fuck ❌ Unknown command: $1"
                say_duck "Use: \"bridge\", \"sync\", or \"help\""
                exit 1
                ;;
            esac
          '')
    ];
    
  };}
         
