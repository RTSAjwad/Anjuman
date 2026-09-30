//! Performs `crux_http` requests for the Leptos (WASM) shell via `gloo-net`.
//!
//! The core hands us a bare `crux_http::protocol::HttpRequest`; we run it and
//! hand back a `crux_http::Result<crux_http::protocol::HttpResponse>`, which the
//! shell resolver feeds back into the core.

use shared::crux_http::{
    protocol::{HttpRequest, HttpResponse},
    HttpError,
};

#[allow(clippy::future_not_send)] // WASM is single-threaded
pub async fn request(
    HttpRequest {
        method,
        url,
        headers,
        ..
    }: &HttpRequest,
) -> shared::crux_http::Result<HttpResponse> {
    let mut request = match method.as_str() {
        "GET" => gloo_net::http::Request::get(url),
        "POST" => gloo_net::http::Request::post(url),
        other => panic!("not yet handling method {other}"),
    };

    for header in headers {
        request = request.header(&header.name, &header.value);
    }

    let response = request
        .send()
        .await
        .map_err(|error| HttpError::Io(error.to_string()))?;
    let body = response
        .binary()
        .await
        .map_err(|error| HttpError::Io(error.to_string()))?;

    Ok(HttpResponse::status(response.status()).body(body).build())
}
