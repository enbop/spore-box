use rust_embed::Embed;
use wstd::http::body::{BodyForthcoming, IncomingBody, OutgoingBody};
use wstd::http::server::{Finished, Responder};
use wstd::http::{IntoBody, Request, Response, StatusCode};
use wstd::io::{AsyncWrite, copy, empty};
use wstd::time::{Duration, Instant};

#[derive(Embed)]
#[folder = "dist/"]
struct Assets;

#[wstd::http_server]
async fn main(request: Request<IncomingBody>, responder: Responder) -> Finished {
    let path = request.uri().path_and_query().unwrap().as_str();
    match path {
        "/wait" => http_wait(request, responder).await,
        "/echo" => http_echo(request, responder).await,
        "/echo-headers" => http_echo_headers(request, responder).await,
        "/echo-trailers" => http_echo_trailers(request, responder).await,
        "/fail" => http_fail(request, responder).await,
        "/bigfail" => http_bigfail(request, responder).await,
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

async fn http_home(_request: Request<IncomingBody>, responder: Responder) -> Finished {
    // Serve index.html for the home page
    if let Some((file, _)) = serve_static_file("/") {
        serve_asset(file, "index.html", responder).await
    } else {
        http_not_found(_request, responder).await
    }
}

async fn http_wait(_request: Request<IncomingBody>, responder: Responder) -> Finished {
    // Get the time now
    let now = Instant::now();

    // Sleep for one second.
    wstd::task::sleep(Duration::from_secs(1)).await;

    // Compute how long we slept for.
    let elapsed = Instant::now().duration_since(now).as_millis();

    // To stream data to the response body, use `Responder::start_response`.
    let mut body = responder.start_response(Response::new(BodyForthcoming));
    let result = body
        .write_all(format!("slept for {elapsed} millis\n").as_bytes())
        .await;
    Finished::finish(body, result, None)
}

async fn http_echo(mut request: Request<IncomingBody>, responder: Responder) -> Finished {
    // Stream data from the request body to the response body.
    let mut body = responder.start_response(Response::new(BodyForthcoming));
    let result = copy(request.body_mut(), &mut body).await;
    Finished::finish(body, result, None)
}

async fn http_fail(_request: Request<IncomingBody>, responder: Responder) -> Finished {
    let body = responder.start_response(Response::new(BodyForthcoming));
    Finished::fail(body)
}

async fn http_bigfail(_request: Request<IncomingBody>, responder: Responder) -> Finished {
    async fn write_body(body: &mut OutgoingBody) -> wstd::io::Result<()> {
        for _ in 0..0x10 {
            body.write_all("big big big big\n".as_bytes()).await?;
        }
        body.flush().await?;
        Ok(())
    }

    let mut body = responder.start_response(Response::new(BodyForthcoming));
    let _ = write_body(&mut body).await;
    Finished::fail(body)
}

async fn http_echo_headers(request: Request<IncomingBody>, responder: Responder) -> Finished {
    let mut response = Response::builder();
    *response.headers_mut().unwrap() = request.into_parts().0.headers;
    let response = response.body(empty()).unwrap();
    responder.respond(response).await
}

async fn http_echo_trailers(request: Request<IncomingBody>, responder: Responder) -> Finished {
    let body = responder.start_response(Response::new(BodyForthcoming));
    let (trailers, result) = match request.into_body().finish().await {
        Ok(trailers) => (trailers, Ok(())),
        Err(err) => (Default::default(), Err(std::io::Error::other(err))),
    };
    Finished::finish(body, result, trailers)
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
