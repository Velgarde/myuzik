use clap::{App, Arg, SubCommand};
use colored::*;
use dirs;
use regex::Regex;
use reqwest;
use rustyline::Editor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{File};
use std::path::{ PathBuf};
use std::process::Command;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;

#[derive(Serialize, Deserialize, Clone)]
struct Song {
    name: String,
    file_path: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Playlist {
    name: String,
    songs: Vec<Song>,
}

#[derive(Serialize, Deserialize)]
struct Myuzik {
    playlists: HashMap<String, Playlist>,
}

impl Myuzik {
    fn new() -> Self {
        Self {
            playlists: HashMap::new(),
        }
    }

    fn load() -> Result<Self, Box<dyn Error>> {
        let config_path = Self::config_path()?;
        if config_path.exists() {
            let file = File::open(config_path)?;
            let myuzik: Myuzik = serde_json::from_reader(file)?;
            Ok(myuzik)
        } else {
            Ok(Self::new())
        }
    }

    fn save(&self) -> Result<(), Box<dyn Error>> {
        let config_path = Self::config_path()?;
        let file = File::create(config_path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }

    fn config_path() -> Result<PathBuf, Box<dyn Error>> {
        let mut path = dirs::home_dir().ok_or("Unable to find home directory")?;
        path.push(".myuzik.json");
        Ok(path)
    }

    fn download_audio(&mut self, url: &str) -> Result<Song, Box<dyn Error>> {
        println!("{}", "Downloading audio...".blue());

        let yt_dlp_path = ensure_yt_dlp()?;
        let current_dir = env::current_dir()?;
        let storage_dir = current_dir.join("storage");

        // Create the storage directory if it doesn't exist
        if !storage_dir.exists() {
            fs::create_dir_all(&storage_dir)?;
            // Set permissions to read, write, and execute for the owner
            let mut perms = fs::metadata(&storage_dir)?.permissions();
            perms.set_mode(0o700);
            fs::set_permissions(&storage_dir, perms)?;
        }

        let output_template = storage_dir.join("%(title)s.%(ext)s").to_string_lossy().to_string();

        let output = Command::new(&yt_dlp_path)
            .args(&[
                "-x",
                "--audio-format", "mp3",
                "--audio-quality", "0",
                "-o", &output_template,
                url,
            ])
            .output()?;

        if output.status.success() {
            let file_name = String::from_utf8(output.stdout)?.trim().to_string();
            let file_path = storage_dir.join(file_name).with_extension("mp3");

            Command::new(&yt_dlp_path)
                .args(&[
                    "-x",
                    "--audio-format", "mp3",
                    "--audio-quality", "0",
                    "-o", &output_template,
                    url,
                ])
                .status()?;

            if file_path.exists() {
                println!("{}", "Download complete!".green());
                Ok(Song {
                    name: file_path.file_name().unwrap().to_string_lossy().into_owned(),
                    file_path: file_path.to_string_lossy().into_owned(),
                })
            } else {
                Err("File not found after download".into())
            }
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(format!("Download failed: {}", error).into())
        }
    }

    fn add_to_playlist(&mut self, playlist_name: &str, song: Song) {
        self.playlists
            .entry(playlist_name.to_string())
            .or_insert_with(|| Playlist {
                name: playlist_name.to_string(),
                songs: Vec::new(),
            })
            .songs
            .push(song);
    }

    fn list_playlists(&self) {
        println!("{}", "Playlists:".green().bold());
        for (name, playlist) in &self.playlists {
            println!("  {} ({} songs)", name.yellow(), playlist.songs.len());
        }
    }

    fn list_songs(&self, playlist_name: &str) {
        if let Some(playlist) = self.playlists.get(playlist_name) {
            println!(
                "{} '{}':",
                "Songs in playlist".green().bold(),
                playlist_name.yellow()
            );
            for (i, song) in playlist.songs.iter().enumerate() {
                println!("  {}. {}", i + 1, song.name);
            }
        } else {
            println!("{}", "Playlist not found.".red());
        }
    }

    fn search_songs(&self, query: &str) -> Vec<(String, Song)> {
        let mut results = Vec::new();
        for (playlist_name, playlist) in &self.playlists {
            for song in &playlist.songs {
                if song.name.to_lowercase().contains(&query.to_lowercase()) {
                    results.push((playlist_name.clone(), song.clone()));
                }
            }
        }
        results
    }

    fn play_song(&self, playlist_name: &str, song_name: &str) -> Result<(), Box<dyn Error>> {
        if let Some(playlist) = self.playlists.get(playlist_name) {
            if let Some(song) = playlist.songs.iter().find(|s| s.name == song_name) {
                println!("{}", "Playing song...".blue());
                Command::new("cmd")
                    .args(&["/C", "start", "wmplayer", &song.file_path])
                    .spawn()?;
                Ok(())
            } else {
                Err("Song not found in playlist".into())
            }
        } else {
            Err("Playlist not found".into())
        }
    }
}

fn ensure_yt_dlp() -> Result<PathBuf, Box<dyn Error>> {
    let yt_dlp_url = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";
    let yt_dlp_path = dirs::cache_dir()
        .ok_or("Unable to find cache directory")?
        .join("yt-dlp.exe");

    if !yt_dlp_path.exists() {
        println!("Downloading yt-dlp...");
        let mut response = reqwest::blocking::get(yt_dlp_url)?;
        let mut file = File::create(&yt_dlp_path)?;
        response.copy_to(&mut file)?;
        println!("yt-dlp downloaded successfully.");
    }

    Ok(yt_dlp_path)
}

fn print_banner() {
    println!(
        "{}",
        r#"
 __  __                 _ _
|  \/  |               (_) |
| \  / |_   _ _   _ ___| | | __
| |\/| | | | | | | |_  / | |/ /
| |  | | |_| | |_| |/ /| |
|_|  |_|\__, |\__,_/___|_|_|\_\
         __/ |
        |___/
    "#
            .bright_cyan()
    );
}

fn is_valid_youtube_url(url: &str) -> bool {
    let re = Regex::new(r"^(https?://)?(www\.)?(youtube\.com|youtu\.be)/.*").unwrap();
    re.is_match(url)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut myuzik = Myuzik::load()?;
    let mut rl = Editor::<()>::new();

    print_banner();

    loop {
        let input = rl.readline("myuzik> ")?;
        let args = input.split_whitespace().collect::<Vec<_>>();

        let matches = App::new("Myuzik")
            .version("0.1.0")
            .author("Your Name")
            .about("Downloads YouTube audio and manages playlists")
            .subcommand(
                SubCommand::with_name("download")
                    .about("Download audio from YouTube")
                    .arg(
                        Arg::with_name("URL")
                            .help("YouTube video URL")
                            .required(true)
                            .index(1),
                    ),
            )
            .subcommand(SubCommand::with_name("list").about("List playlists"))
            .subcommand(
                SubCommand::with_name("songs")
                    .about("List songs in a playlist")
                    .arg(
                        Arg::with_name("PLAYLIST")
                            .help("Playlist name")
                            .required(true)
                            .index(1),
                    ),
            )
            .subcommand(
                SubCommand::with_name("search")
                    .about("Search for songs")
                    .arg(
                        Arg::with_name("QUERY")
                            .help("Search query")
                            .required(true)
                            .index(1),
                    ),
            )
            .get_matches_from_safe(args);

        match matches {
            Ok(matches) => match matches.subcommand() {
                ("download", Some(download_matches)) => {
                    let url = download_matches.value_of("URL").unwrap();
                    if !is_valid_youtube_url(url) {
                        println!("{}", "Invalid YouTube URL.".red());
                        continue;
                    }
                    let song = myuzik.download_audio(url)?;
                    let playlist_name = rl.readline("Enter playlist name to store the song: ")?;
                    myuzik.add_to_playlist(&playlist_name, song);
                    println!("{}", "Audio downloaded and added to playlist.".green());
                }
                ("list", _) => {
                    myuzik.list_playlists();
                }
                ("songs", Some(songs_matches)) => {
                    let playlist_name = songs_matches.value_of("PLAYLIST").unwrap();
                    myuzik.list_songs(playlist_name);
                }
                ("search", Some(search_matches)) => {
                    let query = search_matches.value_of("QUERY").unwrap();
                    let results = myuzik.search_songs(query);
                    println!("{}", "Search results:".green().bold());
                    for (i, (playlist, song)) in results.iter().enumerate() {
                        println!(
                            "  {}. {} (in playlist '{}')",
                            i + 1,
                            song.name,
                            playlist.yellow()
                        );
                    }
                    if !results.is_empty() {
                        let selection = rl.readline("Enter the number of the song to play (or press Enter to cancel): ")?;
                        if let Ok(index) = selection.parse::<usize>() {
                            if index > 0 && index <= results.len() {
                                let (playlist, song) = &results[index - 1];
                                myuzik.play_song(playlist, &song.name)?;
                            }
                        }
                    }
                }
                _ => {
                    println!("Use --help for usage information");
                }
            },
            Err(_) => {
                println!("Invalid command. Use --help for usage information.");
            }
        }

        myuzik.save()?;
    }
}