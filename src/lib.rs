pub mod auth;
pub mod config;
pub mod proxy;
pub mod scope;
pub mod tls;

use hyper::{Body, Request, Response, StatusCode};
use std::net::IpAddr;

use crate::auth::{request_is_authorized, AuthReason};
use crate::proxy::{create_proxied_request, create_proxied_response, request_add_custom_headers};

pub const SERVER_VIA: &str = concat!(env!("CARGO_PKG_VERSION"), " Demogorgon");

pub async fn service_handler(
    req: Request<Body>,
    remote_addr: IpAddr,
    config: config::Config,
) -> Result<Response<Body>, hyper::Error> {
    log::debug!("Request to {}", req.uri());

    let first = req.uri().path().split('/').nth(1).unwrap();
    log::debug!("Creating HTTPS client with Cert Auth");

    match config.backends.get(first) {
        Some(backend) => rev_proxy(req, remote_addr, &backend, &config).await,
        None => Ok(error_response(StatusCode::NOT_FOUND)),
    }
}

async fn rev_proxy(
    req: Request<Body>,
    remote_addr: IpAddr,
    backend: &config::Backend,
    config: &config::Config,
) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path().to_string();

    let response = match request_is_authorized(&req, &backend, &config) {
        Ok(scope) => {
            let client = backend.get_client();
            let req = create_proxied_request(remote_addr, &backend, req, &scope)?;
            let req = request_add_custom_headers(&backend, req);

            log::info!("A {} {{{}}} {} {}", remote_addr, scope, req.method(), path);

            match client.request(req).await {
                Ok(r) => r,
                Err(_) => error_response(StatusCode::GATEWAY_TIMEOUT),
            }
        }
        Err(ar) => match ar {
            AuthReason::BadRequest(reason) | AuthReason::NotImplemented(reason) => {
                log::warn!("D {} {} {}", remote_addr, req.method(), path);
                log::warn!("Bad Request: {}", reason);
                error_response(StatusCode::BAD_REQUEST)
            }
            AuthReason::InvalidCredentials(jwt_error) => {
                log::warn!("D {} {} {}", remote_addr, req.method(), path);
                log::warn!("Invalid token: {}", jwt_error);
                error_response(StatusCode::UNAUTHORIZED)
            }
            AuthReason::InsufficientScope(reason) => {
                log::warn!("D {} {} {}", remote_addr, req.method(), path);
                log::warn!("Insufficient scope: {}", reason);
                error_response(StatusCode::FORBIDDEN)
            }
        },
    };
    let response = process_location_header(response);
    let response = create_proxied_response(response);
    Ok(response)
}

fn process_location_header(response: Response<Body>) -> Response<Body> {
    match response.status() {
        StatusCode::MOVED_PERMANENTLY
        | StatusCode::FOUND
        | StatusCode::SEE_OTHER
        | StatusCode::TEMPORARY_REDIRECT
        | StatusCode::PERMANENT_REDIRECT => {
            log::warn!("Received a redirect response from backend. Blocking.");
            error_response(StatusCode::BAD_GATEWAY)
        }
        _ => response,
    }
}

fn error_response(status: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(status.canonical_reason().unwrap().into())
        .unwrap()
}
