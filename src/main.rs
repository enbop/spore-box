use rust_embed::Embed;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use uuid::Uuid;
use wstd::http::body::IncomingBody;
use wstd::http::server::{Finished, Responder};
use wstd::http::{IntoBody, Request, Response, StatusCode};
use wstd::io::{copy, empty};

#[derive(Embed)]
#[folder = "frontend/build"]
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
    let uri = request.uri();
    let path = uri.path();
    let method = request.method().as_str();

    match path {
        "/api/messages" => match method {
            "GET" => api_get_messages(request, responder).await,
            "POST" => api_send_message(request, responder).await,
            _ => method_not_allowed(responder).await,
        },
        "/api/messages/poll" => match method {
            "GET" => api_poll_messages(request, responder).await,
            _ => method_not_allowed(responder).await,
        },
        "/api/upload" => match method {
            "POST" => api_upload_file(request, responder).await,
            _ => method_not_allowed(responder).await,
        },
        "/" => http_home(request, responder).await,
        _ => {
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
        .body(json.into_body())
        .unwrap();
    responder.respond(response).await
}

async fn api_poll_messages(request: Request<IncomingBody>, responder: Responder) -> Finished {
    let uri = request.uri();
    let query = uri.query().unwrap_or("");

    let since_timestamp = parse_since_parameter(query);
    let new_messages = get_messages_since(&since_timestamp).unwrap_or_default();

    let response_data = serde_json::json!({
        "messages": new_messages,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    let json = serde_json::to_string(&response_data).unwrap_or_else(|_| "{}".to_string());

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(json.into_body())
        .unwrap();
    responder.respond(response).await
}

async fn api_send_message(mut request: Request<IncomingBody>, responder: Responder) -> Finished {
    let mut body_data = Vec::new();

    let copy_result = copy(
        request.body_mut(),
        &mut wstd::io::Cursor::new(&mut body_data),
    )
    .await;

    let send_request = if copy_result.is_ok() {
        let body_str = String::from_utf8_lossy(&body_data);
        serde_json::from_str::<SendMessageRequest>(&body_str).unwrap_or_else(|_| {
            SendMessageRequest {
                content: "Failed to parse request".to_string(),
                sender: "Unknown".to_string(),
                msg_type: "text".to_string(),
                filename: None,
            }
        })
    } else {
        SendMessageRequest {
            content: "Body read failed".to_string(),
            sender: "Unknown".to_string(),
            msg_type: "text".to_string(),
            filename: None,
        }
    };

    let message = Message {
        id: Uuid::new_v4().to_string(),
        content: send_request.content,
        sender: send_request.sender,
        timestamp: chrono::Utc::now().to_rfc3339(),
        msg_type: send_request.msg_type,
        filename: send_request.filename,
        file_size: None,
        mime_type: None,
    };

    // Save the message
    let _ = save_message(&message);

    let json = serde_json::to_string(&message).unwrap_or_else(|_| "{}".to_string());
    let response = Response::builder()
        .status(StatusCode::CREATED)
        .header("Content-Type", "application/json")
        .body(json.into_body())
        .unwrap();
    responder.respond(response).await
}

async fn api_upload_file(_request: Request<IncomingBody>, responder: Responder) -> Finished {
    // TODO: Implement file upload
    let response = Response::builder()
        .status(StatusCode::NOT_IMPLEMENTED)
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

fn parse_since_parameter(query: &str) -> String {
    for param in query.split('&') {
        if let Some((key, value)) = param.split_once('=') {
            if key == "since" {
                return value.replace("%20", " ").replace("%3A", ":");
            }
        }
    }
    "1970-01-01T00:00:00Z".to_string()
}

fn get_messages_since(since: &str) -> Result<Vec<Message>, std::io::Error> {
    let all_messages = load_messages()?;

    let since_time = match chrono::DateTime::parse_from_rfc3339(since) {
        Ok(time) => time,
        Err(_) => {
            return Ok(all_messages);
        }
    };

    let filtered_messages: Vec<Message> = all_messages
        .into_iter()
        .filter(|msg| {
            if let Ok(msg_time) = chrono::DateTime::parse_from_rfc3339(&msg.timestamp) {
                msg_time > since_time
            } else {
                true
            }
        })
        .collect();

    Ok(filtered_messages)
}

async fn http_home(_request: Request<IncomingBody>, responder: Responder) -> Finished {
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
    let file_path = path.trim_start_matches('/');

    if file_path.is_empty() {
        return Assets::get("index.html").map(|f| (f, "index.html".to_string()));
    }

    if let Some(file) = Assets::get(file_path) {
        return Some((file, file_path.to_string()));
    }

    let index_path = format!("{}/index.html", file_path.trim_end_matches('/'));
    Assets::get(&index_path).map(|f| (f, index_path))
}

async fn serve_asset(
    file: rust_embed::EmbeddedFile,
    file_path: &str,
    responder: Responder,
) -> Finished {
    let mut response = Response::builder();

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
