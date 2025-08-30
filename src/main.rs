use rust_embed::Embed;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use uuid::Uuid;
use wstd::http::body::IncomingBody;
use wstd::http::server::{Finished, Responder};
use wstd::http::{IntoBody, Request, Response, StatusCode};
use wstd::io::empty;

#[derive(Embed)]
#[folder = "dist/"]
struct Assets;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Message {
    id: String,
    content: String,
    sender: String,
    timestamp: String,
    #[serde(rename = "type")]
    msg_type: String,
    filename: Option<String>,
    #[serde(rename = "fileSize")]
    file_size: Option<u64>,
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
}

#[derive(Deserialize)]
struct SendMessageRequest {
    content: String,
    sender: String,
    #[serde(rename = "type")]
    msg_type: String,
    filename: Option<String>,
}

#[wstd::http_server]
async fn main(request: Request<IncomingBody>, responder: Responder) -> Finished {
    let path = request.uri().path_and_query().unwrap().as_str();
    match path {
        "/api/messages" => match request.method().as_str() {
            "GET" => api_get_messages(request, responder).await,
            "POST" => api_send_message(request, responder).await,
            _ => method_not_allowed(responder).await,
        },
        "/api/upload" => match request.method().as_str() {
            "POST" => api_upload_file(request, responder).await,
            _ => method_not_allowed(responder).await,
        },
        "/" => http_home(request, responder).await,
        _ => {
            // Try to serve static files
            if let Some((file, file_path)) = serve_static_file(path) {
                serve_asset(file, &file_path, responder).await
            } else {
                http_not_found(request, responder).await
            }
        }
    }
}

async fn method_not_allowed(responder: Responder) -> Finished {
    let response = Response::builder()
        .status(StatusCode::METHOD_NOT_ALLOWED)
        .body(empty())
        .unwrap();
    responder.respond(response).await
}

async fn api_get_messages(_request: Request<IncomingBody>, responder: Responder) -> Finished {
    let messages = load_messages().unwrap_or_default();
    let json = serde_json::to_string(&messages).unwrap_or_else(|_| "[]".to_string());

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(json.into_body())
        .unwrap();
    responder.respond(response).await
}

async fn api_send_message(_request: Request<IncomingBody>, responder: Responder) -> Finished {
    // For now, create a simple test message
    let test_message = Message {
        id: Uuid::new_v4().to_string(),
        content: "Hello from server".to_string(),
        sender: "Server".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        msg_type: "text".to_string(),
        filename: None,
        file_size: None,
        mime_type: None,
    };

    // Save the message
    let _ = save_message(&test_message);

    let json = serde_json::to_string(&test_message).unwrap_or_else(|_| "{}".to_string());
    let response = Response::builder()
        .status(StatusCode::CREATED)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(json.into_body())
        .unwrap();
    responder.respond(response).await
}

async fn api_upload_file(_request: Request<IncomingBody>, responder: Responder) -> Finished {
    // TODO: Implement file upload
    let response = Response::builder()
        .status(StatusCode::NOT_IMPLEMENTED)
        .header("Access-Control-Allow-Origin", "*")
        .body("File upload not implemented yet".into_body())
        .unwrap();
    responder.respond(response).await
}

fn load_messages() -> Result<Vec<Message>, std::io::Error> {
    let file = std::fs::File::open("data/messages.jsonl");
    match file {
        Ok(file) => {
            let reader = BufReader::new(file);
            let mut messages = Vec::new();
            for line in reader.lines() {
                if let Ok(line) = line {
                    if let Ok(message) = serde_json::from_str::<Message>(&line) {
                        messages.push(message);
                    }
                }
            }
            Ok(messages)
        }
        Err(_) => Ok(Vec::new()), // Return empty vec if file doesn't exist
    }
}

fn save_message(message: &Message) -> Result<(), std::io::Error> {
    // Ensure directory exists
    std::fs::create_dir_all("data")?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("data/messages.jsonl")?;

    let json = serde_json::to_string(message)?;
    writeln!(file, "{}", json)?;
    Ok(())
}

async fn http_home(_request: Request<IncomingBody>, responder: Responder) -> Finished {
    // Serve index.html for the home page
    if let Some((file, _)) = serve_static_file("/") {
        serve_asset(file, "index.html", responder).await
    } else {
        http_not_found(_request, responder).await
    }
}

async fn http_not_found(_request: Request<IncomingBody>, responder: Responder) -> Finished {
    let response = Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(empty())
        .unwrap();
    responder.respond(response).await
}

fn serve_static_file(path: &str) -> Option<(rust_embed::EmbeddedFile, String)> {
    // Remove leading slash and try to get the file from assets
    let file_path = path.trim_start_matches('/');

    // If empty path, serve index.html
    if file_path.is_empty() {
        return Assets::get("index.html").map(|f| (f, "index.html".to_string()));
    }

    // Try to get the file directly
    if let Some(file) = Assets::get(file_path) {
        return Some((file, file_path.to_string()));
    }

    // If it's a directory path, try to serve index.html from that directory
    let index_path = format!("{}/index.html", file_path.trim_end_matches('/'));
    Assets::get(&index_path).map(|f| (f, index_path))
}

async fn serve_asset(
    file: rust_embed::EmbeddedFile,
    file_path: &str,
    responder: Responder,
) -> Finished {
    let mut response = Response::builder();

    // Set appropriate Content-Type based on file extension
    if let Some(content_type) = get_content_type(file_path) {
        response = response.header("Content-Type", content_type);
    }

    let response = response.body(file.data.into_body()).unwrap();
    responder.respond(response).await
}

fn get_content_type(filename: &str) -> Option<&'static str> {
    let extension = filename.split('.').last()?;
    match extension.to_lowercase().as_str() {
        "html" => Some("text/html; charset=utf-8"),
        "css" => Some("text/css"),
        "js" => Some("application/javascript"),
        "json" => Some("application/json"),
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "svg" => Some("image/svg+xml"),
        "ico" => Some("image/x-icon"),
        "xml" => Some("application/xml"),
        "txt" => Some("text/plain"),
        _ => None,
    }
}
