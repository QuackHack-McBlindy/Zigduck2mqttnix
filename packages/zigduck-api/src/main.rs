use std::io::{BufRead, BufReader, Read};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::fs::{self, create_dir_all};
use std::env;
use std::io::Write;
use std::sync::{Arc, Mutex, Condvar};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};


use serde::{Serialize, Deserialize};
use serde_json::{Value, json, Map};
use rand::seq::SliceRandom;
use ducktrace_logger::*;
use lazy_static::lazy_static;


fn log(message: &str) {
    eprintln!("[API] {}", message);
}

#[derive(Debug, Serialize, Deserialize)]
struct TvDefaults {
    directories: Directories,
    playlist_file: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Directories {
    root: Option<String>,
}

lazy_static! {
    static ref CONFIG: (String, String) = {
        let config_path = "/etc/zigduck/tv-defaults.json";
        let default_root = "/Pool".to_string();
        let default_playlist = "/Pool/playlist.m3u".to_string();

        match fs::read_to_string(config_path) {
            Ok(content) => {
                let config: TvDefaults = serde_json::from_str(&content).unwrap_or_else(|e| {
                    log(&format!("Failed to parse config, using defaults: {}", e));
                    TvDefaults {
                        directories: Directories { root: Some(default_root.clone()) },
                        playlist_file: Some(default_playlist.clone()),
                    }
                });
                let root = config.directories.root.unwrap_or(default_root);
                let playlist = config.playlist_file.unwrap_or(default_playlist);
                (root, playlist)
            }
            Err(e) => {
                log(&format!("Could not read config file {}: {}; using defaults", config_path, e));
                (default_root, default_playlist)
            }
        }
    };
}

fn get_playlist_path() -> &'static str {
    &CONFIG.1
}

fn get_root_dir() -> &'static str {
    &CONFIG.0
}


lazy_static! {
    static ref TIMER_MANAGER: Arc<TimerManager> = TimerManager::new();
}

lazy_static! {
    static ref PLAYLIST_MUTEX: Mutex<()> = Mutex::new(());
}

use chrono::{Local, Datelike, Timelike, NaiveTime};

const ALARMS_FILE: &str = "/var/lib/zigduck/alarms.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Alarm {
    id: u64,
    name: String,
    hour: u8,
    minute: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    days: Option<Vec<u8>>,
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_fired: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    action: Option<AlarmAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AlarmAction {
    topic: String,
    payload: String,
}

struct AlarmManager {
    next_id: AtomicU64,
    alarms: Mutex<Vec<Alarm>>,
    condvar: Condvar,
}

impl AlarmManager {
    fn new() -> Arc<Self> {
        let alarms: Vec<Alarm> = match fs::read_to_string(ALARMS_FILE) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        let max_id = alarms.iter().map(|a| a.id).max().unwrap_or(0);
        Arc::new(Self {
            next_id: AtomicU64::new(max_id + 1),
            alarms: Mutex::new(alarms),
            condvar: Condvar::new(),
        })
    }

    fn save_to_file(&self) {
        let alarms = self.alarms.lock().unwrap();
        if let Ok(json) = serde_json::to_string_pretty(&*alarms) {
            let _ = fs::create_dir_all("/var/lib/zigduck");
            let _ = fs::write(ALARMS_FILE, json);
        }
    }

    fn add(
        &self,
        hour: u8,
        minute: u8,
        days: Option<Vec<u8>>,
        name: String,
        topic: Option<String>,
        payload: Option<String>,
    ) -> u64 {
        let action = match (topic, payload) {
            (Some(t), Some(p)) => Some(AlarmAction { topic: t, payload: p }),
            _ => None,
        };
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let alarm = Alarm {
            id,
            name,
            hour,
            minute,
            days,
            enabled: true,
            last_fired: None,
            action,
        };
        self.alarms.lock().unwrap().push(alarm);
        self.save_to_file();
        self.condvar.notify_one();
        id
    }

    fn remove(&self, id: u64) -> Result<Alarm, String> {
        let mut alarms = self.alarms.lock().unwrap();
        if let Some(pos) = alarms.iter().position(|a| a.id == id) {
            let alarm = alarms.remove(pos);
            self.save_to_file();
            self.condvar.notify_one();
            Ok(alarm)
        } else {
            Err("Alarm not found".into())
        }
    }

    fn toggle(&self, id: u64) -> Result<Alarm, String> {
        let mut alarms = self.alarms.lock().unwrap();
        if let Some(alarm) = alarms.iter_mut().find(|a| a.id == id) {
            alarm.enabled = !alarm.enabled;
            let cloned = alarm.clone();
            self.save_to_file();
            self.condvar.notify_one();
            Ok(cloned)
        } else {
            Err("Alarm not found".into())
        }
    }

    fn list(&self) -> Vec<Alarm> {
        self.alarms.lock().unwrap().clone()
    }


    fn next_trigger_time(&self) -> Option<NaiveTime> {
        let now = Local::now();
        let today = now.date_naive();
        let current_time = now.time();

        self.alarms.lock().unwrap()
            .iter()
            .filter(|a| a.enabled)
            .filter_map(|a| {
                let alarm_time = NaiveTime::from_hms_opt(a.hour as u32, a.minute as u32, 0)?;
                if alarm_time > current_time {
                    Some(alarm_time)
                } else {
                    None
                }
            })
            .min()
    }
}

lazy_static! {
    static ref ALARM_MANAGER: Arc<AlarmManager> = AlarmManager::new();
}

fn get_mqtt_credentials() -> Option<(String, String)> {
    let user = std::env::var("MQTT_USER").ok()?;
    let password_file = std::env::var("MQTT_PASSWORD_FILE").ok()?;
    let password = std::fs::read_to_string(&password_file)
        .map(|s| s.trim().to_owned())
        .ok()?;
    Some((user, password))
}


fn start_timer_thread(manager: Arc<TimerManager>) {
    std::thread::spawn(move || {
        loop {
            let (next_fire, has_timers) = {
                let timers = manager.timers.lock().unwrap();
                let next = timers.values()
                    .map(|t| t.fire_at)
                    .min();
                (next, !timers.is_empty())
            };

            if let Some(deadline) = next_fire {
                let now = Instant::now();
                if deadline <= now {
                    let mut timers = manager.timers.lock().unwrap();
                    let due_ids: Vec<TimerId> = timers
                        .iter()
                        .filter(|(_, t)| t.fire_at <= now && t.paused_remaining.is_none())
                        .map(|(id, _)| *id)
                        .collect();

                    for id in due_ids {
                        if let Some(timer) = timers.remove(&id) {
                            if let TimerAction::MqttMessage { topic, payload } = &timer.action {
                                let mut cmd_action = std::process::Command::new("mosquitto_pub");
                                cmd_action
                                    .arg("-t").arg(topic)
                                    .arg("-m").arg(payload);

                                if let Some((user, pass)) = get_mqtt_credentials() {
                                    cmd_action.arg("-u").arg(user).arg("-P").arg(pass);
                                }
                                if let Ok(broker) = std::env::var("MQTT_BROKER") {
                                    if broker != "localhost" && broker != "127.0.0.1" {
                                        cmd_action.arg("-h").arg(broker);
                                    }
                                }

                                match cmd_action.output() {
                                    Ok(out) if !out.status.success() => {
                                        dt_error(&format!(
                                            "mosquitto_pub (action) failed: {}",
                                            String::from_utf8_lossy(&out.stderr)
                                        ));
                                    }
                                    Ok(_) => {}
                                    Err(e) => dt_error(&format!("mosquitto_pub (action) spawn error: {}", e)),
                                }
                            }
                        }
                    }
                    continue;
                } else {
                    let sleep_time = deadline - now;
                    let (_timers, _result) = manager.condvar
                        .wait_timeout(manager.timers.lock().unwrap(), sleep_time)
                        .unwrap();
                }
            } else {
                let _guard = manager.condvar.wait(manager.timers.lock().unwrap());
                drop(_guard);
            }
        }
    });
}


type TimerId = u64;

#[derive(Debug, Clone)]
struct Timer {
    id: TimerId,
    name: String,
    fire_at: Instant,
    duration: Duration,
    paused_remaining: Option<Duration>,
    action: TimerAction,
}

#[derive(Debug, Clone)]
enum TimerAction {
    MqttMessage {
        topic: String,
        payload: String,
    },
}

struct TimerManager {
    next_id: AtomicU64,
    timers: Mutex<HashMap<TimerId, Timer>>,
    condvar: Condvar,
}

impl TimerManager {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            next_id: AtomicU64::new(1),
            timers: Mutex::new(HashMap::new()),
            condvar: Condvar::new(),
        })
    }

    fn add(
        &self,
        hours: u32,
        minutes: u32,
        seconds: u32,
        action: TimerAction,
        name: String,
    ) -> TimerId {
        let total_secs = hours * 3600 + minutes * 60 + seconds;
        let duration = Duration::from_secs(total_secs as u64);
        let fire_at = Instant::now() + duration;

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let timer = Timer {
            id,
            name,
            fire_at,
            duration,
            paused_remaining: None,
            action,
        };

        let mut timers = self.timers.lock().unwrap();
        timers.insert(id, timer);
        drop(timers);

        self.condvar.notify_one();
        id
    }

    fn pause(&self, id: TimerId) -> Result<(), String> {
        let mut timers = self.timers.lock().unwrap();
        if let Some(timer) = timers.get_mut(&id) {
            if timer.paused_remaining.is_some() {
                return Err("Timer already paused".into());
            }
            let remaining = timer.fire_at.saturating_duration_since(Instant::now());
            timer.paused_remaining = Some(remaining);
            timer.fire_at = Instant::now() + Duration::from_secs(60 * 60 * 24);
            drop(timers);
            self.condvar.notify_one();
            Ok(())
        } else {
            Err("Timer not found".into())
        }
    }

    fn resume(&self, id: TimerId) -> Result<(), String> {
        let mut timers = self.timers.lock().unwrap();
        if let Some(timer) = timers.get_mut(&id) {
            if let Some(remaining) = timer.paused_remaining.take() {
                timer.fire_at = Instant::now() + remaining;
                drop(timers);
                self.condvar.notify_one();
                Ok(())
            } else {
                Err("Timer not paused".into())
            }
        } else {
            Err("Timer not found".into())
        }
    }

    fn cancel(&self, id: TimerId) -> Result<Timer, String> {
        let mut timers = self.timers.lock().unwrap();
        if let Some(timer) = timers.remove(&id) {
            drop(timers);
            self.condvar.notify_one();
            Ok(timer)
        } else {
            Err("Timer not found".into())
        }
    }

    fn list(&self) -> Vec<Timer> {
        let timers = self.timers.lock().unwrap();
        timers.values().cloned().collect()
    }
}



fn handle_transcode_video_stream(url: &str, stream: &mut std::net::TcpStream) -> Result<(), String> {
    dt_info(&format!("Streaming transcoded video from URL: {}", url));

    let mut child = std::process::Command::new("bash")
        .arg("-c")
        .arg(format!("curl -s -L '{}' | ffmpeg -i pipe:0 -f mp4 -movflags frag_keyframe+empty_moov -preset ultrafast -c:v libx264 -c:a aac -b:a 192k -", url))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Error starting transcoding pipeline: {}", e))?;

    let mut stdout = child.stdout.take()
        .ok_or("Failed to capture ffmpeg stdout".to_string())?;

    // 🦆 says ⮞ send http headers
    stream.write_all(b"HTTP/1.1 200 OK\r\n")
        .map_err(|e| format!("Failed to write status line: {}", e))?;
    stream.write_all(b"Content-Type: video/mp4\r\n")
        .map_err(|e| format!("Failed to write content type: {}", e))?;
    stream.write_all(b"Access-Control-Allow-Origin: *\r\n")
        .map_err(|e| format!("Failed to write CORS header: {}", e))?;
    stream.write_all(b"Transfer-Encoding: chunked\r\n")
        .map_err(|e| format!("Failed to write transfer encoding: {}", e))?;
    stream.write_all(b"\r\n")
        .map_err(|e| format!("Failed to write header terminator: {}", e))?;

    // 🦆 says ⮞ stream video data in chunks
    let mut buffer = [0; 8192];
    loop {
        match stdout.read(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(n) => {
                // 🦆 says ⮞ ciao chunk
                let chunk_header = format!("{:x}\r\n", n);
                stream.write_all(chunk_header.as_bytes())
                    .map_err(|e| format!("Failed to write chunk header: {}", e))?;
                stream.write_all(&buffer[..n])
                    .map_err(|e| format!("Failed to write chunk data: {}", e))?;
                stream.write_all(b"\r\n")
                    .map_err(|e| format!("Failed to write chunk terminator: {}", e))?;
            }
            Err(e) => {
                dt_warning(&format!("Error reading from ffmpeg: {}", e));
                break;
            }
        }
    }

    // 🦆 says ⮞ send bye-bye goodnight chunk
    stream.write_all(b"0\r\n\r\n")
        .map_err(|e| format!("Failed to write final chunk: {}", e))?;

    let _ = child.wait();

    dt_info("Video streaming completed");
    Ok(())
}

fn get_device_ip(query: &str) -> String {
    let ip = get_query_arg(query, "device");
    if ip.is_empty() {
        "192.168.1.224".to_string()
    } else {
        ip
    }
}

fn execute_adb(device_ip: &str, args: &[&str]) -> Result<(), String> {
    let mut cmd = Command::new("adb");
    cmd.arg("-s").arg(device_ip);
    cmd.args(args);
    let output = cmd.output().map_err(|e| format!("Failed to run adb: {}", e))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

fn read_webserver_url() -> Result<String, String> {
    let path = std::env::var("WEBSERVER_SECRET_FILE")
        .unwrap_or_else(|_| "/run/secrets/webserver".to_string());
    std::fs::read_to_string(&path)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("Cannot read webserver URL: {}", e))
}

// 🦆 says ⮞ Password authentication function
fn check_password_auth(headers: &HashMap<String, String>, query: &str) -> bool {
    let password_file_path = match std::env::var("API_PASSWORD_FILE") {
        Ok(path) => path,
        Err(_) => {
            log("API_PASSWORD_FILE not set, authentication failed");
            return false;
        }
    };

    let expected_password = match std::fs::read_to_string(&password_file_path) {
        Ok(content) => content.trim().to_string(),
        Err(_) => {
            log(&format!("Warning: Could not read password file: {}", password_file_path));
            return false;
        }
    };

    if let Some(auth_header) = headers.get("authorization") {
        if auth_header.starts_with("Bearer ") {
            let provided_password = auth_header[7..].trim();
            return provided_password == expected_password;
        } else if auth_header.starts_with("Password ") {
            let provided_password = auth_header[9..].trim();
            return provided_password == expected_password;
        }
    }

    //let query_password = get_query_arg(query, "password");
    //if !query_password.is_empty() && query_password == expected_password {
    //    return true;
    //}

    if let Some(api_key) = headers.get("x-api-key") {
        return api_key.trim() == expected_password;
    }

    false
}
    
fn urldecode(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.bytes().collect::<Vec<_>>();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let (Some(high), Some(low)) = (from_hex(bytes[i + 1]), from_hex(bytes[i + 2])) {
                    let byte = (high << 4) | low;
                    result.push(byte);
                    i += 3;
                    continue;
                }
            }
            b'+' => {
                result.push(b' ');
            }
            _ => {
                result.push(bytes[i]);
            }
        }
        i += 1;
    }

    String::from_utf8(result).unwrap_or_else(|_| s.to_string())
}

fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn send_response(stream: &mut TcpStream, status: &str, body: &str, content_type: Option<&str>) {
    let content_type = content_type.unwrap_or("application/json");
    let response = format!(
        "HTTP/1.1 {}\r\n\
         Content-Type: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: Authorization, Content-Type, X-API-Key\r\n\
         Content-Length: {}\r\n\r\n{}",
        status,
        content_type,
        body.len(),
        body
    );
    if let Err(e) = stream.write_all(response.as_bytes()) {
        dt_warning(&format!("Failed to send response: {}", e));
    }
}

fn get_query_arg(query: &str, arg_name: &str) -> String {
    let parts: Vec<&str> = query.split('&').collect();
    for part in parts {
        if part.starts_with(&format!("{}=", arg_name)) {
            let encoded = &part[arg_name.len() + 1..];
            return urldecode(encoded);
        }
    }
    String::new()
}

fn get_path_arg(query: &str) -> String {
    let parts: Vec<&str> = query.split('&').collect();
    for part in parts {
        if part.starts_with("path=") {
            let encoded = &part[5..];
            return urldecode(encoded);
        }
    }
    String::new()
}

// 🦆 says ⮞ read zigduck state.json
fn handle_state_all() -> String {
    let state_file_path = "/var/lib/zigduck/state.json";
    match fs::read_to_string(state_file_path) {
        Ok(content) => {
            dt_info("Returning full state.json");
            content
        }
        Err(e) => {
            dt_error(&format!("Failed to read state file: {}", e));
            r#"{"error":"Failed to read state file"}"#.to_string()
        }
    }
}

fn handle_state_device(device_name: &str) -> String {
    let state_file_path = "/var/lib/zigduck/state.json";
    match fs::read_to_string(state_file_path) {
        Ok(content) => {
            let state_data: serde_json::Value = serde_json::from_str(&content)
                .unwrap_or_else(|_| json!({}));
            
            if let Some(device_state) = state_data.get(device_name) {
                dt_info(&format!("Returning state for device: {}", device_name));
                device_state.to_string()
            } else {
                dt_warning(&format!("Device not found in state: {}", device_name));
                r#"{"error":"Device not found in state"}"#.to_string()
            }
        }
        Err(e) => {
            dt_error(&format!("Failed to read state file: {}", e));
            r#"{"error":"Failed to read state file"}"#.to_string()
        }
    }
}

fn handle_state_room(room: &str) -> String {
    let state_file_path = "/var/lib/zigduck/state.json";
    let devices_file = "devices.json";
    
    match fs::read_to_string(state_file_path) {
        Ok(content) => {
            let state_data: Value = serde_json::from_str(&content)
                .unwrap_or_else(|_| json!({}));
            
            // 🦆 says ⮞ load devices to filter by room
            let devices_content = fs::read_to_string(devices_file)
                .unwrap_or_else(|_| "{}".to_string());
            let devices: Map<String, Value> = 
                serde_json::from_str(&devices_content).unwrap_or_else(|_| Map::new());
            
            let mut room_devices = Map::new();
            
            let empty_map = Map::new();
            for (device_name, device_state) in state_data.as_object().unwrap_or(&empty_map) {
                if let Some(device_info) = devices.get(device_name) {
                    if let Some(device_room) = device_info.get("room") {
                        if let Some(room_str) = device_room.as_str() {
                            if room_str.to_lowercase() == room.to_lowercase() {
                                room_devices.insert(device_name.clone(), device_state.clone());
                            }
                        }
                    }
                }
            }
            
            dt_info(&format!("Returning state for room: {} ({} devices)", room, room_devices.len()));
            serde_json::to_string(&room_devices).unwrap_or_else(|_| "{}".to_string())
        }
        Err(e) => {
            dt_error(&format!("Failed to read state file: {}", e));
            r#"{"error":"Failed to read state file"}"#.to_string()
        }
    }
} 

fn handle_browse(path_arg: &str, use_v2: bool) -> String {
    let media_root = get_root_dir();
    let full_path = format!("{}/{}", media_root, path_arg);
    
    // 🦆 says ⮞ safety first!
    if !full_path.starts_with(media_root) {
        dt_warning(&format!("Access forbidden for path: {}", path_arg));
        return r#"{"error":"Access forbidden"}"#.to_string();
    }

    let path_std = std::path::Path::new(&full_path);
    if !path_std.exists() || !path_std.is_dir() {
        return format!(r#"{{"error":"Directory not found: {}"}}"#, path_arg);
    }

    let mut directories = Vec::new();
    let mut files = Vec::new();

    if use_v2 {
        // 🦆 says ⮞ browsev2 with find
        let output = Command::new("find")
            .arg(&full_path)
            .arg("-maxdepth")
            .arg("1")
            .arg("-mindepth")
            .arg("1")
            .output();
        
        match output {
            Ok(output) if output.status.success() => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if line.is_empty() { continue; }
                    let item_path = std::path::Path::new(line);
                    if let Some(name) = item_path.file_name().and_then(|n| n.to_str()) {
                        if item_path.is_dir() {
                            directories.push(name.to_string());
                        } else {
                            files.push(name.to_string());
                        }
                    }
                }
            }
            _ => return r#"{"error":"Failed to list directory"}"#.to_string(),
        }
    } else {
        // 🦆 says ⮞ browse logic with ls
        let output = Command::new("ls")
            .arg("-1")
            .arg(&full_path)
            .output();
        
        match output {
            Ok(output) if output.status.success() => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for item in output_str.lines() {
                    if item.is_empty() { continue; }
                    let item_path = path_std.join(item);
                    if item_path.is_dir() {
                        directories.push(item.to_string());
                    } else {
                        files.push(item.to_string());
                    }
                }
            }
            _ => return r#"{"error":"Failed to list directory"}"#.to_string(),
        }
    }

    directories.sort();
    files.sort();

    let dirs_json = serde_json::to_string(&directories).unwrap_or_else(|_| "[]".to_string());
    let files_json = serde_json::to_string(&files).unwrap_or_else(|_| "[]".to_string());

    if use_v2 {
        let real_full_path = path_std.canonicalize().unwrap_or_else(|_| path_std.to_path_buf());
        format!(
            r#"{{"path":"{}","full_path":"{}","directories":{},"files":{}}}"#,
            path_arg,
            real_full_path.display(),
            dirs_json,
            files_json
        )
    } else {
        format!(
            r#"{{"path":"{}","directories":{},"files":{}}}"#,
            path_arg,
            dirs_json,
            files_json
        )
    }
}

fn run_yo_command(args: &[&str]) -> Result<String, String> {
    let output = Command::new("yo")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute yo command: {}", e))?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn handle_file_upload(headers: &HashMap<String, String>, body: &[u8]) -> String {
    let uploads_dir = "/var/lib/zigduck/uploads";
    if let Err(e) = create_dir_all(uploads_dir) {
        return format!(r#"{{"error":"Failed to create uploads directory: {}"}}"#, e);
    }

    let content_type = headers.get("content-type").unwrap_or(&String::new()).clone();
    
    if !content_type.contains("multipart/form-data") {
        return r#"{"error":"Only multipart/form-data uploads are supported"}"#.to_string();
    }
    
    let boundary = if let Some(idx) = content_type.find("boundary=") {
        content_type[idx + "boundary=".len()..].trim().to_string()
    } else {
        return r#"{"error":"No boundary in Content-Type"}"#.to_string();
    };
    
    dt_debug(&format!("Boundary: {}", boundary));
    
    let body_str = match String::from_utf8(body.to_vec()) {
        Ok(s) => s,
        Err(_) => return r#"{"error":"Body is not valid UTF-8"}"#.to_string(),
    };
    
    let boundary_marker = format!("--{}", boundary);
    let parts: Vec<&str> = body_str.split(&boundary_marker).collect();
    
    dt_debug(&format!("Found {} parts", parts.len()));
    
    for (i, part) in parts.iter().enumerate().skip(1) {
        if i == parts.len() - 1 && part.trim().ends_with("--") {
            continue;
        }
        
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        
        log(&format!("Part {}: {} chars", i, part.len()));
        
        if let Some(idx) = part.find("\r\n\r\n") {
            let headers_part = &part[..idx];
            let content_start = idx + 4;
            let content = &part[content_start..];
            
            let mut filename = None;
            for line in headers_part.split("\r\n") {
                if line.to_lowercase().contains("filename=") {
                    if let Some(start_idx) = line.find("filename=\"") {
                        let start = start_idx + "filename=\"".len();
                        if let Some(end_idx) = line[start..].find('\"') {
                            filename = Some(line[start..start + end_idx].to_string());
                            break;
                        }
                    }
                }
            }
            
            if let Some(original_filename) = filename {
                // 🦆 says ⮞ helper 2 get unique filename
                fn get_unique_filename(dir: &str, base: &str) -> Result<String, String> {
                    use std::path::Path;               
                    const MAX_ATTEMPTS: usize = 1000;
                    
                    let path = Path::new(base);
                    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
                    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                    
                    let mut candidate = base.to_string();
                    let mut full_path = Path::new(dir).join(&candidate);
                    
                    if !full_path.exists() {
                        dt_debug(&format!("Base filename available: {}", candidate));
                        return Ok(candidate);
                    }
                    
                    dt_info(&format!("Base filename exists: {}, generating unique name", candidate));
                    
                    for counter in 1..=MAX_ATTEMPTS {
                        candidate = if ext.is_empty() {
                            format!("{}({})", stem, counter)
                        } else {
                            format!("{}({}).{}", stem, counter, ext)
                        };
                        
                        full_path = Path::new(dir).join(&candidate);
                        if !full_path.exists() {
                            log(&format!("Found unique filename: {}", candidate));
                            return Ok(candidate);
                        }
                    }               
                    Err(format!("Could not find unique filename after {} attempts", MAX_ATTEMPTS))
                }
                
                let sanitized = sanitize_filename(&original_filename);
                log(&format!("Sanitized filename: {}", sanitized));
                
                match get_unique_filename(uploads_dir, &sanitized) {
                    Ok(unique_name) => {
                        let destination = format!("{}/{}", uploads_dir, unique_name);
                        let clean_content = content.trim_end_matches("\r\n");
                        
                        log(&format!("Writing {} bytes to {}", clean_content.len(), destination));
                        
                        match std::fs::write(&destination, clean_content) {
                            Ok(_) => {
                                let file_size = clean_content.len();
                                
                                let response = json!({
                                    "status": "success",
                                    "message": "File uploaded successfully",
                                    "files": [{
                                        "filename": unique_name,
                                        "original_filename": original_filename,
                                        "size": file_size,
                                        "path": destination
                                    }]
                                }).to_string();
                                
                                dt_info(&format!("Upload successful: {}", response));
                                return response;
                            }
                            Err(e) => {
                                let error_msg = format!(r#"{{"error":"Failed to write file: {}"}}"#, e);
                                log(&format!("Write error: {}", error_msg));
                                return error_msg;
                            }
                        }
                    }
                    Err(e) => {
                        let error_msg = format!(r#"{{"error":"{}"}}"#, e);
                        log(&format!("Unique filename error: {}", error_msg));
                        return error_msg;
                    }
                }
            }
        }
    }   
    r#"{"error":"No file found in upload"}"#.to_string()
}

fn sanitize_filename(filename: &str) -> String {
    let mut sanitized = String::new();
    for c in filename.chars() {
        if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
            sanitized.push(c);
        } else if c == ' ' {
            sanitized.push('_');
        }
    }    
    // 🦆 says ⮞ make sure we have at least something
    if sanitized.is_empty() {
        format!("file_{}.bin", chrono::Local::now().format("%Y%m%d_%H%M%S"))
    } else {
        sanitized
    }
}
  

// 🦆 says ⮞ device control endpoints
fn handle_device_list() -> String {
    match fs::read_to_string("devices.json") {
        Ok(content) => content,
        Err(_) => r#"{"error":"Devices file not found"}"#.to_string(),
    }
}
        
fn handle_device_rest_control(path: &str) -> String {
    dt_info(&format!("Device control request: {}", path));    
    let segments: Vec<&str> = path.split('/').collect();
    
    if segments.is_empty() {
        dt_warning("Device control called without device name");
        return r#"{"error":"Missing device name"}"#.to_string();
    }
    
    let device_name = urldecode(segments[0]);
    dt_info(&format!("Controlling device: {}", device_name));
    
    let mut commands = Vec::new();
    let mut i = 1;
    
    while i < segments.len() {
        if i + 1 < segments.len() {
            let action = segments[i];
            let value = urldecode(segments[i + 1]);
            commands.push((action, value));
            i += 2;
        } else {
            return r#"{"error":"Malformed command path"}"#.to_string();
        }
    }
    
    if commands.is_empty() {
        return r#"{"error":"No commands specified"}"#.to_string();
    }
    
    handle_device_combined_control(&device_name, &commands)
}


fn handle_device_combined_control(device_name: &str, commands: &[(&str, String)]) -> String {
    dt_info(&format!("Device '{}' commands: {:?}", device_name, commands));

    let devices_json = fs::read_to_string("devices.json").unwrap_or_else(|_| "{}".to_string());
    let devices: HashMap<String, serde_json::Value> =
        serde_json::from_str(&devices_json).unwrap_or_default();

    let mut found_device = None;
    for (dev_name, _) in &devices {
        if dev_name.to_lowercase() == device_name.to_lowercase() {
            found_device = Some(dev_name.clone());
            break;
        }
    }

    let actual_name = match found_device {
        Some(name) => name,
        None => return format!(r#"{{"error":"Device not found: {}"}}"#, device_name),
    };

    let mut args: Vec<String> = vec!["--device".to_string(), actual_name.clone()];
    let mut state_explicit = false;

    for (action, value) in commands {
        match *action {
            "state" => {
                let state_val = value.to_lowercase();
                match state_val.as_str() {
                    "on" | "off" | "toggle" => {
                        args.push("--state".to_string());
                        args.push(state_val);
                        state_explicit = true;
                    }
                    _ => {
                        return format!(r#"{{"error":"Invalid state value: {}"}}"#, value);
                    }
                }
            }
            "brightness" => {
                if let Ok(raw_val) = value.parse::<u16>() {
                    let pct = if raw_val > 100 {
                        ((raw_val as f32 / 254.0) * 100.0).round() as u8
                    } else {
                        raw_val as u8
                    };
                    if pct < 1 || pct > 100 {
                        return format!(
                            r#"{{"error":"Invalid brightness value (must be 1-100 or 1-254): {}"}}"#,
                            value
                        );
                    }
                    args.push("--brightness".to_string());
                    args.push(pct.to_string());
                } else {
                    return format!(r#"{{"error":"Invalid brightness value: {}"}}"#, value);
                }
            }
            "color" | "colour" => {
                let hex_value = if value.starts_with('#') {
                    value.clone()
                } else {
                    format!("#{}", value)
                };
                if hex_value.len() == 7 {
                    args.push("--color".to_string());
                    args.push(hex_value);
                } else {
                    return format!(
                        r#"{{"error":"Invalid color format, use #RRGGBB or RRGGBB"}}"#
                    );
                }
            }
            "temperature" | "temp" | "color_temp" => {
                if let Ok(temp) = value.parse::<u16>() {
                    args.push("--temperature".to_string());
                    args.push(temp.to_string());
                } else {
                    return format!(
                        r#"{{"error":"Invalid temperature value: {}"}}"#,
                        value
                    );
                }
            }
            _ => {
                return format!(r#"{{"error":"Unknown action: {}"}}"#, action);
            }
        }
    }

    if !state_explicit {
        args.push("--state".to_string());
        args.push("on".to_string());
    }

    let output = Command::new("zigduck-cli")
        .args(&args)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let command_list: Vec<String> = commands
                .iter()
                .map(|(a, v)| format!("{}:{}", a, v))
                .collect();
            format!(
                r#"{{"status":"ok","device":"{}","commands":{}}}"#,
                actual_name,
                serde_json::to_string(&command_list).unwrap()
            )
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            dt_error(&format!(
                "Failed to control device '{}': {}",
                actual_name, stderr
            ));
            format!(r#"{{"error":"Failed to control device: {}"}}"#, stderr.trim())
        }
        Err(e) => {
            dt_error(&format!(
                "Failed to execute zigduck-cli for '{}': {}",
                actual_name, e
            ));
            format!(
                r#"{{"error":"Failed to execute zigduck-cli: {}"}}"#,
                e
            )
        }
    }
}
    
    
fn handle_scene_activate(scene_name: &str) -> String {
    if scene_name.is_empty() {
        return r#"{"error":"Missing scene name"}"#.to_string();
    }
    let scenes_content = match fs::read_to_string("scenes.json") {
        Ok(c) => c,
        Err(_) => return r#"{"error":"Scenes file not found"}"#.to_string(),
    };
    let parsed: serde_json::Value = serde_json::from_str(&scenes_content).unwrap_or(json!({}));
    let scenes_obj = parsed
        .get("scenes")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_else(|| serde_json::Map::new());

    let mut scene_map: HashMap<String, serde_json::Value> = HashMap::new();
    for (key, value) in scenes_obj.iter() {
        scene_map.insert(key.to_lowercase(), value.clone());
        scene_map.insert(key.clone(), value.clone());
    }

    let normalized = scene_name.to_lowercase();
    if scene_map.contains_key(&normalized) {
        let actual_name = scenes_obj
            .keys()
            .find(|k| k.to_lowercase() == normalized)
            .cloned()
            .unwrap_or_else(|| scene_name.to_string());
        //match run_yo_command(&["house", "--scene", &actual_name]) { 
        match Command::new("zigduck-cli")
            .arg("--scene")
            .arg(&actual_name)
            .output()
        {
            Ok(output) if output.status.success() => {
                format!(r#"{{"status":"ok","scene":"{}"}}"#, actual_name)
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                dt_error(&format!("Failed to activate scene '{}': {}", actual_name, stderr));
                format!(r#"{{"error":"Failed to activate scene: {}"}}"#, stderr.trim())
            }
            Err(e) => {
                dt_error(&format!("Failed to run zigduck-cli for scene '{}': {}", actual_name, e));
                format!(r#"{{"error":"Failed to run zigduck-cli: {}"}}"#, e)
            }
        }
    } else { format!(r#"{{"error":"Scene not found: {}"}}"#, scene_name) }
}


fn handle_rooms_list() -> String {
    match fs::read_to_string("rooms.json") {
        Ok(content) => content,
        Err(_) => r#"{"error":"Rooms data not available"}"#.to_string(),
    }
}

fn handle_types_list() -> String {
    match fs::read_to_string("types.json") {
        Ok(content) => content,
        Err(_) => r#"{"error":"Types data not available"}"#.to_string(),
    }
}

fn handle_health_check() -> String {
    match Command::new("health").output() {
        Ok(output) if output.status.success() => {
            let health_output = String::from_utf8_lossy(&output.stdout);
            // 🦆 says ⮞ health script already returns JSON, so we can use it directly
            health_output.to_string()
        }
        Ok(output) => {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            // 🦆 says ⮞ fallback if health command fails
            let timestamp = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%z").to_string();
            format!(
                r#"{{"status":"degraded","service":"zigduck-api","timestamp":"{}","error":"Health check failed: {}"}}"#,
                timestamp, error_msg
            )
        }
        Err(e) => {
            // 🦆 says ⮞ fallback if health command not found
            let timestamp = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%z").to_string();
            format!(
                r#"{{"status":"degraded","service":"zigduck-api","timestamp":"{}","error":"Health command failed: {}"}}"#,
                timestamp, e
            )
        }
    }
}

fn handle_health_all() -> String {
    let health_dir = "/var/lib/zigduck/health";
    let mut health_data = std::collections::HashMap::new();

    if let Ok(entries) = std::fs::read_dir(health_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                            health_data.insert(file_stem.to_string(), json);
                        }
                    }
                }
            }
        }
    }

    match serde_json::to_string(&health_data) {
        Ok(json) => json,
        Err(_) => r#"{"error":"Failed to serialize health data"}"#.to_string(),
    }
}



fn read_playlist_lines() -> Result<Vec<String>, String> {
    match fs::read_to_string(get_playlist_path()) {
        Ok(content) => {
            let lines: Vec<String> = content
                .lines()
                .map(|l| l.to_string())
                .filter(|l| !l.trim().is_empty())
                .collect();
            Ok(lines)
        }
        Err(e) => Err(format!("Failed to read playlist: {}", e)),
    }
}

fn write_playlist_lines(lines: &[String]) -> Result<(), String> {
    let content = lines.join("\n") + if lines.is_empty() { "" } else { "\n" };
    fs::write(get_playlist_path(), content).map_err(|e| format!("Failed to write playlist: {}", e))
}

fn handle_m3u_clear() -> String {
    let _lock = PLAYLIST_MUTEX.lock().unwrap();
    match fs::write(get_playlist_path(), "") {
        Ok(_) => json!({"status":"ok","action":"clear"}).to_string(),
        Err(e) => json!({"error": format!("Could not clear playlist: {}", e)}).to_string(),
    }
}

fn handle_m3u_add(entry: &str) -> String {
    if entry.is_empty() {
        return json!({"error":"Missing entry parameter"}).to_string();
    }
    let _lock = PLAYLIST_MUTEX.lock().unwrap();
    match read_playlist_lines() {
        Ok(mut lines) => {
            lines.push(entry.to_string());
            match write_playlist_lines(&lines) {
                Ok(_) => json!({"status":"ok","action":"add","entry":entry}).to_string(),
                Err(e) => json!({"error":e}).to_string(),
            }
        }
        Err(e) => json!({"error":e}).to_string(),
    }
}

fn handle_m3u_remove(index_str: &str) -> String {
    let index: usize = match index_str.parse() {
        Ok(i) => i,
        Err(_) => return json!({"error":"Invalid index (must be a number)"}).to_string(),
    };

    let _lock = PLAYLIST_MUTEX.lock().unwrap();
    match read_playlist_lines() {
        Ok(mut lines) => {
            if index >= lines.len() {
                return json!({"error": format!("Index {} out of bounds (0–{})", index, lines.len().saturating_sub(1))}).to_string();
            }
            let removed = lines.remove(index);
            match write_playlist_lines(&lines) {
                Ok(_) => json!({"status":"ok","action":"remove","index":index,"removed":removed}).to_string(),
                Err(e) => json!({"error":e}).to_string(),
            }
        }
        Err(e) => json!({"error":e}).to_string(),
    }
}

fn handle_m3u_shuffle() -> String {
    use rand::seq::SliceRandom;
    let _lock = PLAYLIST_MUTEX.lock().unwrap();
    match read_playlist_lines() {
        Ok(mut lines) => {
            let mut rng = rand::thread_rng();
            lines.shuffle(&mut rng);
            match write_playlist_lines(&lines) {
                Ok(_) => json!({"status":"ok","action":"shuffle","count":lines.len()}).to_string(),
                Err(e) => json!({"error":e}).to_string(),
            }
        }
        Err(e) => json!({"error":e}).to_string(),
    }
}

fn handle_m3u_list() -> String {
    let _lock = PLAYLIST_MUTEX.lock().unwrap();
    match read_playlist_lines() {
        Ok(lines) => json!({"playlist": lines}).to_string(),
        Err(e) => json!({"error":e}).to_string(),
    }
}


fn handle_alarm_list() -> String {
    let alarms = ALARM_MANAGER.list();
    serde_json::to_string(&alarms).unwrap_or_else(|_| "[]".to_string())
}

fn handle_alarm_add(query: &str) -> String {
    let hours: u8 = get_query_arg(query, "hours").parse().unwrap_or(0);
    let minutes: u8 = get_query_arg(query, "minutes").parse().unwrap_or(0);
    let name = urldecode(&get_query_arg(query, "name"));
    let days_str = get_query_arg(query, "days");
    let days: Option<Vec<u8>> = if days_str.is_empty() {
        None
    } else {
        let parsed: Vec<u8> = days_str.split(',')
            .filter_map(|s| s.trim().parse().ok())
            .filter(|d| *d <= 6)
            .collect();
        if parsed.is_empty() { None } else { Some(parsed) }
    };

    let topic = get_query_arg(query, "topic");
    let payload = get_query_arg(query, "payload");
    let topic_opt = if topic.is_empty() { None } else { Some(urldecode(&topic)) };
    let payload_opt = if payload.is_empty() { None } else { Some(urldecode(&payload)) };

    if name.is_empty() {
        return json!({"error":"Missing alarm name"}).to_string();
    }
    if hours > 23 || minutes > 59 {
        return json!({"error":"Invalid time"}).to_string();
    }

    let id = ALARM_MANAGER.add(hours, minutes, days, name, topic_opt, payload_opt);
    json!({"status":"ok","id":id}).to_string()
}


fn handle_alarm_remove(query: &str) -> String {
    let id: u64 = get_query_arg(query, "id").parse().unwrap_or(0);
    if id == 0 {
        return json!({"error":"Missing id parameter"}).to_string();
    }
    match ALARM_MANAGER.remove(id) {
        Ok(alarm) => json!({"status":"ok","removed":alarm.name}).to_string(),
        Err(e) => json!({"error":e}).to_string(),
    }
}

fn handle_alarm_toggle(query: &str) -> String {
    let id: u64 = get_query_arg(query, "id").parse().unwrap_or(0);
    if id == 0 {
        return json!({"error":"Missing id parameter"}).to_string();
    }
    match ALARM_MANAGER.toggle(id) {
        Ok(alarm) => json!({"status":"ok","id":alarm.id,"enabled":alarm.enabled}).to_string(),
        Err(e) => json!({"error":e}).to_string(),
    }
}


fn start_alarm_thread(manager: Arc<AlarmManager>) {
    std::thread::spawn(move || {
        loop {
            let next_time = manager.next_trigger_time();
            let sleep_duration = match next_time {
                Some(target) => {
                    let now = Local::now().time();
                    let diff = if target > now {
                        target - now
                    } else {
                        chrono::Duration::seconds(60)
                    };
                    diff.to_std().unwrap_or(Duration::from_secs(60))
                }
                None => Duration::from_secs(60),
            };

            let _ = manager.condvar
                .wait_timeout(manager.alarms.lock().unwrap(), sleep_duration)
                .unwrap();

            let now = Local::now();
            let today_str = now.format("%Y-%m-%d").to_string();
            let weekday = now.weekday().num_days_from_sunday() as u8;
            let current_time = now.time();

            let mut alarms = manager.alarms.lock().unwrap();
            let mut changed = false;
            for alarm in alarms.iter_mut() {
                if !alarm.enabled {
                    continue;
                }

                if let Some(ref days) = alarm.days {
                    if !days.contains(&weekday) {
                        continue;
                    }
                }

                if alarm.hour == current_time.hour() as u8
                    && alarm.minute == current_time.minute() as u8
                {
                    if alarm.last_fired.as_deref() == Some(&today_str) {
                        continue;
                    }

                    if let Some(ref action) = alarm.action {
                        let mut cmd = std::process::Command::new("mosquitto_pub");
                        cmd.arg("-t").arg(&action.topic)
                           .arg("-m").arg(&action.payload);

                        if let Some((user, pass)) = get_mqtt_credentials() {
                            cmd.arg("-u").arg(user).arg("-P").arg(pass);
                        }
                        if let Ok(broker) = std::env::var("MQTT_BROKER") {
                            if broker != "localhost" && broker != "127.0.0.1" {
                                cmd.arg("-h").arg(broker);
                            }
                        }

                        match cmd.output() {
                            Ok(out) if !out.status.success() => {
                                dt_error(&format!(
                                    "mosquitto_pub (alarm) failed: {}",
                                    String::from_utf8_lossy(&out.stderr)
                                ));
                            }
                            Ok(_) => {}
                            Err(e) => dt_error(&format!("mosquitto_pub (alarm) spawn error: {}", e)),
                        }
                    }

                    alarm.last_fired = Some(today_str.clone());
                    changed = true;
                }
            }
            if changed {
                manager.save_to_file();
            }
        }
    });
}


fn handle_request(mut stream: TcpStream) {
    let peer_addr = match stream.peer_addr() {
        Ok(addr) => addr.to_string(),
        Err(_) => "unknown".to_string(),
    };

    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    
    // 🦆 says ⮞ read request line
    if reader.read_line(&mut request_line).is_err() || request_line.is_empty() {
        log("No data on stdin; exiting");
        return;
    }
    // 🦆 says ⮞ log requester ip
    dt_info(&format!("[{}] Request: {}", peer_addr, request_line.trim()));
    
    log(&format!("Request: {}", request_line.trim()));

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let method = parts[0];
    let raw_path = parts[1];

    // 🦆 says ⮞ read headers
    let mut content_length = 0;
    let mut headers = HashMap::new();
    let mut header_line = String::new();
    loop {
        header_line.clear();
        if reader.read_line(&mut header_line).is_err() || header_line.is_empty() {
            break;
        }
        if header_line == "\r\n" || header_line == "\n" {
            break;
        }
        
        if let Some((key, value)) = header_line.split_once(':') {
            let key_lower = key.trim().to_lowercase();
            let value_trimmed = value.trim().to_string();
            
            if key_lower == "content-length" {
                content_length = value_trimmed.parse().unwrap_or(0);
            }
            
            headers.insert(key_lower, value_trimmed);
        }
    }

    // 🦆 says ⮞ read body if present
    let mut body = Vec::new();
    if content_length > 0 {
        let mut body_buf = vec![0; content_length];
        if let Ok(()) = reader.read_exact(&mut body_buf) {
            body = body_buf;
            log(&format!("Body size: {} bytes", body.len()));
        }
    }

    // 🦆 says ⮞ parse path and query
    let (path_no_query, query) = match raw_path.split_once('?') {
        Some((path, query)) => (path, query),
        None => (raw_path, ""),
    };

    // 🦆 says ⮞ exclude authentication for health
    if path_no_query != "/health" && path_no_query != "/health/all" && !check_password_auth(&headers, query) {
        send_response(&mut stream, "401 Unauthorized", 
            r#"{"error":"Authentication required","message":"Valid password required in Authorization: Bearer <password> header, X-API-Key header, or ?password= query parameter"}"#, 
            None);
        return;
    }

    // 🦆 says ⮞ route the request
    match (method, path_no_query) {
        // 🦆 says ⮞ handle CORS preflight
        ("OPTIONS", _) => {
            dt_debug("CORS preflight request");
            send_response(&mut stream, "200 OK", "", None);
            return;
        }
    
        ("GET", "/") => {
            dt_info("Root endpoint requested");
            send_response(&mut stream, "200 OK", 
                r#"{"service":"zigduck-api","endpoints":["/timers","/alarms","/alarms/add","/alarms/remove","/alarms/toggle","/health","/health/all","/do","/browse","/browsev2","/device/list","/device/{device}/...","/device/rooms","/device/types","/scene/{scene}","/state","/state/{device}","/state/room/{room}","/tts","/upload","/transcode-video","/playlist/list","/playlist/add","/playlist/remove","/playlist/shuffle","/playlist/clear","/media/playlist","/media/next","/media/previous","/media/play","/media/pause","/media/volume/up","/media/volume/down","/media/power/on","/media/power/off"]}"#,
                None);
        }
        
        ("GET", "/transcode-video") | ("GET", "/api/transcode-video") => {
            let url = get_query_arg(query, "url");
            if url.is_empty() {
                dt_warning("Transcode video called without URL");
                send_response(&mut stream, "400 Bad Request", 
                    r#"{"error":"Missing url parameter"}"#, None);
                return;
            }

            dt_info(&format!("Transcoding video from: {}", url));

            match handle_transcode_video_stream(&url, &mut stream) {
                Ok(_) => {
                    return;
                }
                Err(e) => {
                    dt_error(&format!("Transcoding failed: {}", e));
                    send_response(&mut stream, "500 Internal Server Error", 
                        &format!(r#"{{"error":"Transcoding failed: {}"}}"#, e), None);
                }
            }
        }            
                   
        ("GET", "/browsev2") | ("GET", "/api/browsev2") => {
            let path_arg = get_path_arg(query);
            let response = handle_browse(&path_arg, true);
            send_response(&mut stream, "200 OK", &response, None);
        }
        ("GET", "/browse") | ("GET", "/api/browse") => {
            let path_arg = get_path_arg(query);
            let response = handle_browse(&path_arg, false);
            send_response(&mut stream, "200 OK", &response, None);
        }   
        
        ("GET", "/timers") => {
            let timers = TIMER_MANAGER.list();
            let json_timers: Vec<serde_json::Value> = timers.iter().map(|t| {
                let remaining = if let Some(paused_rem) = t.paused_remaining {
                    paused_rem
                } else {
                    t.fire_at.saturating_duration_since(Instant::now())
                };
                json!({
                    "id": t.id,
                    "name": t.name,
                    "remaining_seconds": remaining.as_secs(),
                    "paused": t.paused_remaining.is_some(),
                    "action": match &t.action {
                        TimerAction::MqttMessage { topic, payload } => json!({
                            "type": "mqtt",
                            "topic": topic,
                            "payload": payload,
                        })
                    }
                })
            }).collect();

            let body = serde_json::to_string(&json_timers).unwrap_or_else(|_| "[]".to_string());
            send_response(&mut stream, "200 OK", &body, None);
        }
        
        ("GET", "/timers/set") => {
            let hours: u32 = get_query_arg(query, "hours").parse().unwrap_or(0);
            let minutes: u32 = get_query_arg(query, "minutes").parse().unwrap_or(0);
            let seconds: u32 = get_query_arg(query, "seconds").parse().unwrap_or(0);
            let topic = get_query_arg(query, "topic");
            let payload = get_query_arg(query, "payload");
            let name = urldecode(&get_query_arg(query, "name"));
            if topic.is_empty() || payload.is_empty() || (hours == 0 && minutes == 0 && seconds == 0) {
                send_response(&mut stream, "400 Bad Request", r#"{"error":"Missing or invalid parameters (need topic, payload, and a positive duration)"}"#, None);
                return;
            }
            let action = TimerAction::MqttMessage { topic, payload };
            let id = TIMER_MANAGER.add(hours, minutes, seconds, action, name);
            send_response(&mut stream, "200 OK", &format!(r#"{{"status":"ok","timer_id":{}}}"#, id), None);
        }
        
        ("GET", "/timers/pause") => {
            let id: TimerId = get_query_arg(query, "id").parse().unwrap_or(0);
            if id == 0 {
                send_response(&mut stream, "400 Bad Request", r#"{"error":"Missing id parameter"}"#, None);
                return;
            }
            match TIMER_MANAGER.pause(id) {
                Ok(()) => send_response(&mut stream, "200 OK", r#"{"status":"ok"}"#, None),
                Err(e) => send_response(&mut stream, "400 Bad Request", &format!(r#"{{"error":"{}"}}"#, e), None),
            }
        }
        
        ("GET", "/timers/resume") => {
            let id: TimerId = get_query_arg(query, "id").parse().unwrap_or(0);
            if id == 0 {
                send_response(&mut stream, "400 Bad Request", r#"{"error":"Missing id parameter"}"#, None);
                return;
            }
            match TIMER_MANAGER.resume(id) {
                Ok(()) => send_response(&mut stream, "200 OK", r#"{"status":"ok"}"#, None),
                Err(e) => send_response(&mut stream, "400 Bad Request", &format!(r#"{{"error":"{}"}}"#, e), None),
            }
        }
        
        ("GET", "/timers/cancel") => {
            let id: TimerId = get_query_arg(query, "id").parse().unwrap_or(0);
            if id == 0 {
                send_response(&mut stream, "400 Bad Request", r#"{"error":"Missing id parameter"}"#, None);
                return;
            }
            match TIMER_MANAGER.cancel(id) {
                Ok(timer) => send_response(&mut stream, "200 OK", &format!(r#"{{"status":"ok","cancelled_timer":{{"id":{},"name":"{}"}}}}"#, timer.id, timer.name), None),
                Err(e) => send_response(&mut stream, "400 Bad Request", &format!(r#"{{"error":"{}"}}"#, e), None),
            }
        }
        
        
        ("GET", "/alarms") | ("GET", "/api/alarms") => {
            let response = handle_alarm_list();
            send_response(&mut stream, "200 OK", &response, None);
        }
        ("GET", "/alarms/add") | ("GET", "/api/alarms/add") => {
            let response = handle_alarm_add(query);
            send_response(&mut stream, "200 OK", &response, None);
        }
        ("GET", "/alarms/remove") | ("GET", "/api/alarms/remove") => {
            let response = handle_alarm_remove(query);
            send_response(&mut stream, "200 OK", &response, None);
        }
        ("GET", "/alarms/toggle") | ("GET", "/api/alarms/toggle") => {
            let response = handle_alarm_toggle(query);
            send_response(&mut stream, "200 OK", &response, None);
        }
        ("GET", "/media/power/on") | ("GET", "/api/media/power/on") => {
            let device = get_device_ip(query);
            match execute_adb(&device, &["shell", "input", "keyevent", "KEYCODE_WAKEUP"]) {
                Ok(_) => send_response(&mut stream, "200 OK",
                    &format!(r#"{{"status":"ok","action":"power_on","device":"{}"}}"#, device), None),
                Err(e) => send_response(&mut stream, "500 Internal Server Error",
                    &format!(r#"{{"error":"{}"}}"#, e), None),
            }
        }

        ("GET", "/media/power/off") | ("GET", "/api/media/power/off") => {
            let device = get_device_ip(query);
            match execute_adb(&device, &["shell", "input", "keyevent", "KEYCODE_SLEEP"]) {
                Ok(_) => send_response(&mut stream, "200 OK",
                    &format!(r#"{{"status":"ok","action":"power_off","device":"{}"}}"#, device), None),
                Err(e) => send_response(&mut stream, "500 Internal Server Error",
                    &format!(r#"{{"error":"{}"}}"#, e), None),
            }
        }        
        
        ("GET", "/media/next") => {
            let device = get_device_ip(query);
            match execute_adb(&device, &["shell", "input", "keyevent", "KEYCODE_MEDIA_NEXT"]) {
                Ok(_) => send_response(&mut stream, "200 OK", &format!(r#"{{"status":"ok","action":"next","device":"{}"}}"#, device), None),
                Err(e) => send_response(&mut stream, "500 Internal Server Error", &format!(r#"{{"error":"{}"}}"#, e), None),
            }
        }

        ("GET", "/media/previous") => {
            let device = get_device_ip(query);
            match execute_adb(&device, &["shell", "input", "keyevent", "KEYCODE_MEDIA_PREVIOUS"]) {
                Ok(_) => send_response(&mut stream, "200 OK", &format!(r#"{{"status":"ok","action":"previous","device":"{}"}}"#, device), None),
                Err(e) => send_response(&mut stream, "500 Internal Server Error", &format!(r#"{{"error":"{}"}}"#, e), None),
            }
        }

        ("GET", "/media/pause") | ("GET", "/media/play") => {
            let device = get_device_ip(query);
            match execute_adb(&device, &["shell", "input", "keyevent", "KEYCODE_MEDIA_PLAY_PAUSE"]) {
                Ok(_) => send_response(&mut stream, "200 OK", &format!(r#"{{"status":"ok","action":"toggle_play","device":"{}"}}"#, device), None),
                Err(e) => send_response(&mut stream, "500 Internal Server Error", &format!(r#"{{"error":"{}"}}"#, e), None),
            }
        }

        ("GET", "/media/volume/up") => {
            let device = get_device_ip(query);
            match execute_adb(&device, &["shell", "input", "keyevent", "KEYCODE_VOLUME_UP"]) {
                Ok(_) => send_response(&mut stream, "200 OK", &format!(r#"{{"status":"ok","action":"volume_up","device":"{}"}}"#, device), None),
                Err(e) => send_response(&mut stream, "500 Internal Server Error", &format!(r#"{{"error":"{}"}}"#, e), None),
            }
        }

        ("GET", "/media/volume/down") => {
            let device = get_device_ip(query);
            match execute_adb(&device, &["shell", "input", "keyevent", "KEYCODE_VOLUME_DOWN"]) {
                Ok(_) => send_response(&mut stream, "200 OK", &format!(r#"{{"status":"ok","action":"volume_down","device":"{}"}}"#, device), None),
                Err(e) => send_response(&mut stream, "500 Internal Server Error", &format!(r#"{{"error":"{}"}}"#, e), None),
            }
        }

        ("GET", "/media/playlist") => {
            let device = get_device_ip(query);
            let url = get_query_arg(query, "url");
            let playlist_url = if url.is_empty() {
                match read_webserver_url() {
                    Ok(base) => format!("{}/playlist.m3u", base),
                    Err(e) => {
                        send_response(&mut stream, "500 Internal Server Error", &format!(r#"{{"error":"{}"}}"#, e), None);
                        return;
                    }
                }
            } else {
                url
            };

            match execute_adb(
                &device,
                &["shell", "am", "start", "-a", "android.intent.action.VIEW", "-d", &playlist_url, "-t", "audio/x-mpegurl"],
            ) {
                Ok(_) => send_response(&mut stream, "200 OK", &format!(r#"{{"status":"ok","action":"play_playlist","device":"{}","url":"{}"}}"#, device, playlist_url), None),
                Err(e) => send_response(&mut stream, "500 Internal Server Error", &format!(r#"{{"error":"Failed to start playlist: {}"}}"#, e), None),
            }
        }
        
                 
        ("GET", "/playlist/clear") => {
            let response = handle_m3u_clear();
            send_response(&mut stream, "200 OK", &response, None);
        }
        ("GET", "/playlist/add") => {
            let entry = urldecode(&get_query_arg(query, "entry"));
            let response = handle_m3u_add(&entry);
            send_response(&mut stream, "200 OK", &response, None);
        }
        ("GET", "/playlist/remove") => {
            let index = get_query_arg(query, "index");
            let response = handle_m3u_remove(&index);
            send_response(&mut stream, "200 OK", &response, None);
        }
        ("GET", "/playlist/shuffle") => {
            let response = handle_m3u_shuffle();
            send_response(&mut stream, "200 OK", &response, None);
        }
        ("GET", "/playlist/list") => {
            let response = handle_m3u_list();
            send_response(&mut stream, "200 OK", &response, None);
        }
          
                 
        ("GET", "/health") | ("GET", "/api/health") => {
            let response = handle_health_check();
            send_response(&mut stream, "200 OK", &response, None);
        }            
        ("GET", "/health/all") | ("GET", "/api/health/all") => {
            let response = handle_health_all();
            send_response(&mut stream, "200 OK", &response, None);
        }
        
        ("GET", "/state") | ("GET", "/api/state") => {
            dt_info("Full state request");
            let response = handle_state_all();
            send_response(&mut stream, "200 OK", &response, Some("application/json"));
        }
        
        ("GET", path) if path.starts_with("/state/") || path.starts_with("/api/state/") => {
            let rest = if let Some(stripped) = path.strip_prefix("/api/state/") {
                stripped
            } else if let Some(stripped) = path.strip_prefix("/state/") {
                stripped
            } else {
                path
            };
            
            let parts: Vec<&str> = rest.split('/').collect();
            
            if parts.is_empty() {
                dt_warning("State endpoint called without parameters");
                send_response(&mut stream, "400 Bad Request", 
                    r#"{"error":"Missing parameters"}"#, None);
                return;
            }
            
            let first_param = parts[0].to_lowercase();
            
            match first_param.as_str() {
                "room" => {
                    if parts.len() < 2 {
                        dt_warning("Room state request without room name");
                        send_response(&mut stream, "400 Bad Request", 
                            r#"{"error":"Missing room name"}"#, None);
                        return;
                    }
                    let room_name = parts[1..].join("/");
                    let decoded_room = urldecode(&room_name);
                    dt_info(&format!("Room state request: {}", decoded_room));
                    let response = handle_state_room(&decoded_room);
                    send_response(&mut stream, "200 OK", &response, Some("application/json"));
                }
                _ => {
                    // 🦆 says ⮞ assume it's a device name
                    let device_name = parts.join("/");
                    let decoded_device = urldecode(&device_name);
                    dt_info(&format!("Device state request: {}", decoded_device));
                    let response = handle_state_device(&decoded_device);
                    send_response(&mut stream, "200 OK", &response, Some("application/json"));
                }
            }
        }   
        
        ("GET", "/device/list") | ("GET", "/api/device/list") => {
            let response = handle_device_list();
            send_response(&mut stream, "200 OK", &response, None);
        }
        
        ("GET", path) if path.starts_with("/device/") || path.starts_with("/api/device/") => {
            let rest = if let Some(stripped) = path.strip_prefix("/api/device/") {
                stripped
            } else if let Some(stripped) = path.strip_prefix("/device/") {
                stripped
            } else {
                path
            };

            if rest == "list" || rest == "rooms" || rest == "types" {
                // let existing handlers handle 'em
            } else {
                dt_info(&format!("Device control: {}", rest));
                let response = handle_device_rest_control(rest);
                send_response(&mut stream, "200 OK", &response, None);
                return;
            }
        }
        
        ("GET", path) if path.starts_with("/scene/") || path.starts_with("/api/scene/") => {
            let scene_name = if let Some(stripped) = path.strip_prefix("/api/scene/") {
                stripped
            } else if let Some(stripped) = path.strip_prefix("/scene/") {
                stripped
            } else {
                path
            };

            // 🦆 says ⮞ replace + with spaces
            let decoded_scene_name = scene_name.replace('+', " ");
            dt_info(&format!("Scene activation: {}", decoded_scene_name));
            
            let response = handle_scene_activate(&decoded_scene_name);
            if response.contains("error") {
                dt_warning(&format!("Scene not found: {}", decoded_scene_name));
                send_response(&mut stream, "404 Not Found", &response, None);
            } else {
                dt_info(&format!("Scene activated: {}", decoded_scene_name));
                send_response(&mut stream, "200 OK", &response, None);
            }
        }
        
        ("GET", "/device/rooms") | ("GET", "/api/device/rooms") => {
            let response = handle_rooms_list();
            send_response(&mut stream, "200 OK", &response, None);
        }
        
        ("GET", "/device/types") | ("GET", "/api/device/types") => {
            let response = handle_types_list();
            send_response(&mut stream, "200 OK", &response, None);
        }
     
        ("GET", "/tts") => {
            let text = urldecode(&get_query_arg(query, "text"));
            if text.is_empty() {
                dt_warning("TTS endpoint called without text");
                send_response(&mut stream, "400 Bad Request", 
                    r#"{"error":"Missing text parameter"}"#, None);
                return;
            }
            dt_info(&format!("TTS request: {}", text));
            
            let output = std::process::Command::new("yo")
                .args(&["say", "--text", &text, "--web"])
                .output();
        
            match output {
                Ok(output) if output.status.success() => {
                    let wav_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    dt_info(&format!("TTS generated: {}", wav_path));
        
                    match std::fs::read(&wav_path) {
                        Ok(content) => {
                            let response = format!(
                                "HTTP/1.1 200 OK\r\n\
                                 Content-Type: audio/wav\r\n\
                                 Content-Length: {}\r\n\
                                 Access-Control-Allow-Origin: *\r\n\
                                 Cache-Control: no-cache\r\n\r\n",
                                content.len()
                            );
        
                            if let Err(e) = stream.write_all(response.as_bytes()) {
                                dt_error(&format!("Failed to send headers: {}", e));
                                return;
                            }
        
                            if let Err(e) = stream.write_all(&content) {
                                dt_error(&format!("Failed to send audio: {}", e));
                            }
                            
                            if let Err(e) = std::fs::remove_file(&wav_path) {
                                dt_warning(&format!("Failed to remove TTS file {}: {}", wav_path, e));
                            }
                        }
                        Err(e) => {
                            dt_error(&format!("Failed to read WAV file: {}", e));
                            send_response(&mut stream, "500 Internal Server Error", 
                                r#"{"error":"Failed to read audio"}"#, None);
                        }
                    }
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    dt_error(&format!("TTS command failed: {}", stderr));
                    send_response(&mut stream, "500 Internal Server Error", 
                        &format!(r#"{{"error":"TTS failed: {}"}}"#, stderr), None);
                }
                Err(e) => {
                    dt_error(&format!("Failed to run TTS command: {}", e));
                    send_response(&mut stream, "500 Internal Server Error", 
                        &format!(r#"{{"error":"TTS command failed: {}"}}"#, e), None);
                }
            }
        }
              
        ("GET", "/do") | ("GET", "/api/do") => {
            let command = get_query_arg(query, "cmd");
            if command.is_empty() {
                dt_warning("Do endpoint called without command");
                send_response(&mut stream, "400 Bad Request", r#"{"error":"Missing cmd parameter"}"#, None);
                return;
            }
            
            dt_info(&format!("Executing command: {}", command));
            let natural_language = if command.to_lowercase().starts_with("do ") {
                command[3..].trim().to_string()
            } else {
                command.trim().to_string()
            };

            if natural_language.is_empty() {
                dt_warning("Empty command after 'do'");
                send_response(&mut stream, "400 Bad Request", r#"{"error":"Empty command after 'do'"}"#, None);
                return;
            }

            match run_yo_command(&["do", "--input", &natural_language]) {
                Ok(output) => {
                    // 🦆 says ⮞ filter out memory & duckTrace logs
                    let filtered_output: String = output
                        .lines()
                        .filter(|line| !line.contains("MEMORY ADJUSTMENT:"))
                        .filter(|line| !line.contains("[🦆📜]"))
                        .collect::<Vec<&str>>()
                        .join("\n");        

                    // 🦆 says ⮞ replace "⮞" (U+2B9E) with "▶" (U+25B6) for iOS
                    let cleaned_output = filtered_output
                        .replace('⮞', "▶")
                        .replace('"', "\\\"")
                        .replace('\n', "\\n");
        
                    dt_info(&format!("Command executed successfully: {}", natural_language));
                    let response = format!(r#"{{"status":"success","command":"{}","output":"{}"}}"#, 
                        natural_language, cleaned_output.trim());
                    send_response(&mut stream, "200 OK", &response, None);
                }
                Err(error) => {
                    let cleaned_error = error.replace('"', "\\\"").replace('\n', "\\n");
                    dt_error(&format!("Command failed '{}': {}", natural_language, cleaned_error));
                    let response = format!(r#"{{"status":"error","command":"{}","error":"{}"}}"#, 
                        natural_language, cleaned_error.trim());
                    send_response(&mut stream, "500 Internal Server Error", &response, None);
                }
            }
        }
        
        ("POST", "/upload") | ("POST", "/api/upload") => {
            dt_info("File upload request");
            let response = handle_file_upload(&headers, &body);
            if response.contains("error") {
                dt_error(&format!("Upload failed: {}", response));
            } else {
                dt_info("File uploaded successfully");
            }
            send_response(&mut stream, "200 OK", &response, None);
        }
        
        _ => {
            send_response(&mut stream, "404 Not Found", &format!(r#"{{"error":"Endpoint not found","path":"{}"}}"#, raw_path), None);
        }
    }
}

fn main() {
    dt_setup(None, None);
    dt_info(&format!("🚀 Starting yo API server"));
        
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        dt_error("Usage: zigduck-api");
        std::process::exit(1);
    }

    let host = &args[1];
    let port = &args[2];
    let address = format!("{}:{}", host, port);

    start_timer_thread(TIMER_MANAGER.clone());
    start_alarm_thread(ALARM_MANAGER.clone());

    // 🦆 says ⮞ port in use?
    if TcpListener::bind(&address).is_err() {
        dt_error(&format!("❌ Port {} is already in use", port));
        std::process::exit(1);
    }

    let listener = TcpListener::bind(&address).expect("Failed to bind to address");
    log("Available endpoints:");
    log("  GET /timers                     - List timers");
    log("  GET /alarms                     - List alarms");
    log("  GET /alarms/add?hours=...&minutes=...&name=...&days=...[&topic=...&payload=...]  - Add alarm");
    log("  GET /alarms/remove?id=...       - Remove alarm");
    log("  GET /alarms/toggle?id=...       - Toggle alarm");
    log("  GET /health                     - Health check (no auth required)");
    log("  GET /health/all                 - All health checks (no auth required)");
    log("  GET /do?cmd=...                 - Execute natural language commands");
    log("  GET /browse?path=...            - Browse media directory (legacy)");
    log("  GET /browsev2?path=...          - Browse media directory (improved)");
    log("  GET /playlist/list              - Get current m3u playlist");
    log("  GET /playlist/add?entry=...     - Add entry to m3u playlist");
    log("  GET /playlist/remove?index=...  - Remove entry from m3u playlist");
    log("  GET /playlist/shuffle           - Shuffle m3u playlist");
    log("  GET /playlist/clear             - Clear m3u playlist");
    log("  GET /tts?text=...               - Text to speech");
    log("  GET /state                     - Get full state of all devices");
    log("  GET /state/{device}            - Get state for specific device");
    log("  GET /state/room/{room}         - Get state for all devices in a room");
    log("  GET /device/list                - List all devices");
    log("  GET /device/rooms               - List devices by room");
    log("  GET /device/types               - List devices by type");
    log("  GET /scene/{scene}              - Activate scene (e.g., /scene/dark)");
    log("  GET /device/{device}/{command}/{value} - Control devices");
    log("  GET /media/next?device=...      - Next track (direct ADB)");
    log("  GET /media/previous?device=...  - Previous track (direct ADB)");
    log("  GET /media/play?device=...      - Toggle play/pause (direct ADB)");
    log("  GET /media/pause?device=...     - (same as play)");
    log("  GET /media/volume/up?device=... - Volume up (direct ADB)");
    log("  GET /media/volume/down?device=... - Volume down (direct ADB)");
    log("  GET /media/playlist?device=...[&url=...] - Start playlist on device");
    log("      Examples:");
    log("      /device/PC/state/on                     - Turn device on");
    log("      /device/PC/state/off                    - Turn device off");
    log("      /device/PC/brightness/200               - Set brightness");
    log("      /device/PC/color/%23FF5733              - Set color (#FF5733)");
    log("      /device/PC/temperature/300              - Set color temperature");
    log("      /device/PC/state/on/brightness/200      - Combined commands");
    log("  POST /upload                     - Upload files");
    log("🔐 Authentication:");
    log("  All endpoints except /health and /health/all require password authentication");
    log("  Use: Authorization: Bearer <password> header");
    log("  Or:  X-API-Key: <password> header");
    log("  Password is read from API_PASSWORD_FILE environment variable");
    log("Press Ctrl+C to stop");
    

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(move || {
                    handle_request(stream);
                });
            }
            Err(e) => {
                dt_warning(&format!("🔌 Connection failed: {}", e));
            }
        }
    }
}




