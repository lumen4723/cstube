use rocket::response::status;
use rocket::http::Status;

use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

use serde_json::Value;

pub async fn download_song(title: &str, url: &str) -> Result<(), std::io::Error> {
    // let title = rewrite_title(title);
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
    // let title = rewrite_title(title);
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
    // let title = rewrite_title(title);
    let mp3path = format!("./mp3list/{}.mp3", title);

    let status = tokio::process::Command::new("rm")
        .args(&[mp3path])
        .status()
        .await?;

    if status.success() {
        Ok(())
    }
    else {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "rm command failed"))
    }
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

pub fn err_to_custom<T>(err: T) -> status::Custom<String> 
    where T: Into<String>
{
    status::Custom(Status::InternalServerError, err.into())
}

pub fn is_escape_or_special_char(ch: char) -> bool {
    ch == '\"' || ch == '\'' || ch == '\\' || ch == '~'
    || ch == '!' || ch == '@' || ch == '#' || ch == '$'
    || ch == '%' || ch == '^' || ch == '&' || ch == '*'
    || ch == '(' || ch == ')' || ch == '/' || ch == '.'
    || ch == ':' || ch == ';' || ch == '`' || ch == ','
    || ch == '[' || ch == ']' || ch == '{' || ch == '}'
    || ch == '=' || ch == '_' || ch == '-' || ch == '+'
    || ch == '|' || ch == '<' || ch == '>' || ch == '?'
}