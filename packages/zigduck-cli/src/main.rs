use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
    collections::HashMap,
};    
use clap::{
    Parser,
    ValueEnum,
};
use rand:: {
    Rng,
    seq::SliceRandom,
};
use ducktrace_logger::*;
use serde::{Deserialize, Serialize};
use rumqttc::{Client, MqttOptions, QoS};
use anyhow::{Result, Context};
use colored::*;
use reqwest::blocking::Client as HttpClient;
use clap::Subcommand;
use comfy_table::presets::UTF8_FULL;
use comfy_table::*;

 
#[derive(Debug, Deserialize, Clone)]
struct CliConfig {
    mosquitto: Option<MosquittoConfig>,
    hue: Option<HueConfig>,
    api: Option<ApiConfig>,
}

#[derive(Debug, Deserialize, Clone)]
struct ApiConfig {
    url: Option<String>,
    password_file: Option<String>,
}


#[derive(Debug, Deserialize, Clone)]
struct MosquittoConfig {
    broker: String,
    user: String,
    password_file: Option<String>,
    #[serde(default = "default_base_topic")]
    base_topic: String,
}

fn default_base_topic() -> String {
    "zigbee2mqtt".to_string()
}

#[derive(Debug, Deserialize, Clone)]
struct HueConfig {
    bridge_ip: Option<String>,
    password_file: Option<String>,
}



#[derive(Subcommand, Debug)]
enum Commands {
    #[command(name = "timer")]
    Timer {
        #[command(subcommand)]
        action: TimerAction,
    },
    #[command(name = "alarm")]
    Alarm {
        #[command(subcommand)]
        action: AlarmAction,
    },
}

#[derive(Subcommand, Debug)]
enum AlarmAction {
    List,
    Add {
        #[arg(long)]
        hours: u8,
        #[arg(long)]
        minutes: u8,
        #[arg(long)]
        name: String,
        #[arg(long)]
        days: Option<String>,
        #[arg(long, help = "MQTT topic to publish when alarm fires")]
        topic: Option<String>,
        #[arg(long, help = "MQTT payload to publish")]
        payload: Option<String>,
    },
    Remove {
        #[arg(long)]
        id: u64,
    },
    Toggle {
        #[arg(long)]
        id: u64,
    },
}

#[derive(Subcommand, Debug)]
enum TimerAction {
    List,
    Set {
        #[arg(long, help = "Hours")]
        hours: Option<u32>,
        #[arg(long, help = "Minutes")]
        minutes: Option<u32>,
        #[arg(long, help = "Seconds")]
        seconds: Option<u32>,
        #[arg(long, help = "MQTT topic to publish when timer fires")]
        topic: Option<String>,
        #[arg(long, help = "MQTT payload to publish")]
        payload: Option<String>,
        #[arg(long, help = "Human‑readable name for the timer")]
        name: Option<String>,
    },    
    Pause {
        #[arg(long, help = "Timer ID")]
        id: u64,
    },
    Resume {
        #[arg(long, help = "Timer ID")]
        id: u64,
    },
    Cancel {
        #[arg(long, help = "Timer ID")]
        id: u64,
    },
}

#[derive(Parser)]
#[command(
    name = "zigduck-cli",
    version = "0.1.0",
    author = "QuackHack-McBLindy",
    about = "High-performance unified home automation controller",
    long_about = "Control Zigbee and Hue devices, scenes, and automations with Rust speed and reliability"
)]
struct Cli {
    #[arg(long, short, help = "MQTT broker host", env = "MQTT_BROKER", default_value = "127.0.0.1")]
    broker: String,

    #[arg(long, short = 'u', help = "MQTT username", env = "MQTT_USER", default_value = "mqtt")]
    user: String,

    #[arg(long, help = "MQTT password file", env = "MQTT_PASSWORD_FILE")]
    password_file: Option<PathBuf>,

    #[arg(long, help = "MQTT password", env = "MQTT_PASSWORD")]
    password: Option<String>,

    #[arg(long, short = 'v', action = clap::ArgAction::Count, help = "Verbosity level")]
    verbose: u8,

    #[arg(long, help = "Path to devices configuration", env = "DEVICES_CONFIG")]
    devices_config: Option<PathBuf>,

    #[arg(long, help = "Path to scenes configuration", env = "SCENES_CONFIG")]
    scenes_config: Option<PathBuf>,

    #[arg(long, help = "Hue Bridge IP", env = "HUE_BRIDGE_IP")]
    hue_bridge_ip: Option<String>,

    #[arg(long, help = "Hue Bridge API key", env = "HUE_API_KEY")]
    hue_api_key: Option<String>,

    #[arg(long, help = "Hue Bridge API key file", env = "HUE_KEY_FILE")]
    hue_key_file: Option<PathBuf>,


    #[arg(long, help = "Device name (friendly name)")]
    device: Option<String>,

    #[arg(long, help = "Room name")]
    room: Option<String>,

    #[arg(long, help = "Scene name")]
    scene: Option<String>,

    #[arg(long, help = "List devices, rooms, scenes, lights, or sensors")]
    list: Option<Option<ListType>>,

    #[arg(long, help = "Show a formatted device status table including state, battery, temperature")]
    status: bool,

    #[arg(long, help = "Path to local state.json (overrides API fetch)", env = "ZIGDUCK_STATE_FILE")]
    state_file: Option<PathBuf>,

    #[arg(long, num_args(0..=1), default_missing_value = "120", help = "Pairing duration in seconds (default: 120)")]
    pair: Option<Option<u16>>,

    #[arg(long, num_args = 0..=1, default_missing_value = "true", help = "Control all lights (optional true/false)")]
    all_lights: Option<String>,
    
    #[arg(long, help = "Control all blinds globally (up or down)")]
    blinds: Option<String>,

    #[arg(long, help = "Room name for cheap mode")]
    cheap_mode: Option<String>,

    #[arg(long, requires = "topic", help = "Publish a raw MQTT message")]
    publish: bool,
    
    #[arg(long, help = "MQTT topic (used with --publish)")]
    topic: Option<String>,

    #[arg(long, help = "Send raw JSON to a device")]
    json_cmd: bool,

    #[arg(long, help = "Device state: on/off/toggle/max/dark")]
    state: Option<String>,

    #[arg(long, help = "Brightness percentage (1-100)")]
    brightness: Option<u8>,

    #[arg(long, help = "Color name or hex code")]
    color: Option<String>,

    #[arg(long, help = "Color temperature (153-500)")]
    temperature: Option<u16>,

    #[arg(long, requires = "device", help = "Transition time in seconds")]
    transition: Option<f32>,

    #[arg(long, help = "Raw JSON payload (used with --json-cmd or --publish)")]
    payload: Option<String>,

    #[arg(long, value_enum, default_value = "auto", requires = "json_cmd", help = "Backend type (auto/zigbee/hue)")]
    backend: BackendType,

    #[arg(long, requires = "list", help = "Output list as JSON")]
    json_output: bool,

    #[arg(long, requires = "pair", help = "Watch for new devices during pairing")]
    watch: bool,

    #[arg(long, requires = "scene", help = "Pick a random scene")]
    random: bool,

    #[arg(long, requires = "scene", help = "Restrict scene to a specific room")]
    scene_room: Option<String>,

    #[arg(long, default_value_t = 300, requires = "cheap_mode", help = "Delay in seconds for cheap mode")]
    delay: u64,
    
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long, help = "zigduck API URL", env = "API_URL")]
    api_url: Option<String>,

    #[arg(long, help = "File containing API password", env = "API_PASSWORD_FILE")]
    api_password_file: Option<PathBuf>,

    #[arg(long, help = "API password directly", env = "API_PASSWORD")]
    api_password: Option<String>,
}

#[derive(Clone, ValueEnum)]
enum DeviceState {
    On,
    Off,
    Toggle,
}

#[derive(Clone, ValueEnum)]
enum ListType {
    Devices,
    Rooms,
    Scenes,
    Lights,
    Sensors,
}

#[derive(Clone, ValueEnum)]
enum BackendType {
    Auto,
    Zigbee,
    Hue,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceConfig {
    friendly_name: String,
    room: String,
    #[serde(rename = "type")]
    device_type: String,
    #[serde(default = "default_endpoint")]
    endpoint: u8,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    battery_type: Option<String>,
    hue_id: Option<u16>,
    supports_color: Option<bool>,
    supports_temperature: Option<bool>,
}

fn default_endpoint() -> u8 {
    11
}


#[derive(Debug, Clone, Serialize, Deserialize)]
struct SceneConfig {
    #[serde(default)]
    friendly_name: Option<String>,
    devices: HashMap<String, serde_json::Value>,
}


struct HueClient {
    base_url: String,
    client: HttpClient,
}

impl HueClient {
    fn new(bridge_ip: &str, api_key: &str) -> Result<Self> {
        let base_url = format!("http://{}/api/{}", bridge_ip, api_key);
        Ok(Self {
            base_url,
            client: HttpClient::new(),
        })
    }

    fn set_light_state(&self, light_id: u16, state: serde_json::Value) -> Result<()> {
        let url = format!("{}/lights/{}/state", self.base_url, light_id);
        let response = self.client
            .put(&url)
            .json(&state)
            .send()
            .context("Failed to send Hue API request")?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("Hue API error {}: {}", status, body);
        }
        Ok(())
    }

    fn get_light_state(&self, light_id: u16) -> Result<serde_json::Value> {
        let url = format!("{}/lights/{}", self.base_url, light_id);
        let response = self.client
            .get(&url)
            .send()
            .context("Failed to get Hue light state")?;
        if !response.status().is_success() {
            anyhow::bail!("Failed to get light state: {}", response.status());
        }
        Ok(response.json()?)
    }
}


struct ZigduckController {
	mqtt_client: Client,
	hue_client: Option<HueClient>,
	devices: HashMap<String, DeviceConfig>,
	scenes: HashMap<String, SceneConfig>,
	verbose: bool,
	base_topic: String,
}

impl ZigduckController {
	fn new(
	    broker: String,
	    user: String,
	    password: String,
	    hue_bridge_ip: Option<String>,
	    hue_api_key: Option<String>,    
	    devices_config: Option<PathBuf>,
	    scenes_config: Option<PathBuf>,
	    verbose: bool,
	    base_topic: String,
	) -> Result<Self> {
	    let mut mqttoptions = MqttOptions::new("zigduck-cli", &broker, 1883);
	    mqttoptions.set_credentials(&user, &password);
	    mqttoptions.set_keep_alive(Duration::from_secs(5));
	    
	    let hue_client = if let (Some(ip), Some(key)) = (hue_bridge_ip, hue_api_key) {
	        Some(HueClient::new(&ip, &key)?)
	    } else {
	        None
	    };
	    
	    let (mqtt_client, mut connection) = Client::new(mqttoptions, 10);
	    
	    let mqtt_client_clone = mqtt_client.clone();
	    std::thread::spawn(move || {
	        for notification in connection.iter() {
	            match notification {
	                Ok(_) => {},
	                Err(e) => {
	                    eprintln!("MQTT connection error: {}", e);
	                    break;
	                }
	            }
	        }
	    });
	    
	    std::thread::sleep(Duration::from_millis(100));
	    
	    let devices = Self::load_devices(devices_config)?;
	    let scenes = Self::load_scenes(scenes_config)?;
	    
	    if verbose {
	        println!("{} Connected to MQTT broker: {}", "✅".green(), broker);
	        println!("{} Loaded {} devices", "📱".blue(), devices.len());
	        println!("{} Loaded {} scenes", "🎨".purple(), scenes.len());
	        if hue_client.is_some() {
	            println!("{} Hue Bridge connected", "💡".yellow());
	        }
	    }
	    
	    Ok(Self {
	        mqtt_client: mqtt_client_clone,
	        hue_client,
	        devices,
	        scenes,
	        verbose,
	        base_topic,
	    })
	}
	
	fn load_devices(config_path: Option<PathBuf>) -> Result<HashMap<String, DeviceConfig>> {
	    let config_path = config_path.ok_or_else(|| anyhow::anyhow!("No devices config provided"))?;
	    
	    if !config_path.exists() {
	        println!("{} No devices config found, using empty list", "⚠️".yellow());
	        return Ok(HashMap::new());
	    }
	    
	    let devices_json = fs::read_to_string(config_path)
	        .context("Failed to read devices config file")?;
	    
	    let devices: HashMap<String, DeviceConfig> = serde_json::from_str(&devices_json)
	        .map_err(|e| anyhow::anyhow!("Failed to parse devices JSON: {}", e))?;
	    
	    Ok(devices)
	}
	
	fn load_scenes(config_path: Option<PathBuf>) -> Result<HashMap<String, SceneConfig>> {
	    let config_path = config_path.ok_or_else(|| anyhow::anyhow!("No scenes config provided"))?;
	    
	    if !config_path.exists() {
	        println!("{} No scenes config found, using empty list", "⚠️".yellow());
	        return Ok(HashMap::new());
	    }
	    
	    let scenes_json = fs::read_to_string(config_path)
	        .context("Failed to read scenes config file")?;
	    
	    let scenes: HashMap<String, SceneConfig> = serde_json::from_str(&scenes_json)
	        .map_err(|e| anyhow::anyhow!("Failed to parse scenes JSON: {}", e))?;
	    
	    Ok(scenes)
	}
	
    fn publish_message(&mut self, topic: &str, payload: &str) -> Result<()> {
        let json: serde_json::Value = serde_json::from_str(payload)
            .unwrap_or_else(|_| serde_json::Value::String(payload.to_string()));
        self.publish_mqtt(topic, json)
    }
	
    fn control_blinds(&mut self, direction: &str) -> Result<()> {
        let position = match direction.to_lowercase().as_str() {
            "up" | "open"   => 100,
            "down" | "close" => 0,
            _ => anyhow::bail!("Direction must be 'up' or 'down'"),
        };

        let blind_names: Vec<String> = self.devices.values()
            .filter(|d| d.device_type == "blind")
            .map(|d| d.friendly_name.clone())
            .collect();

        if blind_names.is_empty() {
            println!("🦆 No blinds configured.");
            return Ok(());
        }

        println!("Controlling {} blinds: {}…", blind_names.len(), direction);
        for name in &blind_names {
            let topic = format!("{}/{}/set", self.base_topic, name);
            let payload = serde_json::json!({ "position": position });
            self.publish_mqtt(&topic, payload)?;
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    }
	
	fn color_name_to_hue_sat(&self, color_name: &str) -> Result<(u16, u8)> {
	    let mut rng = rand::thread_rng();
	    
	    match color_name.to_lowercase().as_str() {
	        "red" => Ok((rng.gen_range(0..6000), 254)),
	        "orange" => Ok((rng.gen_range(6000..10000), 254)),
	        "yellow" => Ok((rng.gen_range(10000..15000), 254)),
	        "green" => Ok((rng.gen_range(20000..30000), 254)),
	        "cyan" => Ok((rng.gen_range(30000..36000), 254)),
	        "blue" => Ok((rng.gen_range(45000..50000), 254)),
	        "purple" => Ok((rng.gen_range(50000..56000), 254)),
	        "pink" => Ok((rng.gen_range(56000..62000), 150)),
	        "magenta" => Ok((rng.gen_range(58000..65535), 254)),
	        "white" => Ok((rng.gen_range(0..65535), 30)),
	        "gray" | "grey" => Ok((rng.gen_range(0..65535), 60)),
	        "black" => Ok((rng.gen_range(0..65535), 0)),
	        "random" => Ok((rng.gen_range(0..65535), rng.gen_range(0..254))),
	        _ => anyhow::bail!("Unknown color: {}", color_name),
	    }
	}
	
	fn hex_to_xy(&self, hex: &str) -> Result<(f32, f32)> {
	    let hex = hex.trim_start_matches('#');
	    if hex.len() != 6 {
	        anyhow::bail!("Invalid hex color: {}", hex);
	    }
	    
	    let r = u8::from_str_radix(&hex[0..2], 16)? as f32 / 255.0;
	    let g = u8::from_str_radix(&hex[2..4], 16)? as f32 / 255.0;
	    let b = u8::from_str_radix(&hex[4..6], 16)? as f32 / 255.0;
	    
	    let r = if r > 0.04045 {
	        ((r + 0.055) / 1.055).powf(2.4)
	    } else {
	        r / 12.92
	    };
	    
	    let g = if g > 0.04045 {
	        ((g + 0.055) / 1.055).powf(2.4)
	    } else { g / 12.92 };
	    
	    let b = if b > 0.04045 {
	        ((b + 0.055) / 1.055).powf(2.4)
	    } else { b / 12.92 };
	    
	    let x = r * 0.649926 + g * 0.103455 + b * 0.197109;
	    let y = r * 0.234327 + g * 0.743075 + b * 0.022598;
	    let z = r * 0.000000 + g * 0.053077 + b * 1.035763;
	    
	    let sum = x + y + z;
	    if sum == 0.0 {
	        Ok((0.5, 0.4))
	    } else { Ok((x / sum, y / sum)) }
	}
	
fn color_name_to_hex(&self, color_name: &str) -> Result<String> {
 let hex = match color_name.to_lowercase().as_str() {
     "red" => "#FF0000".to_string(),
     "green" => "#00FF00".to_string(),
     "blue" => "#0000FF".to_string(),
     "yellow" => "#FFFF00".to_string(),
     "orange" => "#FFA500".to_string(),
     "purple" => "#800080".to_string(),
     "pink" => "#FFC0CB".to_string(),
     "white" => "#FFFFFF".to_string(),
     "black" => "#000000".to_string(),
     "gray" | "grey" => "#808080".to_string(),
     "brown" => "#A52A2A".to_string(),
     "cyan" => "#00FFFF".to_string(),
     "magenta" => "#FF00FF".to_string(),
     "turquoise" => "#40E0D0".to_string(),
     "teal" => "#008080".to_string(),
     "lime" => "#00FF00".to_string(),
     "maroon" => "#800000".to_string(),
     "olive" => "#808000".to_string(),
     "navy" => "#000080".to_string(),
     "lavender" => "#E6E6FA".to_string(),
     "coral" => "#FF7F50".to_string(),
     "gold" => "#FFD700".to_string(),
     "silver" => "#C0C0C0".to_string(),
     "random" => {
         let mut rng = rand::thread_rng();
         format!("#{:06X}", rng.gen_range(0..0xFFFFFF))
     }
     _ if color_name.starts_with('#') && color_name.len() == 7 => {
         return Ok(color_name.to_string());
     }
     _ => anyhow::bail!("Unknown color: {}", color_name),
 };

 Ok(hex)
}
	
	fn publish_mqtt(&mut self, topic: &str, payload: serde_json::Value) -> Result<()> {
	    let payload_str = serde_json::to_string(&payload)?;
	    
	    if self.verbose {
	        println!("{} {} → {}", "🦆 MQTT".cyan(), topic.blue(), payload_str.yellow());
	    }
	    
	    self.mqtt_client
	        .publish(topic, QoS::AtMostOnce, false, payload_str)
	        .map_err(|e| anyhow::anyhow!("Failed to publish MQTT message: {}", e))?;
	    
	    std::thread::sleep(Duration::from_millis(50));
	    Ok(())
	}
	
fn find_device(&self, query: &str) -> Result<DeviceConfig> {
 let query_lower = query.to_lowercase();

 for device in self.devices.values() {
     if device.friendly_name.to_lowercase() == query_lower {
         return Ok(device.clone());
     }
 }

 for device in self.devices.values() {
     if device.friendly_name.to_lowercase().contains(&query_lower) {
         return Ok(device.clone());
     }
 }

 anyhow::bail!("Device not found: {}", query)
}

	
	fn is_hue_device(&self, device: &DeviceConfig) -> bool {
	    device.hue_id.is_some() && (device.device_type == "light" || device.device_type == "hue_light")
	}
	
fn control_device_with_params(
 &mut self,
 device_name: &str,
 state: &DeviceState,
 brightness: Option<u8>,
 color: Option<String>,
 temperature: Option<u16>,
 transition: Option<f32>,
) -> Result<()> {
 let device = self.find_device(device_name)?.clone();  // Clone the device

 if self.is_hue_device(&device) {
     self.control_hue_device(&device, state, brightness, color, temperature, transition)
 } else {
     self.control_zigbee_device(&device.friendly_name, state, brightness, color, temperature, transition)
 }
}
	
	fn control_hue_device(
	    &mut self,
	    device: &DeviceConfig,
	    state: &DeviceState,
	    brightness: Option<u8>,
	    color: Option<String>,
	    temperature: Option<u16>,
	    transition: Option<f32>,
	) -> Result<()> {
	    let hue_id = device.hue_id.unwrap();
	    let hue_client = self.hue_client.as_ref()
	        .context("Hue client not initialized")?;
	    
	    let mut payload = serde_json::Map::new();
	    
	    match state {
	        DeviceState::On => {
	            payload.insert("on".to_string(), serde_json::Value::Bool(true));
	            
	            if let Some(bri) = brightness {
	                if !(1..=100).contains(&bri) {
	                    anyhow::bail!("Brightness must be between 1-100");
	                }
	                let hue_bri = (bri as f32 * 2.54).round() as u8;
	                if hue_bri > 0 {
	                    payload.insert("bri".to_string(), serde_json::Value::Number(hue_bri.into()));
	                }
	            }
	            
	            if let Some(color_val) = &color {
	                if let Some(temp_val) = temperature {
	                    payload.insert("ct".to_string(), serde_json::Value::Number(temp_val.into()));
	                } else {
	                    let hex = self.color_name_to_hex(color_val)?;
	                    let (hue, sat) = if color_val == "white" {
	                        (0, 0)
	                    } else if color_val == "random" {
	                        let mut rng = rand::thread_rng();
	                        (rng.gen_range(0..65535), rng.gen_range(0..254))
	                    } else if let Ok((h, s)) = self.color_name_to_hue_sat(color_val) {
	                        (h, s)
	                    } else {
	                        let xy = self.hex_to_xy(&hex)?;
	                        payload.insert("xy".to_string(), serde_json::json!([xy.0, xy.1]));
	                        (0, 0)
	                    };
	                    
	                    if hue > 0 || sat > 0 {
	                        payload.insert("hue".to_string(), serde_json::Value::Number(hue.into()));
	                        payload.insert("sat".to_string(), serde_json::Value::Number(sat.into()));
	                    }
	                }
	            } else if let Some(temp_val) = temperature {
	                payload.insert("ct".to_string(), serde_json::Value::Number(temp_val.into()));
	            }
	            
	            if let Some(trans) = transition {
	                let trans_time = (trans * 10.0).round() as u16;
	                payload.insert("transitiontime".to_string(), serde_json::Value::Number(trans_time.into()));
	            }
	        }
	        DeviceState::Off => {
	            payload.insert("on".to_string(), serde_json::Value::Bool(false));
	        }
	        DeviceState::Toggle => {
	            let current = hue_client.get_light_state(hue_id)?;
	            let is_on = current.get("state")
	                .and_then(|s| s.get("on"))
	                .and_then(|o| o.as_bool())
	                .unwrap_or(false);
	            
	            payload.insert("on".to_string(), serde_json::Value::Bool(!is_on));
	        }
	    }
	    
	    let payload_json = serde_json::Value::Object(payload);
	    
	    if self.verbose {
	        println!("{} Hue Light {} (ID: {}) → {}", "💡".yellow(), 
	            device.friendly_name, hue_id, payload_json.to_string());
	    }
	    
	    hue_client.set_light_state(hue_id, payload_json)
	}
	
	fn control_zigbee_device(
	    &mut self,
	    device_name: &str,
	    state: &DeviceState,
	    brightness: Option<u8>,
	    color: Option<String>,
	    temperature: Option<u16>,
	    transition: Option<f32>,
	) -> Result<()> {
	    let mut payload = serde_json::Map::new();
	    
	    match state {
	        DeviceState::On => {
	            payload.insert("state".to_string(), "ON".into());
	            
	            if let Some(bri) = brightness {
	                if !(1..=100).contains(&bri) {
	                    anyhow::bail!("Brightness must be between 1-100");
	                }
	                let mqtt_bri = (bri as f32 * 2.54).round() as u8;
	                payload.insert("brightness".to_string(), mqtt_bri.into());
	            }
	            
	            if let Some(color_val) = &color {
	                let hex = self.color_name_to_hex(color_val)?;
	                payload.insert("color".to_string(), 
	                    serde_json::json!({"hex": hex}));
	            }
	            
	            if let Some(temp_val) = temperature {
	                payload.insert("color_temp".to_string(), temp_val.into());
	            }
	            
	            if let Some(trans) = transition {
	                payload.insert("transition".to_string(), trans.into());
	            }
	        }
	        DeviceState::Off => {
	            payload.insert("state".to_string(), "OFF".into());
	        }
	        DeviceState::Toggle => {
	            payload.insert("state".to_string(), "TOGGLE".into());
	        }
	    }
	    
  	    let topic = format!("{}/{}/set", self.base_topic, device_name);
	    self.publish_mqtt(&topic, serde_json::Value::Object(payload))
	}
	

fn control_device_with_json(
 &mut self,
 device_name: &str,
 json_str: &str,
 backend_type: &BackendType,
) -> Result<()> {
 let device = self.find_device(device_name)?.clone();  // Clone the device
 let payload: serde_json::Value = serde_json::from_str(json_str)
     .context("Failed to parse JSON")?;

 match backend_type {
     BackendType::Auto => {
         if self.is_hue_device(&device) {
             self.control_hue_with_json(&device, payload)
         } else {
             self.control_zigbee_with_json(&device.friendly_name, payload)
         }
     }
     BackendType::Zigbee => {
         self.control_zigbee_with_json(&device.friendly_name, payload)
     }
     BackendType::Hue => {
         self.control_hue_with_json(&device, payload)
     }
 }
}
	
	fn control_hue_with_json(&mut self, device: &DeviceConfig, payload: serde_json::Value) -> Result<()> {
	    let hue_id = device.hue_id.unwrap();
	    let hue_client = self.hue_client.as_ref()
	        .context("Hue client not initialized")?;
	    
	    if self.verbose {
	        println!("{} Hue Light {} (ID: {}) → {}", "💡".yellow(), 
	            device.friendly_name, hue_id, payload.to_string());
	    }
	    
	    hue_client.set_light_state(hue_id, payload)
	}
	
	fn control_zigbee_with_json(&mut self, device_name: &str, payload: serde_json::Value) -> Result<()> {
	    let topic = format!("{}/{}/set", self.base_topic, device_name);
	    self.publish_mqtt(&topic, payload)
	}
	
	fn control_room(
	    &mut self,
	    room_name: &str,
	    state: &DeviceState,
	    brightness: Option<u8>,
	    color: Option<String>,
	    temperature: Option<u16>,
	) -> Result<()> {
	    let device_names: Vec<String> = self.devices
	        .values()
	        .filter(|d| d.room.to_lowercase() == room_name.to_lowercase() && 
	                  (d.device_type == "light" || d.device_type == "hue_light"))
	        .map(|d| d.friendly_name.clone())
	        .collect();

	    if device_names.is_empty() {
	        anyhow::bail!("No lights found in room: {}", room_name);
	    }

	    println!("{} Controlling {} lights in {}", 
	        "💡".green(), device_names.len(), room_name.bold());

	    for device_name in device_names {
	        self.control_device_with_params(
	            &device_name,
	            state,
	            brightness,
	            color.clone(),
	            temperature,
	            None,
	        )?;
	        std::thread::sleep(Duration::from_millis(50));
	    }

	    Ok(())
	}
	
fn activate_scene(&mut self, scene_name: &str, random: bool, room_filter: Option<&str>) -> Result<()> {
    let scene_to_activate = if random {
        let scene_names: Vec<String> = self.scenes.keys().cloned().collect();
        if scene_names.is_empty() {
            anyhow::bail!("No scenes configured");
        }
        let mut rng = rand::thread_rng();
        scene_names.choose(&mut rng).unwrap().clone()
    } else {
        scene_name.to_string()
    };

    println!("{} Activating scene: {}", "🎨".purple(), scene_to_activate.bold());

    let scene = self.scenes
        .get(&scene_to_activate)
        .context(format!("Scene not found: {}", scene_to_activate))?;


    let devices: Vec<(String, serde_json::Value)> = scene.devices
        .iter()
        .filter(|(device_name, _)| {
            if let Some(room) = room_filter {
                self.devices.get(*device_name)
                    .map(|d| d.room.eq_ignore_ascii_case(room))
                    .unwrap_or(false)
            } else {
                true
            }
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if devices.is_empty() {
        let msg = if let Some(room) = room_filter {
            format!("No devices in scene '{}' match room '{}'", scene_to_activate, room)
        } else {
            format!("No devices found in scene: {}", scene_to_activate)
        };
        anyhow::bail!("{}", msg);
    }

    if self.verbose {
        println!("{} Scene '{}' has {} devices (filtered to {})", 
            "🔍".cyan(), scene_to_activate, scene.devices.len(), devices.len());
    }

    let mut hue_count = 0;
    let mut zigbee_count = 0;

    for (device_name, settings) in devices {
        if let Ok(device) = self.find_device(&device_name) {
            if self.is_hue_device(&device) {
                if let Some(hue_client) = &self.hue_client {
                    let hue_id = device.hue_id.unwrap();
                    let hue_payload = self.convert_to_hue_payload(&settings)?;

                    if self.verbose {
                        println!("{} Hue {} (ID: {}) → {}", "💡".yellow(), 
                            device_name, hue_id, hue_payload.to_string());
                    }

                    hue_client.set_light_state(hue_id, hue_payload)?;
                    hue_count += 1;
                } else {
                    println!("{} Skipping Hue device {}: Hue client not initialized", "⚠️".yellow(), device_name);
                }
            } else {
                let topic = format!("{}/{}/set", self.base_topic, device_name);
                if self.verbose {
                    println!("{} MQTT {} → {}", "🦆".cyan(), topic, settings.to_string());
                }
                self.publish_mqtt(&topic, settings)?;
                zigbee_count += 1;
            }
        } else {
            println!("{} Device {} not found", "⚠️".yellow(), device_name);
        }

        std::thread::sleep(Duration::from_millis(10));
    }

    println!("{} Scene '{}' activated ({} Hue, {} Zigbee)", 
        "✅".green(), scene_to_activate, hue_count, zigbee_count);
    Ok(())
}

fn convert_to_hue_payload(&self, settings: &serde_json::Value) -> Result<serde_json::Value> {
let mut payload = serde_json::Map::new();


if let Some(state) = settings.get("state").and_then(|s| s.as_str()) {
payload.insert("on".to_string(), serde_json::Value::Bool(state == "ON"));
} else {
payload.insert("on".to_string(), serde_json::Value::Bool(true));
}


if let Some(brightness) = settings.get("brightness") {
if let Some(bri) = brightness.as_u64() {
   let hue_bri = (bri as f32).min(254.0) as u8;
   if hue_bri > 0 {
       payload.insert("bri".to_string(), serde_json::Value::Number(hue_bri.into()));
   }
} else if let Some(bri) = brightness.as_f64() {
   let hue_bri = (bri as f32).min(254.0) as u8;
   if hue_bri > 0 {
       payload.insert("bri".to_string(), serde_json::Value::Number(hue_bri.into()));
   }
}
}


if let Some(color_obj) = settings.get("color") {
if let Some(xy_array) = color_obj.get("xy") {
   if let Some(xy) = xy_array.as_array() {
       if xy.len() == 2 {
           payload.insert("xy".to_string(), serde_json::json!(xy));
       }
   }
}
}


if let Some(temp) = settings.get("color_temp") {
if let Some(ct) = temp.as_u64() {
   let hue_ct = if ct > 500 { 500 } else if ct < 153 { 153 } else { ct as u16 };
   payload.insert("ct".to_string(), serde_json::Value::Number(hue_ct.into()));
}
}


if let Some(transition) = settings.get("transition") {
if let Some(t) = transition.as_f64() {
   let trans_time = (t * 10.0).round() as u16;
   payload.insert("transitiontime".to_string(), serde_json::Value::Number(trans_time.into()));
} else if let Some(t) = transition.as_u64() {
   let trans_time = (t as f64 * 10.0).round() as u16;
   payload.insert("transitiontime".to_string(), serde_json::Value::Number(trans_time.into()));
}
}

Ok(serde_json::Value::Object(payload))
}
	
	fn enter_pairing_mode(&mut self, duration: u16, watch: bool) -> Result<()> {
	    println!("{} Entering pairing mode for {} seconds...", "📡".blue(), duration);
	    
	    let enable_payload = serde_json::json!({
	        "value": true,
	        "time": duration
	    });
	    
  	    let pairing_topic = format!("{}/bridge/request/permit_join", self.base_topic);
        self.publish_mqtt(&pairing_topic, enable_payload)?;
	    
	    if watch {
	        println!("{} Watching for new devices...", "👀".cyan());
	        println!("{} Put your device in pairing mode now!", "👉".yellow());
	        std::thread::sleep(Duration::from_secs(duration as u64));
	    } else {
	        println!("{} Pairing mode active for {} seconds", "⏰".yellow(), duration);
	        std::thread::sleep(Duration::from_secs(duration as u64));
	    }
	    
	    let disable_payload = serde_json::json!({
	        "value": false
	    });
	    
   	    self.publish_mqtt(&pairing_topic, disable_payload)?;
	    
	    println!("{} Pairing mode finished", "✅".green());
	    Ok(())
	}
	
	fn control_all_lights(&mut self, state: &DeviceState, brightness: Option<u8>, color: Option<String>) -> Result<()> {
	    let device_names: Vec<String> = self.devices
	        .values()
	        .filter(|d| d.device_type == "light" || d.device_type == "hue_light")
	        .map(|d| d.friendly_name.clone())
	        .collect();

	    println!("{} Controlling {} lights", "💡".green(), device_names.len());

	    for device_name in device_names {
	        self.control_device_with_params(
	            &device_name,
	            state,
	            brightness,
	            color.clone(),
	            None,
	            None,
	        )?;
	        std::thread::sleep(Duration::from_millis(50));
	    }    
	    Ok(())
	}
	
	fn cheap_mode(&mut self, room: &str, delay: u64) -> Result<()> {
	    println!("{} Energy saving mode for {} ({} seconds delay)", 
	        "💰".green(), room, delay);
	    
	    self.control_room(room, &DeviceState::On, Some(50), None, None)?;
	    
	    println!("{} Lights on, will turn off in {} seconds...", "⏰".yellow(), delay);
	    
	    std::thread::sleep(Duration::from_secs(delay));
	    
	    self.control_room(room, &DeviceState::Off, None, None, None)?;            
	    println!("{} Lights turned off for energy saving", "✅".green());
	    
	    Ok(())
	}
	
	fn list_items(&self, what: &ListType, json: bool) -> Result<()> {
	    match what {
	        ListType::Devices => {
	            let devices_list: Vec<_> = self.devices.values().collect();
	            if json {
	                println!("{}", serde_json::to_string_pretty(&devices_list)?);
	            } else {
	                println!("\n{} All Devices ({} total):", "📱".blue(), devices_list.len());
	                for device in devices_list {
	                    let hue_info = if device.hue_id.is_some() {
	                        format!(" [Hue ID: {}]", device.hue_id.unwrap())
	                    } else {
	                        "".to_string()
	                    };
	                    println!("  • {} [{}]{}{}", 
	                        device.friendly_name.bold(), 
	                        device.room, 
	                        hue_info,
	                        if device.supports_color.unwrap_or(false) { " 🎨" } else { "" });
	                }
	            }
	        }
	        ListType::Rooms => {
	            let mut rooms = std::collections::HashMap::new();
	            for device in self.devices.values() {
	                *rooms.entry(&device.room).or_insert(0) += 1;
	            }
	            
	            if json {
	                println!("{}", serde_json::to_string_pretty(&rooms)?);
	            } else {
	                println!("\n{} Rooms ({} total):", "🏠".blue(), rooms.len());
	                for (room, count) in rooms {
	                    println!("  • {} ({} devices)", room.bold(), count);
	                }
	            }
	        }
	        ListType::Scenes => {
	            if json {
	                println!("{}", serde_json::to_string_pretty(&self.scenes)?);
	            } else {
	                println!("\n{} Scenes ({} total):", "🎨".purple(), self.scenes.len());
	                for (scene_name, scene) in &self.scenes {
	                    let friendly_name = scene.friendly_name
	                        .as_deref()
	                        .unwrap_or(scene_name);
	                    println!("  • {} ({} devices)", friendly_name.bold(), scene.devices.len());
	                }
	            }
	        }
	        ListType::Lights => {
	            let lights: Vec<_> = self.devices.values()
	                .filter(|d| d.device_type == "light" || d.device_type == "hue_light")
	                .collect();
	            
	            if json {
	                println!("{}", serde_json::to_string_pretty(&lights)?);
	            } else {
	                println!("\n{} Lights ({} total):", "💡".yellow(), lights.len());
	                for light in lights {
	                    let hue_info = if light.hue_id.is_some() {
	                        format!(" [Hue ID: {}]", light.hue_id.unwrap())
	                    } else { "".to_string() };
	                    println!("  • {} [{}]{}", light.friendly_name.bold(), light.room, hue_info);
	                }
	            }
	        }
	        ListType::Sensors => {
	            let sensors: Vec<_> = self.devices.values()
	                .filter(|d| d.device_type.contains("sensor") || 
	                           d.device_type.contains("motion") || 
	                           d.device_type.contains("contact"))
	                .collect();
	            
	            if json {
	                println!("{}", serde_json::to_string_pretty(&sensors)?);
	            } else {
	                println!("\n{} Sensors ({} total):", "📡".cyan(), sensors.len());
	                for sensor in sensors {
	                    println!("  • {} [{}]", sensor.friendly_name.bold(), sensor.room);
	                }
	            }
	        }
	    }
	    
	    Ok(())
	}
}

fn api_list_timers(api_url: &str, password: &str) -> Result<()> {
    let client = HttpClient::new();
    let resp = client
        .get(format!("{}/timers", api_url))
        .header("Authorization", format!("Bearer {}", password))
        .send()
        .context("Failed to reach API")?;
    if !resp.status().is_success() {
        anyhow::bail!("API error: {}", resp.status());
    }
    let body: serde_json::Value = resp.json()?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}

fn api_set_timer(
    api_url: &str,
    password: &str,
    hours: u32,
    minutes: u32,
    seconds: u32,
    topic: &str,
    payload: &str,
    name: Option<&str>,
) -> Result<()> {
    let client = HttpClient::new();
    let mut params = vec![
        ("hours", hours.to_string()),
        ("minutes", minutes.to_string()),
        ("seconds", seconds.to_string()),
        ("topic", topic.to_string()),
        ("payload", payload.to_string()),
    ];
    if let Some(n) = name {
        params.push(("name", n.to_string()));
    }
    let resp = client
        .get(format!("{}/timers/set", api_url))
        .query(&params)
        .header("Authorization", format!("Bearer {}", password))
        .send()
        .context("Failed to reach API")?;
    if !resp.status().is_success() {
        anyhow::bail!("API error: {}", resp.status());
    }
    let body: serde_json::Value = resp.json()?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}

fn api_pause_timer(api_url: &str, password: &str, id: u64) -> Result<()> {
    let client = HttpClient::new();
    let resp = client
        .get(format!("{}/timers/pause", api_url))
        .query(&[("id", id.to_string())])
        .header("Authorization", format!("Bearer {}", password))
        .send()
        .context("Failed to reach API")?;
    if !resp.status().is_success() {
        anyhow::bail!("API error: {}", resp.status());
    }
    let body: serde_json::Value = resp.json()?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}

fn api_resume_timer(api_url: &str, password: &str, id: u64) -> Result<()> {
    let client = HttpClient::new();
    let resp = client
        .get(format!("{}/timers/resume", api_url))
        .query(&[("id", id.to_string())])
        .header("Authorization", format!("Bearer {}", password))
        .send()
        .context("Failed to reach API")?;
    if !resp.status().is_success() {
        anyhow::bail!("API error: {}", resp.status());
    }
    let body: serde_json::Value = resp.json()?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}

fn api_cancel_timer(api_url: &str, password: &str, id: u64) -> Result<()> {
    let client = HttpClient::new();
    let resp = client
        .get(format!("{}/timers/cancel", api_url))
        .query(&[("id", id.to_string())])
        .header("Authorization", format!("Bearer {}", password))
        .send()
        .context("Failed to reach API")?;
    if !resp.status().is_success() {
        anyhow::bail!("API error: {}", resp.status());
    }
    let body: serde_json::Value = resp.json()?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}

fn api_list_alarms(api_url: &str, password: &str) -> Result<()> {
    let client = HttpClient::new();
    let resp = client
        .get(format!("{}/alarms", api_url))
        .header("Authorization", format!("Bearer {}", password))
        .send()
        .context("Failed to reach API")?;
    if !resp.status().is_success() {
        anyhow::bail!("API error: {}", resp.status());
    }
    let body: serde_json::Value = resp.json()?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}

fn api_add_alarm(
    api_url: &str,
    password: &str,
    hours: u8,
    minutes: u8,
    name: &str,
    days: Option<&str>,
    topic: &str,
    payload: &str,
) -> Result<()> {
    let client = HttpClient::new();
    let mut params = vec![
        ("hours", hours.to_string()),
        ("minutes", minutes.to_string()),
        ("name", name.to_string()),
        ("topic", topic.to_string()),
        ("payload", payload.to_string()),
    ];
    if let Some(d) = days {
        if !d.is_empty() {
            params.push(("days", d.to_string()));
        }
    }
    let resp = client
        .get(format!("{}/alarms/add", api_url))
        .query(&params)
        .header("Authorization", format!("Bearer {}", password))
        .send()
        .context("Failed to reach API")?;
    if !resp.status().is_success() {
        anyhow::bail!("API error: {}", resp.status());
    }
    let body: serde_json::Value = resp.json()?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}

fn api_remove_alarm(api_url: &str, password: &str, id: u64) -> Result<()> {
    let client = HttpClient::new();
    let resp = client
        .get(format!("{}/alarms/remove", api_url))
        .query(&[("id", id.to_string())])
        .header("Authorization", format!("Bearer {}", password))
        .send()
        .context("Failed to reach API")?;
    if !resp.status().is_success() {
        anyhow::bail!("API error: {}", resp.status());
    }
    let body: serde_json::Value = resp.json()?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}

fn api_toggle_alarm(api_url: &str, password: &str, id: u64) -> Result<()> {
    let client = HttpClient::new();
    let resp = client
        .get(format!("{}/alarms/toggle", api_url))
        .query(&[("id", id.to_string())])
        .header("Authorization", format!("Bearer {}", password))
        .send()
        .context("Failed to reach API")?;
    if !resp.status().is_success() {
        anyhow::bail!("API error: {}", resp.status());
    }
    let body: serde_json::Value = resp.json()?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}



fn fetch_and_print_status_table(
    api_url: Option<&str>,
    api_password: &str,
    state_file: Option<&Path>,
    verbose: bool,
) -> Result<()> {
    let json_str = if let Some(path) = state_file {
        fs::read_to_string(path).context("Failed to read state file")?
    } else if let Some(base_url) = api_url {
        let client = reqwest::blocking::Client::new();
        let url = format!("{}/state", base_url.trim_end_matches('/'));
        if verbose {
            println!("Fetching state from API: {}", url);
        }
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_password))
            .send()
            .context("Failed to reach API")?;
        if !resp.status().is_success() {
            anyhow::bail!("API returned {}", resp.status());
        }
        resp.text()?
    } else { anyhow::bail!("No state source available (set --api-url or --state-file)"); };

    let data: HashMap<String, serde_json::Value> =
        serde_json::from_str(&json_str).context("Invalid state JSON")?;

    fn json_to_number(v: &serde_json::Value) -> Option<f64> {
        v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    }

    let mut rows = Vec::new();

    for (device_name, props) in &data {
        let state_display = if let Some(state) = props.get("state").and_then(|v| v.as_str()) {
            match state {
                "ON" => Some("ON"),
                "OFF" => Some("OFF"),
                _ => None,
            }
        } else if let Some(pos) = props.get("position").and_then(json_to_number) {
            if (pos - 100.0).abs() < f64::EPSILON {
                Some("OPEN")
            } else { None }
            
        } else if let Some(contact) = props.get("contact") {
            let is_closed = contact
                .as_bool()
                .or_else(|| contact.as_str().map(|s| s.eq_ignore_ascii_case("true")));
            match is_closed {
                Some(true) => Some("CLOSED"),
                Some(false) => Some("OPEN"),
                None => None,
            }
        } else { None };

        let battery_str = props
            .get("battery")
            .and_then(json_to_number)
            .map(|b| {
                let icon = if b > 40.0 { "🔋" } else { "🪫" };
                format!("{} {:.0}%", icon, b)
            })
            .unwrap_or_default();

        let temp_str = props
            .get("temperature")
            .and_then(json_to_number)
            .map(|t| format!("{:.2}°C", t))
            .unwrap_or_default();

        if state_display.is_none() && battery_str.is_empty() && temp_str.is_empty() {
            continue;
        }

        rows.push((
            state_display.unwrap_or("").to_string(),
            device_name.clone(),
            battery_str,
            temp_str,
        ));
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("State").add_attribute(Attribute::Bold),
            Cell::new("Device").add_attribute(Attribute::Bold),
            Cell::new("Battery").add_attribute(Attribute::Bold),
            Cell::new("Temperature").add_attribute(Attribute::Bold),
        ]);

    for (state, device, battery, temp) in &rows {
        table.add_row(vec![
            Cell::new(state),
            Cell::new(device),
            Cell::new(battery),
            Cell::new(temp),
        ]);
    }

    println!("{table}");
    Ok(())
}




fn main() -> Result<()> {
    let debug = std::env::var("DEBUG").is_ok();
    if debug { std::env::set_var("DT_LOG_LEVEL", "DEBUG"); }
    dt_setup(None, None);
    dt_debug!("Started zigduck-cli!");    

	let mut cli = Cli::parse();
	
    let default_config_path = PathBuf::from("/etc/zigduck/config.json");
    let config: Option<CliConfig> = if default_config_path.exists() {
        match fs::read_to_string(&default_config_path) {
            Ok(content) => serde_json::from_str(&content).ok(),
            Err(_) => None,
        }
    } else { None };

    if let Some(cfg) = &config {
        if cli.broker == "127.0.0.1" && !std::env::var("MQTT_BROKER").is_ok() {
            if let Some(mosq) = &cfg.mosquitto {
                cli.broker = mosq.broker.clone();
            }
        }
        if cli.user == "mqtt" && !std::env::var("MQTT_USER").is_ok() {
            if let Some(mosq) = &cfg.mosquitto {
                cli.user = mosq.user.clone();
            }
        }
        if cli.password.is_none() && cli.password_file.is_none() {
            if let Some(mosq) = &cfg.mosquitto {
                if let Some(pw_file) = &mosq.password_file {
                    cli.password_file = Some(pw_file.into());
                }
            }
        }
        if cli.hue_bridge_ip.is_none() && !std::env::var("HUE_BRIDGE_IP").is_ok() {
            if let Some(hue) = &cfg.hue {
                cli.hue_bridge_ip = hue.bridge_ip.clone();
            }
        }
        if cli.hue_api_key.is_none() {
            if let Some(hue) = &cfg.hue {
                if let Some(pw_file) = &hue.password_file {
                    if let Ok(key) = fs::read_to_string(pw_file) {
                        cli.hue_api_key = Some(key.trim().to_string());
                    }
                }
            }
        }
    }

    let api_url = if let Some(url) = cli.api_url.clone() {
        url
    } else if let Some(cfg) = &config {
        cfg.api.as_ref()
            .and_then(|a| a.url.clone())
            .unwrap_or_else(|| "http://192.168.1.211:13335".to_string())
    } else { "http://192.168.1.211:13335".to_string() };

    let api_password = if let Some(pw) = cli.api_password.clone() {
        pw
    } else if let Some(pf) = &cli.api_password_file {
        fs::read_to_string(pf)?.trim().to_string()
    } else if let Some(cfg) = &config {
        cfg.api.as_ref()
            .and_then(|a| a.password_file.as_ref())
            .and_then(|pf| fs::read_to_string(pf).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    if cli.devices_config.is_none() && !std::env::var("DEVICES_CONFIG").is_ok() {
        let default_devices = PathBuf::from("/etc/zigduck/devices.json");
        if default_devices.exists() {
            cli.devices_config = Some(default_devices);
        }
    }

    if cli.scenes_config.is_none() && !std::env::var("SCENES_CONFIG").is_ok() {
        let default_scenes = PathBuf::from("/etc/zigduck/scenesCLI.json");
        if default_scenes.exists() {
            cli.scenes_config = Some(default_scenes);
        }
    }

    let password = if let Some(password_file) = cli.password_file {
        fs::read_to_string(password_file)?.trim().to_string()
    } else if let Some(password) = cli.password {
        password
    } else if let Ok(password) = std::env::var("MQTT_PASSWORD") {
        password
    } else { "".to_string() };

    let hue_api_key = if let Some(key_file) = cli.hue_key_file {
        Some(fs::read_to_string(key_file)?.trim().to_string())
    } else { cli.hue_api_key.or_else(|| std::env::var("HUE_API_KEY").ok()) };

    if cli.status {
        fetch_and_print_status_table(
            Some(&api_url),
            &api_password,
            cli.state_file.as_deref(),
            cli.verbose > 0,
        )?;
        return Ok(());
    }

    let base_topic = if let Some(cfg) = &config {
        cfg.mosquitto.as_ref()
            .map(|m| m.base_topic.clone())
            .unwrap_or_else(|| "zigbee2mqtt".to_string())
    } else { "zigbee2mqtt".to_string() };

    let mut controller = ZigduckController::new(
        cli.broker, cli.user, password,
        cli.hue_bridge_ip, hue_api_key,
        cli.devices_config, cli.scenes_config,
        cli.verbose > 0,
        base_topic.clone(),
    )?;

    
    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Timer { action } => match action {
                TimerAction::List => api_list_timers(&api_url, &api_password)?,
                TimerAction::Set { hours, minutes, seconds, topic, payload, name } => {
                    let h = hours.unwrap_or(0);
                    let m = minutes.unwrap_or(0);
                    let s = seconds.unwrap_or(0);
                    if h == 0 && m == 0 && s == 0 {
                        anyhow::bail!("At least one of hours/minutes/seconds must be positive");
                    }
                    let topic = topic.unwrap_or_else(|| format!("{}/timer/finished", base_topic));
                    let payload = payload.unwrap_or_else(|| r#"{"status":"finished"}"#.to_string());
                    api_set_timer(&api_url, &api_password, h, m, s, &topic, &payload, name.as_deref())?;
                }
                TimerAction::Pause { id } => api_pause_timer(&api_url, &api_password, id)?,
                TimerAction::Resume { id } => api_resume_timer(&api_url, &api_password, id)?,
                TimerAction::Cancel { id } => api_cancel_timer(&api_url, &api_password, id)?,
            },
            Commands::Alarm { action } => match action {
                AlarmAction::List => api_list_alarms(&api_url, &api_password)?,
                AlarmAction::Add { hours, minutes, name, days, topic, payload } => {
                    let topic = topic.unwrap_or_else(|| format!("{}/alarm/triggered", base_topic));
                    let payload = payload.unwrap_or_else(|| r#"{"alarm":"triggered"}"#.to_string());
                    api_add_alarm(
                        &api_url,
                        &api_password,
                        hours,
                        minutes,
                        &name,
                        days.as_deref(),
                        &topic,
                        &payload,
                    )?;
                }
                AlarmAction::Remove { id } => api_remove_alarm(&api_url, &api_password, id)?,
                AlarmAction::Toggle { id } => api_toggle_alarm(&api_url, &api_password, id)?,
            },
        }
        return Ok(());
    }
    

    fn parse_state(state_str: &str, brightness: &mut Option<u8>, color: &mut Option<String>) -> Result<DeviceState> {
        match state_str.to_lowercase().as_str() {
            "on" => Ok(DeviceState::On),
            "off" => Ok(DeviceState::Off),
            "toggle" => Ok(DeviceState::Toggle),
            "max" => {
                if brightness.is_none() {
                    *brightness = Some(100);
                }
                if color.is_none() {
                    *color = Some("white".to_string());
                }
                Ok(DeviceState::On)
            }
            "dark" => {
                Ok(DeviceState::Off)
            }
            _ => anyhow::bail!("Invalid state: {}. Must be on/off/toggle/max/dark", state_str),
        }
    }

    fn all_lights_enabled(val: &Option<String>) -> bool {
        match val {
            None => false,
            Some(s) => {
                let s = s.to_lowercase();
                !(s == "false" || s == "0" || s == "no" || s == "off")
            }
        }
    }


    if let Some(device_name) = cli.device {
        let state_str = cli.state.as_deref().context("--state is required for device")?;
        let mut brightness = cli.brightness;
        let mut color = cli.color;
        let state = parse_state(state_str, &mut brightness, &mut color)?;
        controller.control_device_with_params(
            &device_name,
            &state,
            brightness,
            color,
            cli.temperature,
            cli.transition,
        )
    } else if let Some(scene_name) = cli.scene {
        controller.activate_scene(
            &scene_name,
            cli.random,
            cli.room.as_deref()
        )?;
        Ok(())
    } else if let Some(room_name) = cli.room {
        let state_str = cli.state.as_deref().context("--state is required for room")?;
        let mut brightness = cli.brightness;
        let mut color = cli.color;
        let state = parse_state(state_str, &mut brightness, &mut color)?;
        controller.control_room(
            &room_name,
            &state,
            brightness,
            color,
            cli.temperature,
        )
    } else if let Some(list_arg) = cli.list {
        let what = list_arg.unwrap_or(ListType::Devices);
        controller.list_items(&what, cli.json_output)?;
        Ok(())
    } else if let Some(pair_arg) = cli.pair {
        let duration = pair_arg.unwrap_or(120);
        controller.enter_pairing_mode(duration, cli.watch)
    } else if all_lights_enabled(&cli.all_lights) {
        let state_str = cli.state.as_deref().context("--state is required for all-lights")?;
        let mut brightness = cli.brightness;
        let mut color = cli.color;
        let state = parse_state(state_str, &mut brightness, &mut color)?;
        controller.control_all_lights(&state, brightness, color)
    } else if let Some(room_name) = cli.cheap_mode {
        controller.cheap_mode(&room_name, cli.delay)        
    } else if let Some(direction) = cli.blinds {
        controller.control_blinds(&direction)?;
        return Ok(());
    } else if cli.publish {
        let topic = cli.topic.as_deref().context("--topic required for --publish")?;
        let payload = cli.payload.as_deref().context("--payload required for --publish")?;
        controller.publish_message(topic, payload)?;
        return Ok(());
    } else if cli.json_cmd {
        let device_name = cli.device.context("--device is required for JSON command")?;
        let payload = cli.payload.context("--payload is required for JSON command")?;
        controller.control_device_with_json(&device_name, &payload, &cli.backend)
    } else {
        eprintln!("{} No action specified. Use --help for usage.", "🦆".cyan());
        Ok(())
    }
}
