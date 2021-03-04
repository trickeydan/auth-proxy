use hyper::header::{HeaderValue, AUTHORIZATION};
use hyper::Request;
use jsonwebtoken::errors::Error as JWTError;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::fs;
use uuid::Uuid;

use crate::config::{Backend, Config};
use crate::scope::ScopeEntry;

#[derive(Debug, Deserialize)]
struct Claims {
    sub: Uuid,
    exp: usize,
    scopes: Vec<ScopeEntry>,
}

pub enum AuthReason {
    BadRequest(&'static str),
    InvalidToken(JWTError),
    NotImplemented(&'static str),
    InsufficientScope(String),
}

pub fn request_is_authorized<B>(
    req: &Request<B>,
    backend: &Backend,
    config: &Config,
) -> Result<ScopeEntry, AuthReason> {
    match req.headers().get(AUTHORIZATION) {
        Some(header) => {
            let token = extract_token_from_header(&header)?;
            check_token_is_valid(token, &config, &backend)
        }
        None => Err(AuthReason::BadRequest("Missing authorization header")),
    }
}

fn extract_token_from_header(header: &HeaderValue) -> Result<&str, AuthReason> {
    let header = header.to_str().expect("Unable to parse header as str");
    if !header.starts_with("Bearer ") {
        return Err(AuthReason::BadRequest("Authorization must be Bearer"));
    }
    Ok(header.trim_start_matches("Bearer "))
}

fn get_jwt_validation(config: &Config) -> Validation {
    let mut validation = Validation::new(config.auth.algorithm);
    validation.iss = Some(String::from(&config.auth.issuer));
    validation
}

fn load_ec_decoding_key() -> DecodingKey<'static> {
    let secret =
        fs::read("public_key.pem").unwrap_or_else(|_| panic!("Unable to read file public key"));
    DecodingKey::from_ec_pem(&secret)
        .map(DecodingKey::into_static)
        .unwrap()
}

fn check_token_is_valid(
    token: &str,
    config: &Config,
    backend: &Backend,
) -> Result<ScopeEntry, AuthReason> {
    let (validation, key) = match config.auth.algorithm {
        Algorithm::ES256 | Algorithm::ES384 => {
            (get_jwt_validation(&config), load_ec_decoding_key())
        }
        _ => {
            return Err(AuthReason::NotImplemented(
                "Unable to use non ES key, not implemented",
            ))
        }
    };

    let token_data = match decode::<Claims>(&token, &key, &validation) {
        Ok(c) => c,
        Err(err) => return Err(AuthReason::InvalidToken(err)),
    };

    for token_scope in &token_data.claims.scopes {
        if token_scope > &backend.scope {
            return Ok(token_scope.clone());
        }
    }

    Err(AuthReason::InsufficientScope(format!(
        "{:?} is insufficient scope to reach {}",
        token_data.claims.scopes, backend.scope
    )))
}
