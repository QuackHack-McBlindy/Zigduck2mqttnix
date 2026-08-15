{ 
  config,
  lib,
  ...
} : let
  inherit (lib)
    filterAttrs mapAttrsToList mapAttrs' nameValuePair flatten
    optional attrNames concatStringsSep take length unique
    hasAttr attrValues subtractLists listToAttrs map attrsToList
    ;

  getAllFriendlyNames =
    let devices = config.house.zigbee.devices or { };
    in mapAttrsToList (_: device: device.friendly_name) devices;

  friendlyNamesSet =
    let names = getAllFriendlyNames;
    in builtins.listToAttrs (map (name: { inherit name; value = true; }) names);

  deviceExistsByFriendlyName = deviceName:
    builtins.hasAttr deviceName friendlyNamesSet;

  roomExists = roomName:
    builtins.hasAttr roomName (config.house.rooms or { });

  isMqttEnabled = config.house.zigbee.mosquitto != null && 
                  (config.house.zigbee.mosquitto.username != null || 
                   config.house.zigbee.mosquitto.passwordFile != null);

  isValidHexColor = color:
    let cleanColor = lib.removePrefix "#" color;
    in lib.strings.match "[0-9A-Fa-f]{6}" cleanColor != null;

  isValidBrightness = brightness:
    brightness >= 0 && brightness <= 254;

  isValidState = state:
    builtins.elem state [ "ON" "OFF" ];

  getAllMotionSensors =
    let devices = config.house.zigbee.devices or { };
    in filterAttrs (_: device: device.type == "motion") devices;

  getMotionSensorNames =
    let motionSensors = getAllMotionSensors;
    in mapAttrsToList (_: device: device.friendly_name) motionSensors;

  validateScene = sceneName: sceneDevices:
    let availableNames = getAllFriendlyNames;
    in flatten (mapAttrsToList (deviceName: settings:
      [
        {
          assertion = deviceExistsByFriendlyName deviceName;
          message = "🦆 duck say ⮞ fuck ❌ Scene '${sceneName}' references non-existent device '${deviceName}'. Available: ${concatStringsSep ", " (take 10 availableNames)}${if length availableNames > 10 then "..." else ""}";
        }
        
        {
          assertion = settings ? state -> isValidState settings.state;
          message = "🦆 duck say ⮞ fuck ❌ Scene '${sceneName}' device '${deviceName}' has invalid state '${settings.state}' (must be ON or OFF)";
        }
        
        {
          assertion = settings ? brightness -> isValidBrightness settings.brightness;
          message = "🦆 duck say ⮞ fuck ❌ Scene '${sceneName}' device '${deviceName}' has invalid brightness ${toString settings.brightness} (must be 0-254)";
        }
        
        {
          assertion = settings ? color -> (
            (settings.color ? hex && isValidHexColor settings.color.hex) ||
            (settings.color ? xy && lib.isList settings.color.xy && lib.length settings.color.xy == 2) ||
            (settings.color ? hue && settings.color ? saturation && lib.isInt settings.color.hue && lib.isInt settings.color.saturation) ||
            (settings.color ? ct && lib.isInt settings.color.ct)
          );
          message = "🦆 duck say ⮞ fuck ❌ Scene '${sceneName}' device '${deviceName}' has invalid color format (must have hex, xy, hue/sat, or ct)";
        }
        
      ]
    ) sceneDevices);

  validateDevice = deviceId: device:
    [
      {
        assertion = roomExists device.room;
        message = "🦆 duck say ⮞ fuck ❌ Device '${device.friendly_name}' (${deviceId}) assigned to non-existent room '${device.room}'";
      }

      {
        assertion = isValidState "ON";
        message = "🦆 duck say ⮞ fuck ❌ Device '${device.friendly_name}' state validation failed";
      }
      
    ];

  validateMotionSensors = automationName: sensors:
    let
      availableSensors = getMotionSensorNames;
      invalidSensors = lib.filter (sensor: !lib.elem sensor availableSensors) sensors;
    in
      optional (invalidSensors != [ ]) {
        assertion = false;
        message = "🦆 duck say ⮞ fuck ❌ Automation '${automationName}' references non-existent motion sensors: ${toString invalidSensors}. Available: ${toString availableSensors}";
      };

  sceneValidations = flatten (
    mapAttrsToList validateScene (config.house.zigbee.scenes or { })
  );

  deviceValidations = flatten (
    mapAttrsToList validateDevice (config.house.zigbee.devices or { })
  );

  motionSensorValidations = flatten (
    mapAttrsToList (name: automation:
      validateMotionSensors name (automation.motion_sensors or [ ])
    ) (config.house.zigbee.automations.presence_based or { })
  );

  duplicateFriendlyNameValidation =
    let
      friendlyNames = getAllFriendlyNames;
      uniqueNames = lib.unique friendlyNames;
    in
    [{
      assertion = length friendlyNames == length uniqueNames;
      message = "🦆 duck say ⮞ fuck ❌ Duplicate friendly names found: ${toString (subtractLists uniqueNames friendlyNames)}";
    }];

  validateMqttTriggered = automationName: automation:
    [{
      assertion = automation.topic != "";
      message = "🦆 duck say ⮞ fuck ❌ MQTT automation '${automationName}' has empty topic";
    }];

  mqttTriggeredValidations = flatten (
    mapAttrsToList validateMqttTriggered (config.house.zigbee.automations.mqtt_triggered or { })
  );

  mqttValidations = [
    {
      assertion = config.house.zigbee.mosquitto != null ->
        (config.house.zigbee.mosquitto.username != null) == (config.house.zigbee.mosquitto.passwordFile != null);
      message = "🦆 duck say ⮞ fuck ❌ MQTT authentication requires both username and passwordFile to be set together";
    }
    
    {
      assertion = config.house.zigbee.mosquitto != null && config.house.zigbee.mosquitto.ssl.enable ->
        (config.house.zigbee.mosquitto.ssl.clientCertFile != null) == (config.house.zigbee.mosquitto.ssl.clientKeyFile != null);
      message = "🦆 duck say ⮞ fuck ❌ MQTT SSL client authentication requires both clientCertFile and clientKeyFile";
    }
    
  ];

  syncBoxTvValidation = {
    assertion = config.house.zigbee.hueSyncBox != null &&
                config.house.zigbee.hueSyncBox.enable &&
                config.house.zigbee.hueSyncBox.syncBox.tv != "" ->
                builtins.hasAttr config.house.zigbee.hueSyncBox.syncBox.tv config.house.tv;
    message = let
      syncBox = config.house.zigbee.hueSyncBox;
      tv = syncBox.syncBox.tv;
      availableTvs = attrNames config.house.tv;
    in "🦆 duck say ⮞ fuck ❌ Hue Sync Box references non-existent TV '${tv}'. Available TVs: ${toString availableTvs}";
  };

  # --- New assertions ---

  # 1. Validate that excluded devices in no.motion exist
  noMotionConfig = config.house.zigbee.no.motion or {};
  noMotionTrigger = noMotionConfig.trigger or {};
  noMotionAllLightsOff = noMotionTrigger.all.lights.off or {};
  excludeDevices = noMotionAllLightsOff.exclude or [];

  excludedDevicesExistValidation =
    map (deviceName: {
      assertion = deviceExistsByFriendlyName deviceName;
      message = "🦆 duck say ⮞ fuck ❌ No-motion automation excludes non-existent device '${deviceName}'";
    }) excludeDevices;

  # 2. Compare no.motion timeout with motion.trigger.lights.duration
  #    (no.motion.after is in minutes, multiply by 60 to compare with duration in seconds)
  noMotionTimeoutComparison =
    let
      noMotionEnabled = noMotionAllLightsOff.enable or false;
      noMotionAfter = noMotionAllLightsOff.after or 0;
      motionDuration = config.house.zigbee.motion.trigger.lights.duration or 0;
    in
      optional noMotionEnabled {
        assertion = (noMotionAfter * 60) > motionDuration;
        message = "🦆 duck say ⮞ fuck ❌ No-motion timeout (${toString noMotionAfter} min × 60 = ${toString (noMotionAfter * 60)}) must be greater than motion trigger lights duration (${toString motionDuration})";
      };

in
{
  config.assertions =
    sceneValidations
    ++ deviceValidations
    ++ duplicateFriendlyNameValidation
    ++ motionSensorValidations
    ++ mqttValidations
    ++ mqttTriggeredValidations
    ++ [ syncBoxTvValidation ]
    ++ excludedDevicesExistValidation
    ++ noMotionTimeoutComparison;
}
