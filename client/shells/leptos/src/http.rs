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
        body,
    }: &HttpRequest,
) -> shared::crux_http::Result<HttpResponse> {
    let builder = match method.as_str() {
        "GET" => gloo_net::http::Request::get(url),
        "POST" => gloo_net::http::Request::post(url),
        other => panic!("not yet handling method {other}"),
    };

    let builder = headers.iter().fold(builder, |request, header| {
        request.header(&header.name, &header.value)
    });

    // Forward the request body. The core hands us raw bytes (already serialized
    // JSON for `POST /auth/login`, etc.); for bodyless requests (GET) it's an
    // empty `Vec<u8>`, which we skip so we don't send `GET` with a body.
    //
    // `gloo-net`'s `.body()` consumes the `RequestBuilder` and returns a
    // `Request` (both have `.send()`), so branch rather than trying to reuse one
    // type for both paths.
    let response = if body.is_empty() {
        builder.send().await
    } else {
        builder
            .body(body.clone())
            .map_err(|error| HttpError::Io(error.to_string()))?
            .send()
            .await
    }
    .map_err(|error| HttpError::Io(error.to_string()))?;

    let response_body = response
        .binary()
        .await
        .map_err(|error| HttpError::Io(error.to_string()))?;

    Ok(HttpResponse::status(response.status())
        .body(response_body)
        .build())
}
