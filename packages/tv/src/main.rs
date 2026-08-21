use barely_fuzzy::{best_fuz, trigram_similarity, levenshtein_similarity};
use clap::Parser;
use rand::seq::SliceRandom;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::env;
use std::process::Command;

#[derive(Deserialize, Debug)]
struct YoutubeSearchResponse {
    items: Vec<YoutubeItem>,
}

#[derive(Deserialize, Debug)]
struct YoutubeItem {
    id: YoutubeId,
    snippet: YoutubeSnippet,
}

#[derive(Deserialize, Debug)]
struct YoutubeId {
    #[serde(rename = "videoId")]
    video_id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct YoutubeSnippet {
    title: String,
}

fn search_youtube(query: &str, api_key: &str) -> Result<(String, String), String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get("https://www.googleapis.com/youtube/v3/search")
        .query(&[
            ("part", "snippet"),
            ("type", "video"),
            ("maxResults", "1"),
            ("q", query),
            ("key", api_key),
        ])
        .send()
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("YouTube API error: {}", resp.status()));
    }

    let search_resp: YoutubeSearchResponse = resp.json().map_err(|e| e.to_string())?;
    if let Some(item) = search_resp.items.first() {
        let video_id = item.id.video_id.as_deref().unwrap_or("");
        let title = &item.snippet.title;
        if video_id.is_empty() {
            return Err("No video ID in response".to_string());
        }
        Ok((format!("https://www.youtube.com/watch?v={}", video_id), title.clone()))
    } else { Err("No results found".to_string()) }
}


#[derive(Parser)]
#[command(about = "Cast media to an Android TV device via ADB")]
struct Args {
    #[arg(
        short,
        long,
        value_parser = [
            "on", "off", "up", "down", "next", "prev", "previous", "pause", "play",
            "call", "youtube", "tv", "movie", "podcast", "music", "musicvideo",
            "audiobook", "jukebox", "song", "othervideo", "livetv",
            "play_playlist", "favourites", "starred", "star", "like",
            "nav_up", "nav_down", "nav_left", "nav_right",
            "nav_select", "nav_menu", "nav_back",
            "channel_up", "channel_down", "nav_home", "nav_recents",
        ]
    )]
    typ: String,

    #[arg(short, long)]
    search: Option<String>,

    #[arg(long)]
    season: Option<String>,

    #[arg(long)]
    room: Option<String>,

    #[arg(long)]
    ip: Option<String>,

    #[arg(long)]
    no_shuffle: bool,

    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    shuffle: Option<bool>,

    #[arg(long)]
    max_items: Option<usize>,

    #[arg(long, default_value = "/etc/zigduck/tv-defaults.json")]
    config: PathBuf,
}


// configuration
#[derive(Deserialize)]
struct Config {
    device_ip: String,
    rooms: HashMap<String, String>,
    tvs: HashMap<String, TvConfig>,
    directories: HashMap<String, String>,
    webserver_file: Option<String>,
    playlist_file: String,
    max_items: usize,
    shuffle: bool,
    youtube_api_key_file: Option<String>,
    favourites_file: String,
}

#[derive(Deserialize, Clone)]
struct ChannelConfig {
    name: String,
    #[serde(default)]
    cmd: Option<String>,
    #[serde(default)]
    stream_url: Option<String>,
    id: Option<u32>,
}

#[derive(Deserialize)]
struct TvConfig {
    ip: String,
    room: String,
    #[serde(default)]
    is_default: bool,
    keymap: HashMap<String, String>,
    #[serde(default)]
    apps: HashMap<String, String>,
    #[serde(default)]
    channels: HashMap<String, ChannelConfig>,
}

impl Config {
    fn webserver(&self) -> Option<String> {
        let path = self.webserver_file.as_ref()?;
        let path = std::path::Path::new(path);
        let url = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read webserver URL file: {}", path.display()))
            .trim()
            .to_string();
        Some(url)
    }
    
    fn youtube_api_key(&self) -> Option<String> {
        let path = self.youtube_api_key_file.as_ref()?;
        let path = std::path::Path::new(path);
        let key = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read YouTube API key file: {}", path.display()))
            .trim()
            .to_string();
        Some(key)
    }
}

fn load_config(path: &Path) -> Config {
    let data = std::fs::read_to_string(path).expect("Failed to read config file");
    serde_json::from_str(&data).expect("Invalid JSON config. Please check your `config.house` Nix configuration.")
}

fn resolve_tv<'a>(args: &Args, config: &'a Config) -> &'a TvConfig {
    if let Some(ip) = &args.ip {
        if let Some(tv) = config.tvs.values().find(|tv| &tv.ip == ip) {
            return tv;
        }
        eprintln!("Warning: no TV found with IP '{}', falling back to default", ip);
    }
    if let Some(room) = &args.room {
        if let Some(tv) = config.tvs.values().find(|tv| &tv.room == room) {
            return tv;
        }
        eprintln!("Warning: no TV found in room '{}', falling back to default", room);
    }
    config.tvs.values().find(|tv| tv.is_default)
        .or_else(|| config.tvs.values().next())
        .expect("No TVs defined in config")
}


// adb helpers
fn adb(ip: &str, cmd: &[&str]) -> std::process::Output {
    let mut args = vec!["-s", ip];
    args.extend(cmd);
    let output = Command::new("adb")
        .args(&args)
        .output()
        .expect("adb not found or not executable");
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("ADB command failed: adb -s {} {}\n{}", ip, cmd.join(" "),  stderr.trim());
    }
    output
}

fn adb_keyevent(ip: &str, key: &str) {
    adb(ip, &["shell", "input", "keyevent", key]);
}

fn wake_and_connect(ip: &str, wake_key: &str) {
    let connect = Command::new("adb")
        .args(["connect", ip])
        .output()
        .expect("Failed to run adb");
    if !connect.status.success() {
        let stderr = String::from_utf8_lossy(&connect.stderr);
        eprintln!("ADB connect to {} failed:\n{}", ip, stderr.trim());
        return;
    }
    adb_keyevent(ip, wake_key);
}

fn play_playlist(ip: &str, playlist_url: &str, wake_key: &str) {
    wake_and_connect(ip, wake_key);
    let cmd = format!(
        "am start -a android.intent.action.VIEW -d \"{}\" -t \"audio/x-mpegurl\"",
        playlist_url
    );
    adb(ip, &["shell", &cmd]);
}

fn generate_folder_playlist(
    folder: &Path,
    root_dir: &Path, 
    webserver: &str,
    shuffle: bool,
    max_items: usize,
    playlist_path: &Path,
) {
    let web_folder = root_dir
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();

    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(folder)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let ext = e.path().extension().and_then(|s| s.to_str()).unwrap_or("");
            !matches!(
                ext.to_lowercase().as_str(),
                "nfo" | "jpg" | "jpeg" | "png" | "gif" | "m3u" | "txt" | "db" | "log" | "torrent"
            )
        })
        .map(|e| e.path().to_owned())
        .collect();

    if shuffle {
        files.shuffle(&mut rand::thread_rng());
    }
    files.truncate(max_items);

    let mut playlist = String::from("#EXTM3U\n");
    playlist.push_str("https://raw.githubusercontent.com/QuackHack-McBlindy/share/main/intro.mp4\n");

    for f in &files {
        let rel = f.strip_prefix(root_dir).unwrap();
        let encoded = rel.to_string_lossy().replace(' ', "%20");
        playlist.push_str(&format!("{}/{}/{}\n", webserver, web_folder, encoded));
    }

    std::fs::write(playlist_path, playlist).expect("Failed to write playlist");
    println!("Playlist written to {}", playlist_path.display());
}


// fuzzy match a directory name (barely-fuzzy)
fn fuzzy_match_dir(base: &Path, query: &str) -> Option<String> {
    if !base.is_dir() {
        return None;
    }

    let dirs: Vec<String> = std::fs::read_dir(base)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    if dirs.is_empty() {
        return None;
    }

    let candidates_bytes: Vec<&[u8]> = dirs.iter().map(|s| s.as_bytes()).collect();
    let query_bytes = query.as_bytes();

    let (best_bytes, score) = best_fuz(query_bytes, &candidates_bytes, 30);
    let best = dirs
        .iter()
        .find(|d| d.as_bytes() == best_bytes)
        .cloned()
        .unwrap_or_default();

    if score > 30 {
        Some(best)
    } else { None }
}

// file‑based fuzzy match returning the top 3 (score, path) using a combined score
// 80% Levenshtein + 20% trigram
fn fuzzy_match_files(
    base: &Path,
    query: &str,
    extensions: &[&str],
) -> Vec<(f64, PathBuf)> {
    let mut results = Vec::new();
    let query_lower = query.to_lowercase();
    let query_bytes = query_lower.as_bytes();

    for entry in walkdir::WalkDir::new(base)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if !extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
            continue;
        }
        let stem = path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let stem_bytes = stem.to_lowercase().as_bytes().to_vec();
        let lev = levenshtein_similarity(query_bytes, &stem_bytes) as u8;
        let tri = trigram_similarity(query_bytes, &stem_bytes);
        let combined = (lev as f64 * 0.8) + (tri as f64 * 0.2);
        results.push((combined, path.to_owned()));
    }

    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    results.truncate(3);
    results
}

fn make_playlist_from_files(
    files: &[PathBuf],
    base: &Path,
    webserver: &str,
    playlist_path: &Path,
) {
    let web_folder = base.file_name().unwrap().to_str().unwrap();

    let mut playlist = String::from("#EXTM3U\n");
    playlist.push_str("https://raw.githubusercontent.com/QuackHack-McBlindy/share/main/intro.mp4\n");
    for f in files {
        let rel = f.strip_prefix(base).unwrap();
        let encoded = rel.to_string_lossy().replace(' ', "%20");
        playlist.push_str(&format!("{}/{}/{}\n", webserver, web_folder, encoded));
    }
    std::fs::write(playlist_path, playlist).expect("Failed to write playlist");
}

fn play_youtube_video(ip: &str, video_url: &str, wake_key: &str) {
    wake_and_connect(ip, wake_key);
    let cmd = format!(
        "am start -a android.intent.action.VIEW -d \"{}\" com.google.android.youtube.tv",
        video_url
    );
    adb(ip, &["shell", &cmd]);
}

fn get_current_activity(ip: &str) -> Option<String> {
    let out = adb_output_string(ip, &["shell", "dumpsys", "window", "windows"]);
    for line in out.lines() {
        if line.contains("mCurrentFocus") || line.contains("mFocusedApp") {
            if let Some(token) = line
                .split_whitespace()
                .last()
                .map(|t| t.trim_end_matches('}'))
            {
                if token.contains('/') {
                    return Some(token.to_string());
                }
            }
        }
    }
    None
}

fn get_current_track(ip: &str) -> Option<String> {
    let out = adb_output_string(ip, &["shell", "dumpsys", "media_session"]);
    for line in out.lines() {
        if let Some(pos) = line.find("description=") {
            let rest = &line[pos + "description=".len()..];
            let track = rest.split(',').next().unwrap_or(rest).trim();
            if !track.is_empty() {
                return Some(track.to_string());
            }
        }
    }
    None
}

fn adb_output_string(ip: &str, cmd: &[&str]) -> String {
    let output = Command::new("adb")
        .args(["-s", ip])
        .args(cmd)
        .output()
        .expect("adb not found or not executable");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn open_app(ip: &str, apps: &HashMap<String, String>, app: &str) -> Result<(), String> {
    let target_activity = apps
        .get(app)
        .ok_or_else(|| format!("App '{}' not found in config", app))?;

    if let Some(current) = get_current_activity(ip) {
        if current.as_str() == target_activity.as_str() {
            println!("App {} is already active: {}", app, current);
            return Ok(());
        }
    }

    println!("Opening app {}: {}", app, target_activity);
    let cmd = format!("am start -n {}", target_activity);
    let output = adb(ip, &["shell", &cmd]);
    if output.status.success() {
        Ok(())
    } else { Err(format!("Failed to open app {}", app)) }
}

fn control_device(
    ip: &str,
    keymap: &HashMap<String, String>,
    apps: &HashMap<String, String>,
    action: &str,
) -> Result<(), String> {
    if let Some(app) = action.strip_prefix("open_") {
        return open_app(ip, apps, app);
    }

    if action == "find_remote" {
        let cmd = "am start -n com.nvidia.remotelocator/.ShieldRemoteLocatorActivity";
        let output = adb(ip, &["shell", cmd]);
        return if output.status.success() {
            Ok(())
        } else { Err("find_remote failed".to_string()) };
    }

    let key_name = match action {
        "power_on" => "power_on",
        "power_off" => "power_off",
        "play_pause" => "play_pause",
        "next" => "next",
        "previous" => "previous",
        "volume_up" => "volume_up",
        "volume_down" => "volume_down",
        "channel_up" => "channel_up",
        "channel_down" => "channel_down",
        "nav_up" => "nav_up",
        "nav_down" => "nav_down",
        "nav_left" => "nav_left",
        "nav_right" => "nav_right",
        "nav_select" => "nav_select",
        "nav_back" => "nav_back",
        "nav_home" => "nav_home",
        "nav_menu" => "nav_menu",
        "nav_recents" => "nav_recents",
        _ => return Err(format!("Unknown action: {}", action)),
    };

    let keycode = keymap
        .get(key_name)
        .ok_or_else(|| format!("Keymap missing '{}' for action '{}'", key_name, action))?;
    adb_keyevent(ip, keycode);
    Ok(())
}

fn start_channel(ip: &str, channel_number: &str) {
    adb(ip, &["shell", "input", "keyevent", "KEYCODE_CLEAR"]);
    std::thread::sleep(std::time::Duration::from_millis(300));

    for ch in channel_number.chars() {
        let digit = ch.to_string();
        let digit_str = digit.as_str();
        adb(ip, &["shell", "input", "text", digit_str]);
        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    std::thread::sleep(std::time::Duration::from_millis(1500));
    adb(ip, &["shell", "input", "keyevent", "KEYCODE_ENTER"]);
}

fn execute_channel_command(
    ip: &str,
    keymap: &HashMap<String, String>,
    apps: &HashMap<String, String>,
    cmd: &str,
) {
    for raw_part in cmd.split("&&") {
        let part = raw_part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some(secs) = part.strip_prefix("wait ") {
            if let Ok(s) = secs.trim().parse::<u64>() {
                println!("Waiting {} seconds", s);
                std::thread::sleep(std::time::Duration::from_secs(s));
            }
        } else if let Some(channel_number) = part.strip_prefix("start_channel_") {
            if channel_number.chars().all(|c| c.is_ascii_digit()) {
                println!("Starting channel {}", channel_number);
                start_channel(ip, channel_number);
            }
        } else if part.starts_with("open_")
            || part.starts_with("nav_")
            || [
                "power_on",
                "power_off",
                "play_pause",
                "next",
                "previous",
                "volume_up",
                "volume_down",
                "channel_up",
                "channel_down",
                "nav_home",
                "nav_menu",
                "nav_back",
                "nav_recents",
                "find_remote",
            ]
            .contains(&part)
        {
            if let Err(e) = control_device(ip, keymap, apps, part) {
                eprintln!("Error executing '{}': {}", part, e);
            }
        } else { eprintln!("Unknown channel command: {}", part); }
    }
}

fn play_livetv_channel(
    ip: &str,
    keymap: &HashMap<String, String>,
    apps: &HashMap<String, String>,
    channel_id: &str,
    channel: &ChannelConfig,
) {
    println!("Playing channel {}: {}", channel_id, channel.name);

    let _ = control_device(ip, keymap, apps, "power_on");
    std::thread::sleep(std::time::Duration::from_secs(5));

    if let Some(cmd) = channel.cmd.as_deref() {
        if !cmd.is_empty() {
            println!("Using custom channel command: {}", cmd);
            execute_channel_command(ip, keymap, apps, cmd);
            return;
        }
    }

    if let Some(stream_url) = channel.stream_url.as_deref() {
        if !stream_url.is_empty() {
            println!("Using stream URL: {}", stream_url);
            let cmd = format!("am start -a android.intent.action.VIEW -d \"{}\"", stream_url);
            adb(ip, &["shell", &cmd]);
            return;
        }
    }

    if let Some(id) = channel.id {
        println!("Using numeric channel ID: {}", id);
        start_channel(ip, &id.to_string());
        return;
    }

    println!("Using default channel number input");
    start_channel(ip, channel_id);
}

fn resolve_channel(
    channels: &HashMap<String, ChannelConfig>,
    search: Option<&str>,
) -> Option<(String, ChannelConfig)> {
    if let Some(search) = search {
        if let Some(channel) = channels.get(search) {
            return Some((search.to_string(), channel.clone()));
        }

        let lower = search.to_lowercase();
        for (id, channel) in channels {
            if channel.name.to_lowercase().contains(&lower) {
                return Some((id.clone(), channel.clone()));
            }
        }
    } else {
        println!("Available channels:");
        for (id, channel) in channels {
            println!("  {}: {}", id, channel.name);
        }
    }
    None
}



// MAIN
fn main() {
    let mut args = Args::parse();

    if let Ok(env_path) = env::var("TV_CTL_CONFIG") {
        if args.config == PathBuf::from("/etc/zigduck/tv-defaults.json") {
            args.config = PathBuf::from(env_path);
        }
    }

    let config = load_config(&args.config);
    let tv = resolve_tv(&args, &config);
    let device_ip = &tv.ip;
    let keymap = &tv.keymap;
    let webserver_url = config.webserver();
    let youtube_api_key = config.youtube_api_key();
    let media_root = Path::new(
        config.directories.get("root")
            .expect("Media root directory missing in config")
    );

    let playlist_rel = Path::new(&config.playlist_file)
        .strip_prefix(media_root)
        .expect("playlist_file must be inside the media root directory")
        .to_string_lossy()
        .replace(' ', "%20");

    let favourites_rel = Path::new(&config.favourites_file)
        .strip_prefix(media_root)
        .expect("favourites_file must be inside the media root directory")
        .to_string_lossy()
        .replace(' ', "%20");

    let shuffle = args.shuffle.unwrap_or_else(|| {
        if args.no_shuffle {
            false
        } else if args.season.is_some() {
            false
        } else { config.shuffle }
    });
    
    let max_items = args.max_items.unwrap_or(config.max_items);

    let typ = args.typ.to_lowercase();

    // direct key events
    match typ.as_str() {
        "on" => {
            wake_and_connect(&device_ip, &keymap["power_on"]);
            println!("Device woken up");
            return;
        }
        "off" => {
            wake_and_connect(&device_ip, &keymap["power_on"]);
            adb_keyevent(&device_ip, &keymap["power_off"]);
            println!("Device put to sleep");
            return;
        }
        "up" => {
            adb_keyevent(&device_ip, &keymap["volume_up"]);
            adb_keyevent(&device_ip, &keymap["volume_up"]);
            println!("Increased the volume.");
            return;
        }
        "down" => {
            for _ in 0..3 {
                adb_keyevent(&device_ip, &keymap["volume_down"]);
            }
            println!("Lowered the volume.");
            return;
        }
        "next" => {
            adb_keyevent(&device_ip, &keymap["next"]);
            println!("Playing next track/episode.");
            return;
        }
        "prev" | "previous" => {
            adb_keyevent(&device_ip, &keymap["previous"]);
            println!("Playing previous track/episode.");
            return;
        }
        "pause" | "play" => {
            adb_keyevent(&device_ip, &keymap["play_pause"]);
            return;
        }
        "nav_up" => {
            adb_keyevent(&device_ip, &keymap["nav_up"]);
            println!("Navigating up.");
            return;
        }
        "nav_down" => {
            adb_keyevent(&device_ip, &keymap["nav_down"]);
            println!("Navigating down.");
            return;
        }
        "nav_left" => {
            adb_keyevent(&device_ip, &keymap["nav_left"]);
            println!("Navigating left.");
            return;
        }
        "nav_right" => {
            adb_keyevent(&device_ip, &keymap["nav_right"]);
            println!("Navigating right.");
            return;
        }
        "nav_select" => {
            adb_keyevent(&device_ip, &keymap["nav_select"]);
            println!("Selecting.");
            return;
        }
        "nav_menu" => {
            adb_keyevent(&device_ip, &keymap["nav_menu"]);
            println!("Opening menu.");
            return;
        }
        "nav_back" => {
            adb_keyevent(&device_ip, &keymap["nav_back"]);
            println!("Going back.");
            return;
        }
        "channel_up" => {
            adb_keyevent(&device_ip, &keymap["channel_up"]);
            println!("Channel up.");
            return;
        }
        "channel_down" => {
            adb_keyevent(&device_ip, &keymap["channel_down"]);
            println!("Channel down.");
            return;
        }
        "nav_home" => {
            adb_keyevent(&device_ip, &keymap["nav_home"]);
            println!("Going to home screen.");
            return;
        }
        "nav_recents" => {
            adb_keyevent(&device_ip, &keymap["nav_recents"]);
            println!("Opening recent apps.");
            return;
        }
        "play_playlist" => {
            let url = webserver_url.as_ref().expect("Webserver URL is required. Set `config.house.https.urlFile` in configuration.nix");
            let playlist_url = format!("{}/{}", url, playlist_rel);
            play_playlist(&device_ip, &playlist_url, &keymap["power_on"]);
            return;
        }
        "favourites" | "starred" => {
            let url = webserver_url.as_ref()
                .expect("Webserver URL is required. Set `config.house.https.urlFile` in configuration.nix");
            let playlist_url = format!("{}/{}", url, favourites_rel);
            play_playlist(&device_ip, &playlist_url, &keymap["power_on"]);
            return;
        }
        "star" | "like" => {
            let favourites_path = Path::new(&config.favourites_file);
            let current_track = get_current_track(&device_ip);
            match current_track {
                Some(track) => {
                    let existing = std::fs::read_to_string(favourites_path)
                        .unwrap_or_default();

                    if existing.lines().any(|line| line == track) {
                        println!("Track already in favourites: {}", track);
                    } else {
                        let mut file = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(favourites_path)
                            .expect("Failed to open favourites file for appending");

                        use std::io::Write;
                        writeln!(file, "{}", track)
                            .expect("Failed to write to favourites file");
                        println!("Added to favourites: {}", track);
                    }
                }
                None => { eprintln!("Could not determine current track. Is something playing?"); }
            }
            return;
        }
        "call" => {
            wake_and_connect(&device_ip, &keymap["power_on"]);
            let cmd = "am start -n com.nvidia.remotelocator/.ShieldRemoteLocatorActivity";
            adb(&device_ip, &["shell", cmd]);
            println!("Calling remote... beep ... beep!");
            return;
        }        
        "youtube" => {
            let query = match &args.search {
                Some(q) => q,
                None => {
                    eprintln!("Search query required for YouTube");
                    return;
                }
            };
            let api_key = youtube_api_key.as_ref().expect("YouTube API key is required for YouTube search. Set `config.house.media.youtubePasswordFile` in configuration.nix");
            match search_youtube(query, api_key) {
                Ok((video_url, title)) => {
                    println!("Playing YouTube video: {}", title);
                    play_youtube_video(&device_ip, &video_url, &keymap["power_on"]);
                }
                Err(e) => eprintln!("YouTube search failed: {}", e),
            }
            return;
        }
        
        _ => {}
    }

    if typ == "livetv" {
        let tv = resolve_tv(&args, &config);
        let device_ip = &tv.ip;
        let keymap = &tv.keymap;
        let apps = &tv.apps;
        let channels = &tv.channels;

        if let Some((channel_id, channel)) = resolve_channel(channels, args.search.as_deref()) {
            play_livetv_channel(device_ip, keymap, apps, &channel_id, &channel);
        } else { eprintln!("Channel not found or no channel specified"); }
        return;
    }

    // jukebox: all music shuffled
    if typ == "jukebox" {
        let url = webserver_url.as_ref().expect("Webserver URL is required. Set `config.house.https.urlFile` in configuration.nix");
        let music_dir = config.directories.get("music").expect("music dir missing");
        let path = Path::new(music_dir);
        generate_folder_playlist(
            path,
            path,
            url,
            true,
            max_items,
            Path::new(&config.playlist_file),
        );
        let playlist_url = format!("{}/{}", url, playlist_rel);
        play_playlist(&device_ip, &playlist_url, &keymap["power_on"]);
        return;
    }


    // directory‑based types
    let dir_types = ["tv", "movie", "podcast", "music", "musicvideo", "audiobook"];
    if dir_types.contains(&typ.as_str()) {
        let dir_key = &typ;
        let base = match config.directories.get(dir_key) {
            Some(d) => PathBuf::from(d),
            None => {
                eprintln!("Unknown type '{}'", typ);
                return;
            }
        };

        let mut target_path = base.clone();
        let query = args.search.as_deref().unwrap_or("");
        if !query.is_empty() {
            if let Some(found) = fuzzy_match_dir(&base, query) {
                target_path = base.join(found);
                println!("Matched folder: {}", target_path.display());
            } else {
                eprintln!("No match for '{}'", query);
                return;
            }
        }

        // handle season inside a TV show
        if let Some(season) = &args.season {
            let season_num: u32 = season.parse().unwrap_or(1);
            let season_candidates = [
                format!("Season {}", season_num),
                format!("Season {:02}", season_num),
                format!("S{:02}", season_num),
                format!("s{:02}", season_num),
            ];
            let mut season_dir = None;
            for cand in &season_candidates {
                let p = target_path.join(cand);
                if p.is_dir() {
                    season_dir = Some(p);
                    break;
                }
            }
            if let Some(dir) = season_dir {
                target_path = dir;
            } else {
                eprintln!("Season folder not found in '{}'", target_path.display());
                return;
            }
        }

        let url = webserver_url.as_ref().expect("Webserver URL is required. Set `config.house.https.urlFile` in configuration.nix");
        generate_folder_playlist(
            &target_path,
            &base,
            url,
            shuffle,
            max_items,
            Path::new(&config.playlist_file),
        );
        let playlist_url = format!("{}/{}", url, playlist_rel);
        play_playlist(&device_ip, &playlist_url, &keymap["power_on"]);
        return;
    }

    // file‑based types
    if typ == "song" || typ == "othervideo" {
        let (dir_key, exts): (&str, &[&str]) = if typ == "song" {
            ("music", &["mp3", "flac", "m4a", "wav"][..])
        } else { ("othervideo", &["mp4", "mkv", "avi", "mov"][..]) };

        let base = config.directories.get(dir_key).unwrap();
        let base_path = Path::new(base);
        let query = args.search.as_deref().unwrap_or("");
        if query.is_empty() {
            eprintln!("A search query is required for '{}'", typ);
            return;
        }
        let matches = fuzzy_match_files(base_path, query, exts);
        if matches.is_empty() {
            eprintln!("No files matched '{}'", query);
            return;
        }
        let files: Vec<PathBuf> = matches.into_iter().map(|(_, p)| p).collect();
        let url = webserver_url.as_ref().expect("Webserver URL is required. Set `config.house.https.urlFile` in configuration.nix");
        make_playlist_from_files(
            &files,
            base_path,
            url,
            Path::new(&config.playlist_file),
        );
        let playlist_url = format!("{}/{}", url, playlist_rel);
        play_playlist(&device_ip, &playlist_url, &keymap["power_on"]);
        return;
    }

    eprintln!("Unsupported type: {}", typ);
}
