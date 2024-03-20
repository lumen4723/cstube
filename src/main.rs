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
    is_valid_url, rewrite_title, find_duration, err_to_custom
};

lazy_static! {
    static ref IS_PLAYING: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static ref IS_STOPPING: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Song {
    title: String,
    url: String,
    duration: i64,
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
        response.set_header(
            rocket::http::Header::new("Access-Control-Allow-Origin", "*")
        );
        response.set_header(
            rocket::http::Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS")
        );
        response.set_header(
            rocket::http::Header::new("Access-Control-Allow-Headers", "*")
        );
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
        let mut title = items[i]["snippet"]["title"].as_str().unwrap_or("").to_string();
        title = rewrite_title(title);

        let video_id = &items[i]["id"]["videoId"].as_str().unwrap_or("");

        let url = format!("https://www.youtube.com/watch?v={}", video_id);
        result.push_str(&format!("{{\"title\": \"{}\", \"url\": \"{}\", \"duration\": -1}}", title, url));
        if i < items.len() - 1 {result.push_str(",");}
    }
    result.push_str("]");
    
    result
}

#[get("/url")]
pub async fn geturl() -> Json<Value> {
    let file_path = "./mp3list/index.json";

    return match read_json_from_file(file_path).await {
        Ok(json) => Json(json),
        Err(_) => Json(Value::Array(vec![])),
    };
}

#[post("/url", format = "json", data = "<sdata>")]
pub async fn addurl(mut sdata: Json<Song>) -> Result<String, status::Custom<String>> {
    let file_path = "./mp3list/index.json";

    let mut json = match read_json_from_file(file_path).await {
        Ok(json) => json,
        Err(_) => Value::Array(vec![]),
    };

    if !is_valid_url(sdata.url.clone()) || sdata.duration != -1 {
        return Err(err_to_custom("Invaild URL"));
    }

    sdata.title = rewrite_title(sdata.title.clone());

    sdata.duration = match find_duration(&sdata.url.clone()).await {
        Ok(x) => x,
        Err(_) => -1,
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
    {
        let mut is_playing = IS_PLAYING.lock().await;
        if *is_playing {
            return "Already playing. Please wait.".to_string();
        }
    
        *is_playing = true;
    }

    let file_path = "./mp3list/index.json";

    spawn(async move {
        loop {
            let json = match read_json_from_file(file_path).await {
                Ok(json) => Arc::new(Mutex::new(json)),
                Err(_) => Arc::new(Mutex::new(Value::Array(vec![]))),
            };
        
            let json_clone = Arc::clone(&json);
            let file_path_clone = file_path.to_string();

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
                        if *is_stopping { break; }
                    }

                    let json = match read_json_from_file(file_path).await {
                        Ok(json) => Arc::new(Mutex::new(json)),
                        Err(_) => Arc::new(Mutex::new(Value::Array(vec![]))),
                    };
                    let mut json = json.lock().await;
                    let _ = del_json_to_file(&file_path_clone, &mut json, 0).await;
                    continue;
                }
            }

            break;
        }

        let mut is_playing = IS_PLAYING.lock().await;
        *is_playing = false;
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

    {
        let mut is_playing = IS_PLAYING.lock().await;
        if *is_playing {
            return "Already playing. Please wait.".to_string();
        }
    
        *is_playing = true;
    }

    let file_path = "./mp3list/index.json";

    spawn(async move {
        {
            let json = match read_json_from_file(file_path).await {
                Ok(json) => Arc::new(Mutex::new(json)),
                Err(_) => Arc::new(Mutex::new(Value::Array(vec![]))),
            };

            let json_clone = Arc::clone(&json);
            let file_path_clone = file_path.to_string();

            let mut json = json_clone.lock().await;
            let _ = del_json_to_file(&file_path_clone, &mut json, 0).await;
        }

        loop {
            let json = match read_json_from_file(file_path).await {
                Ok(json) => Arc::new(Mutex::new(json)),
                Err(_) => Arc::new(Mutex::new(Value::Array(vec![]))),
            };
            
            let json_clone = Arc::clone(&json);
            let file_path_clone = file_path.to_string();

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
                        if *is_stopping { break; }
                    }

                    let json = match read_json_from_file(file_path).await {
                        Ok(json) => Arc::new(Mutex::new(json)),
                        Err(_) => Arc::new(Mutex::new(Value::Array(vec![]))),
                    };
                    let mut json = json.lock().await;
                    let _ = del_json_to_file(&file_path_clone, &mut json, 0).await;
                    continue;
                }
            }
            
            break;
        }

        let mut is_playing = IS_PLAYING.lock().await;
        *is_playing = false;
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
