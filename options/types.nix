{
  lib,
  ...
} : let
  inherit (lib) types mkOption mkEnableOption mkMerge;  

  roomType = types.submodule {
    options = {
      icon = mkOption {
        type = types.str;
        description = "Material Design (mdi) icon representing the room.";
      };
    };
  }; 


  automationActionType = types.oneOf [
    (types.str)
    (types.submodule {
      options = {
        type = mkOption {
          type = types.enum ["mqtt" "shell" "scene" "wait"];
          default = "shell";
          description = "Type of automation action";
        };
        command = mkOption {
          type = types.nullOr types.str;
          default = null;
          description = "The shell command to execute (for shell type)";
        };
        topic = mkOption {
          type = types.nullOr types.str;
          default = null;
          description = "MQTT topic (for mqtt type)";
        };
        message = mkOption {
          type = types.nullOr types.str;
          default = null;
          description = "MQTT message (for mqtt type)";
        };
        scene = mkOption {
          type = types.nullOr types.str;
          default = null;
          description = "Scene name (for scene type)";
        };
        duration = mkOption {
          type = types.nullOr types.int;
          default = null;
          description = "Duration in seconds (for wait type)";
        };                
      };
    })
  ];


  dimmerActionType = types.submodule {
    options = {
      enable = mkEnableOption "Enable this dimmer action";
      description = mkOption {
        type = types.str;
        description = "Description of this action";
      };
      extra_actions = mkOption {
        type = types.listOf automationActionType;
        default = [];
        description = "Additional actions to perform when this dimmer action triggers";
      };
      override_actions = mkOption {
        type = types.listOf automationActionType;
        default = [];
        description = "If defined, replaces default behavior with these actions";
      };
    };
  };


  statusCardType = with lib.types; submodule {
    options = {
      enable = mkEnableOption "this status card";
      title = mkOption { type = str; };
      icon = mkOption { type = str; };
      color = mkOption { type = str; default = "#2ecc71"; };
      theme = lib.mkOption {
        type = lib.types.str;
        default = "neon";
        description = "Theme for this card (neon, minimal, dark, glass, colorful)";
      };
      group = mkOption {
        type = str;
        default = "default";
        example = "sensors";
        description = "Status cards are ordered by it's group name";
      };        

      source = mkOption {
        type = enum [ "file" ];
        default = "file";
      };      

      filePath = mkOption { 
        type = str; 
        default = ""; 
        description = "Path to JSON file for file source";
      };    

      jsonField = mkOption { 
        type = str; 
        default = ""; 
        description = "JSON field to extract from file for main value";
      };    
      detailsJsonField = mkOption { 
        type = nullOr str; 
        default = null;
        description = "JSON field to extract from file for details (optional)";
      };   

      format = mkOption { 
        type = str; 
        default = "{value}"; 
        description = "Format string for main value. Use {value} placeholder";
      };   
      detailsFormat = mkOption { 
        type = str; 
        default = "{value}"; 
        description = "Format string for details value. Use {value} placeholder";
      };
      chart = mkOption { 
        type = bool; 
        default = false; 
        description = "Wether to show a history chart in the status card";
      };
      historyField = mkOption { 
        type = str; 
        default = "history"; 
        description = "JSON field to extract history data from for the chart";
      };


      on_click_action = mkOption { 
        type = lib.types.listOf automationActionType;
        default = [];
        description = "Actions to perform when clicking this status card";
      };    
  
      # fallback values
      defaultValue = mkOption { type = str; default = ""; };
      defaultDetails = mkOption { type = str; default = ""; };   
      # legacy support - will be used if detailsJsonField is null
      details = mkOption { 
        type = str; 
        default = ""; 
        description = "Static details text (used if detailsJsonField is not set)";
      };
    };
  };

in {
  inherit automationActionType dimmerActionType roomType statusCardType;
}
