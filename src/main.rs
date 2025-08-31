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
        _ if path.starts_with("/api/files/") => match method {
            "GET" => serve_uploaded_file(path, responder).await,
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

async fn api_upload_file(mut request: Request<IncomingBody>, responder: Responder) -> Finished {
    let mut body_data = Vec::new();

    let copy_result = copy(
        request.body_mut(),
        &mut wstd::io::Cursor::new(&mut body_data),
    )
    .await;

    if copy_result.is_err() {
        let response = Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("Failed to read request body".into_body())
            .unwrap();
        return responder.respond(response).await;
    }

    // Parse multipart form data manually
    let content_type = request
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !content_type.starts_with("multipart/form-data") {
        let response = Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("Expected multipart/form-data".into_body())
            .unwrap();
        return responder.respond(response).await;
    }

    // Extract boundary
    let boundary = if let Some(boundary_part) = content_type.split("boundary=").nth(1) {
        boundary_part.trim_matches('"')
    } else {
        let response = Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("Missing boundary in multipart data".into_body())
            .unwrap();
        return responder.respond(response).await;
    };

    // Parse multipart data
    let (file_data, filename, sender) = match parse_multipart_data(&body_data, boundary) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Multipart parsing error: {}", err);
            let response = Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(format!("Multipart parsing error: {}", err).into_body())
                .unwrap();
            return responder.respond(response).await;
        }
    };

    // Create uploads directory inside data folder
    if let Err(e) = std::fs::create_dir_all("data/uploads") {
        eprintln!("Failed to create upload directory: {}", e);
        let response = Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body("Failed to create upload directory".into_body())
            .unwrap();
        return responder.respond(response).await;
    }

    // Generate unique filename
    let file_id = Uuid::new_v4().to_string();
    let extension = filename
        .split('.')
        .last()
        .map(|ext| format!(".{}", ext))
        .unwrap_or_default();
    let stored_filename = format!("{}{}", file_id, extension);
    let file_path = format!("data/uploads/{}", stored_filename);

    // Save file
    if let Err(e) = std::fs::write(&file_path, &file_data) {
        eprintln!("Failed to save file: {}", e);
        let response = Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body("Failed to save file".into_body())
            .unwrap();
        return responder.respond(response).await;
    }

    // Determine message type based on file extension
    let msg_type = if is_image_file(&filename) {
        "image"
    } else {
        "file"
    };

    // Determine MIME type
    let mime_type = get_mime_type(&filename);

    // Create message
    let message = Message {
        id: Uuid::new_v4().to_string(),
        content: stored_filename, // Store the file ID/name
        sender,
        timestamp: chrono::Utc::now().to_rfc3339(),
        msg_type: msg_type.to_string(),
        filename: Some(filename),
        file_size: Some(file_data.len() as u64),
        mime_type: Some(mime_type.to_string()),
    };

    // Save message
    if let Err(e) = save_message(&message) {
        eprintln!("Failed to save message: {}", e);
        let response = Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body("Failed to save message".into_body())
            .unwrap();
        return responder.respond(response).await;
    }

    let json = serde_json::to_string(&message).unwrap_or_else(|_| "{}".to_string());
    let response = Response::builder()
        .status(StatusCode::CREATED)
        .header("Content-Type", "application/json")
        .body(json.into_body())
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

fn parse_multipart_data(data: &[u8], boundary: &str) -> Result<(Vec<u8>, String, String), String> {
    let boundary_start = format!("--{}", boundary);
    let boundary_end = format!("--{}--", boundary);

    let data_str = String::from_utf8_lossy(data);

    let mut file_data = Vec::new();
    let mut filename = String::new();
    let mut sender = String::from("Unknown");

    // Split by boundary markers
    let parts: Vec<&str> = data_str.split(&boundary_start).collect();

    for part in parts {
        if part.trim().is_empty() || part.starts_with("--") {
            continue;
        }

        // Split headers and body by double newline
        let sections: Vec<&str> = part.splitn(2, "\r\n\r\n").collect();
        if sections.len() < 2 {
            // Try with just \n\n
            let sections: Vec<&str> = part.splitn(2, "\n\n").collect();
            if sections.len() < 2 {
                continue;
            }
        }

        let headers = sections[0];
        let body = sections[1];

        // Check if this is a file part
        if headers.contains("filename=") {
            // Extract filename
            for line in headers.lines() {
                if line.contains("filename=") {
                    if let Some(start) = line.find("filename=\"") {
                        let start = start + 10; // length of "filename=\""
                        if let Some(end) = line[start..].find('"') {
                            filename = line[start..start + end].to_string();
                        }
                    }
                    break;
                }
            }

            // Extract file data - need to work with bytes, not string
            // Find the start position in the original byte array
            if let Some(body_start) = find_body_start_in_bytes(data, part) {
                if let Some(body_end) =
                    find_body_end_in_bytes(data, body_start, &boundary_start, &boundary_end)
                {
                    file_data = data[body_start..body_end].to_vec();
                }
            }
        } else if headers.contains("name=\"sender\"") {
            // Extract sender value
            sender = body
                .trim()
                .trim_end_matches(&boundary_end)
                .trim()
                .to_string();
        }
    }

    if file_data.is_empty() {
        return Err("No file data found".to_string());
    }

    if filename.is_empty() {
        return Err("No filename found".to_string());
    }

    Ok((file_data, filename, sender))
}

fn find_body_start_in_bytes(data: &[u8], part_str: &str) -> Option<usize> {
    // Find where this part starts in the original bytes
    let part_bytes = part_str.as_bytes();

    // Look for the double CRLF or double LF pattern in the part
    if let Some(relative_pos) = part_bytes.windows(4).position(|w| w == b"\r\n\r\n") {
        // Now find this pattern in the original data
        let pattern_start = relative_pos;
        let pattern = &part_bytes[pattern_start..pattern_start + 4];

        // Find all occurrences of this pattern in the original data
        for i in 0..data.len().saturating_sub(3) {
            if &data[i..i + 4] == pattern {
                // Verify this is the right position by checking some context
                let context_len = std::cmp::min(20, pattern_start);
                if pattern_start >= context_len {
                    let context = &part_bytes[pattern_start - context_len..pattern_start];
                    let data_context_start = i.saturating_sub(context_len);
                    if data_context_start < data.len() && i <= data.len() {
                        let data_context = &data[data_context_start..i];
                        if data_context == context {
                            return Some(i + 4);
                        }
                    }
                }
            }
        }
    }

    // Fallback: try with just \n\n
    if let Some(relative_pos) = part_bytes.windows(2).position(|w| w == b"\n\n") {
        let pattern_start = relative_pos;
        let pattern = &part_bytes[pattern_start..pattern_start + 2];

        for i in 0..data.len().saturating_sub(1) {
            if &data[i..i + 2] == pattern {
                let context_len = std::cmp::min(20, pattern_start);
                if pattern_start >= context_len {
                    let context = &part_bytes[pattern_start - context_len..pattern_start];
                    let data_context_start = i.saturating_sub(context_len);
                    if data_context_start < data.len() && i <= data.len() {
                        let data_context = &data[data_context_start..i];
                        if data_context == context {
                            return Some(i + 2);
                        }
                    }
                }
            }
        }
    }

    None
}

fn find_body_end_in_bytes(
    data: &[u8],
    start: usize,
    boundary_start: &str,
    _boundary_end: &str,
) -> Option<usize> {
    let search_data = &data[start..];

    // Look for the next boundary
    let end_boundary_bytes = format!("--{}--", boundary_start.trim_start_matches("--"))
        .as_bytes()
        .to_vec();

    // First check for end boundary
    if let Some(pos) = search_in_bytes(search_data, &end_boundary_bytes) {
        return Some(start + pos);
    }

    // Then check for next part boundary (with leading CRLF or LF)
    let crlf_boundary = format!("\r\n{}", boundary_start);
    let lf_boundary = format!("\n{}", boundary_start);

    if let Some(pos) = search_in_bytes(search_data, crlf_boundary.as_bytes()) {
        return Some(start + pos);
    }

    if let Some(pos) = search_in_bytes(search_data, lf_boundary.as_bytes()) {
        return Some(start + pos);
    }

    // If no boundary found, use end of data
    Some(data.len())
}

fn search_in_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }

    for i in 0..=haystack.len() - needle.len() {
        if &haystack[i..i + needle.len()] == needle {
            return Some(i);
        }
    }

    None
}

fn is_image_file(filename: &str) -> bool {
    let extension = filename.split('.').last().unwrap_or("").to_lowercase();
    matches!(
        extension.as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "svg" | "webp" | "bmp" | "ico"
    )
}

fn get_mime_type(filename: &str) -> &'static str {
    let extension = filename.split('.').last().unwrap_or("").to_lowercase();
    match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "zip" => "application/zip",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        _ => "application/octet-stream",
    }
}

async fn serve_uploaded_file(path: &str, responder: Responder) -> Finished {
    // Extract filename from path like "/api/files/filename.ext"
    let stored_filename = path.strip_prefix("/api/files/").unwrap_or("");
    let file_path = format!("data/uploads/{}", stored_filename);

    match std::fs::read(&file_path) {
        Ok(file_data) => {
            let mut response = Response::builder().status(StatusCode::OK);

            // Set content type based on file extension
            if let Some(content_type) = get_content_type(stored_filename) {
                response = response.header("Content-Type", content_type);
            }

            // For non-image files, add download header
            if !is_image_file(stored_filename) {
                response = response.header("Content-Disposition", "attachment");
            }

            let response = response.body(file_data.into_body()).unwrap();
            responder.respond(response).await
        }
        Err(_) => {
            let response = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("File not found".into_body())
                .unwrap();
            responder.respond(response).await
        }
    }
}
