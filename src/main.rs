#[macro_use] extern crate rocket;

use rocket::{Request, Response};
use rocket::response::status;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::fs::{NamedFile, FileServer};
use rocket::serde::json::Json;

use serde_json::{json, Value};
use serde::{Deserialize, Serialize};

use reqwest;

use tokio::spawn;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::{thread, time};
use lazy_static::lazy_static;

use lib:: {
    read_json_from_file, write_json_to_file, del_json_to_file,
    download_song, play_song, stop_song,
    err_to_custom, is_escape_or_special_char
};

lazy_static! {
    static ref IS_PLAYING: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static ref IS_STOPPING: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Song {
    title: String,
    url: String,
}

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "",
            kind: Kind::Response
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS"));
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Headers", "*"));
    }
}

#[get("/")]
pub async fn index() -> Option<NamedFile> {
    NamedFile::open("./view/index.html").await.ok()
}

#[get("/search?<word>")]
pub async fn search(word: String) -> String {
    let api_key = "<input your youtube api-key>";
    let request_url = format!(
        "https://www.googleapis.com/youtube/v3/search?part=snippet&maxResults=10&type=video&q={}&key={}",
        word, api_key
    );

    let body = reqwest::get(&request_url).await.unwrap()
        .text().await.unwrap()
        .parse::<serde_json::Value>().unwrap();

    let items = body["items"].as_array().unwrap();

    let mut result = "[".to_string();

    for i in 0..items.len() {
        let title = items[i]["snippet"]["title"].as_str().unwrap_or("").to_string();
        let mut char_indices = title.char_indices().peekable();

        let mut title = String::new();
        while let Some((_idx, ch)) = char_indices.next() {
            if char_indices.peek().is_some() && char_indices.peek().unwrap().0 > 48 {
                title.push_str("...");
                break;
            }
            else if !is_escape_or_special_char(ch) {
                title.push(ch);
            }
        }
        title.push((i as u8 + '0' as u8) as char);

        let video_id = &items[i]["id"]["videoId"].as_str().unwrap_or("");


        let url = format!("https://www.youtube.com/watch?v={}", video_id);
        result.push_str(&format!("{{\"title\": \"{}\", \"url\": \"{}\"}}", title, url));
        if i < items.len() - 1 {result.push_str(",");}
    }
    
    result.push_str("]");
    
    result
}

#[get("/url")]
pub async fn geturl() -> Json<Value> {
    let file_path = "./mp3list/index.json";

    match read_json_from_file(file_path).await {
        Ok(json) => {
            return Json(json);
        },
        Err(_) => {
            return Json(Value::Array(vec![]));
        },
    };
}

#[post("/url", format = "json", data = "<sdata>")]
pub async fn addurl(sdata: Json<Song>) -> Result<String, status::Custom<String>> {
    let file_path = "./mp3list/index.json";

    let mut json = match read_json_from_file(file_path).await {
        Ok(json) => json,
        Err(_) => Value::Array(vec![]),
    };

    if let Some(arr) = json.as_array_mut() {
        let sdata_clone = sdata.clone();
        arr.push(json!(sdata.into_inner()));
        
        if write_json_to_file(file_path, &json).await.is_err() {
            return Err(err_to_custom("Failed to write to file"));
        }

        if let Err(e) = download_song(&sdata_clone.title, &sdata_clone.url).await {
            return Err(err_to_custom(&format!("Failed to download song: {}", e)));
        }

        Ok("Song added successfully".to_string())
    }
    else {
        Err(err_to_custom("Failed to update the JSON file"))
    }
}

#[delete("/url/<idx>")]
pub async fn delurl(idx: usize) -> Result<String, status::Custom<String>> {
    let file_path = "./mp3list/index.json";

    let mut json = match read_json_from_file(file_path).await {
        Ok(json) => json,
        Err(_) => Value::Array(vec![]),
    };

    del_json_to_file(file_path, &mut json, idx).await
}

#[post("/play")]
pub async fn play() -> String {
    let file_path = "./mp3list/index.json";

    let json = match read_json_from_file(file_path).await {
        Ok(json) => Arc::new(Mutex::new(json)),
        Err(_) => Arc::new(Mutex::new(Value::Array(vec![]))),
    };

    let json_clone = Arc::clone(&json);
    let file_path_clone = file_path.to_string();

    spawn(async move {
        loop {
            let title = {
                let json = json_clone.lock().await;
                if json.as_array().map(|arr| arr.is_empty()).unwrap_or(true) {
                    break;
                }
                json.as_array()
                    .and_then(|arr| arr.get(0))
                    .and_then(|item| item.get("title"))
                    .and_then(|title| title.as_str())
                    .map(ToString::to_string)
            };

            if let Some(title) = title {
                {
                    let mut is_stopping = IS_STOPPING.lock().await;
                    *is_stopping = false;
                }

                let _ = IS_PLAYING.lock().await;
                if play_song(&title).await.is_ok() {
                    {
                        let is_stopping = IS_STOPPING.lock().await;
                        if *is_stopping {
                            break;
                        }
                    }

                    let mut json = json_clone.lock().await;
                    let _ = del_json_to_file(&file_path_clone, &mut json, 0).await;
                    continue;
                }
            }

            break;
        }
    });

    "Request received, processing...".to_string()
}

#[post("/stop")]
pub async fn stop() -> String {
    {
        let mut is_stopping = IS_STOPPING.lock().await;
        *is_stopping = true;
    }
    
    match stop_song().await {
        Ok(_) => "Song stoped successfully.".to_string(),
        Err(e) => format!("Error stoping song: {}", e),
    }
}

#[post("/next")]
pub async fn next() -> String {
    let file_path = "./mp3list/index.json";

    let json = match read_json_from_file(file_path).await {
        Ok(json) => Arc::new(Mutex::new(json)),
        Err(_) => Arc::new(Mutex::new(Value::Array(vec![]))),
    };

    let json_clone = Arc::clone(&json);
    let file_path_clone = file_path.to_string();

    spawn(async move {
        loop {
            let title = {
                let mut json = json_clone.lock().await;
                if json.as_array().map(|arr| arr.is_empty()).unwrap_or(true) {
                    break;
                }
                
                let mut title = String::new();
                let _ = del_json_to_file(&file_path_clone, &mut json, 0).await;
            
                if let Some(arr) = json.as_array_mut() {
                    if !arr.is_empty() {
                        title = arr[0]["title"].clone().as_str().unwrap_or_default().to_string();
                    }
                };
                
                match write_json_to_file(&file_path_clone, &json).await {
                    Ok(_) => Some(title),
                    Err(_) => None,
                }
            };
        
            if let Some(title) = title {
                {
                    let mut is_stopping = IS_STOPPING.lock().await;
                    *is_stopping = true;
                }
                let _ = stop_song().await;
                thread::sleep(time::Duration::from_millis(32));
        
                {
                    let mut is_stopping = IS_STOPPING.lock().await;
                    *is_stopping = false;
                }
                
                let _ = IS_PLAYING.lock().await;
                if play_song(&title).await.is_ok() {
                    {
                        let is_stopping = IS_STOPPING.lock().await;
                        if *is_stopping {
                            break;
                        }
                    }

                    let mut json = json_clone.lock().await;
                    let _ = del_json_to_file(&file_path_clone, &mut json, 0).await;
                    continue;
                }
            }
            
            break;
        }
    });

    "Request received, processing...".to_string()
}



#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(CORS)
        .mount("/", routes![
            index, search,
            geturl, addurl, delurl,
            play, stop, next
        ])
        .mount("/", FileServer::from("./view"))
}
