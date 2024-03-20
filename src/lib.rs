use rocket::response::status;
use rocket::http::Status;

use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

use serde_json::Value;
use regex::Regex;
use rand::Rng;
use glob::glob;
use std::fs;

pub async fn find_duration(url: &str) -> Result<i64, std::io::Error> {
    let metadata_output = tokio::process::Command::new("youtube-dl")
        .args(&["-j", url])
        .output()
        .await?;

    if !metadata_output.status.success() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to get metadata"));
    }

    let metadata_str = String::from_utf8_lossy(&metadata_output.stdout).to_string();

    Ok(
        serde_json::from_str::<Value>(&metadata_str).unwrap()
            .get("duration").and_then(Value::as_i64).unwrap_or(0)
    )
}

pub async fn download_song(title: &str, url: &str) -> Result<(), std::io::Error> {
    let output_path = format!("./mp3list/{}.mp3", title);

    let status = tokio::process::Command::new("youtube-dl")
        .args(&["-x", "--audio-format", "mp3", "-o", &output_path, url])
        .status()
        .await?;

    if status.success() {
        Ok(())
    }
    else {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "youtube-dl command failed"))
    }
}

pub async fn play_song(title: &str) -> Result<(), std::io::Error> {
    let mp3path = format!("./mp3list/{}.mp3", title);

    let status = tokio::process::Command::new("mpg123")
        .args(&[mp3path])
        .status()
        .await?;

    if status.success() {
        Ok(())
    }
    else {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "mpg123 command failed"))
    }
}

pub async fn del_song(title: &str) -> Result<(), std::io::Error> {
    let pattern = format!("./mp3list/{}.*", title);

    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                if let Err(e) = fs::remove_file(path) {
                    return Err(e);
                }
            },
            Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }

    Ok(())
}

pub async fn stop_song() -> Result<(), std::io::Error> {
    let status = tokio::process::Command::new("pkill")
        .arg("mpg123")
        .status()
        .await?;

    if status.success() {
        Ok(())
    }
    else {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "mpg123 command failed"))
    }
}

pub async fn read_json_from_file(file_path: &str) -> io::Result<Value> {
    let mut contents = String::new();
    File::open(file_path).await?.read_to_string(&mut contents).await?;
    Ok(serde_json::from_str(&contents)?)
}

pub async fn write_json_to_file(file_path: &str, json: &Value) -> io::Result<()> {
    let new_contents = serde_json::to_string(json)?;
    File::create(file_path).await?.write_all(new_contents.as_bytes()).await?;
    Ok(())
}

pub async fn del_json_to_file(
    file_path: &str, json: &mut Value, idx: usize
) -> Result<String, status::Custom<String>> {
    match json.as_array_mut() {
        Some(arr) if idx < arr.len() => {
            let title = arr[idx]["title"].as_str().unwrap_or_default();
    
            if let Err(e) = del_song(title).await {
                return Err(status::Custom(Status::InternalServerError, e.to_string()));
            }
    
            arr.remove(idx);

            match write_json_to_file(file_path, &json).await {
                Ok(_) => Ok("mp3 deleted successfully.".to_string()),
                Err(_) => Err(err_to_custom("Failed to write to file")),
            }
        },
        Some(_) => Err(err_to_custom("Index out of bounds")),
        None => Err(err_to_custom("Failed to parsing the JSON file")),
    }
}

pub fn is_valid_url(url: String) -> bool {
    let url_pattern = Regex::new(r#"^https://www\.youtube\.com/watch\?v=[^;&?/|<>'$\s"\\]{11}$"#).unwrap();
    url_pattern.is_match(&url)
}

pub fn rewrite_title(title: String) -> String {
    let special_chars = "\"'\\~!@#$%^&*()/.,:;`,[]{}=_-+|<>?";
    let mut new_title: String = title.chars()
        .enumerate()
        .filter_map(|(idx, ch)| {
            if idx <= 48 && !special_chars.contains(ch) {
                Some(ch)
            }
            else {
                None
            }
        })
        .collect();

    if title.chars().count() > 48 {
        new_title.push_str("...");
    }
    
    new_title.push_str(&format!("{:02}", rand::thread_rng().gen_range(0..100)));

    new_title
}

pub fn err_to_custom<T>(err: T) -> status::Custom<String> 
    where T: Into<String>
{
    status::Custom(Status::InternalServerError, err.into())
}
