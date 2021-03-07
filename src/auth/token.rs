use super::scope::ScopeEntry;
use super::AuthReason;
use super::{Authentication, Authenticator, FrontendAuthType};
use crate::config::TokenAuthConfig;
use hyper::header::{HeaderValue, AUTHORIZATION};
use hyper::Request;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::fs;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct Claims {
    sub: Uuid,
    exp: usize,
    scopes: Vec<ScopeEntry>,
}

pub struct TokenAuthenticator {
    config: TokenAuthConfig,
}

impl TokenAuthenticator {
    pub fn new(config: &TokenAuthConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    fn get_authorization_header<B>(req: &Request<B>) -> Result<&HeaderValue, AuthReason> {
        match req.headers().get(AUTHORIZATION) {
            Some(header) => Ok(header),
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

    fn get_jwt_validation(&self) -> Validation {
        let mut validation = Validation::new(self.config.algorithm);
        validation.iss = Some(String::from(&self.config.issuer));
        validation
    }
}

impl Authenticator for TokenAuthenticator {
    fn authenticate<B>(&self, req: &Request<B>) -> Result<Authentication, AuthReason> {
        let header = TokenAuthenticator::get_authorization_header(req)?;
        let token = TokenAuthenticator::extract_token_from_header(header)?;

        let (validation, key) = match self.config.algorithm {
            Algorithm::ES256 | Algorithm::ES384 => (
                self.get_jwt_validation(),
                load_ec_decoding_key(&self.config.keyfile),
            ),
            _ => {
                return Err(AuthReason::NotImplemented(
                    "Unable to use non ES key, not implemented",
                ))
            }
        };

        let token_data = match decode::<Claims>(&token, &key, &validation) {
            Ok(c) => c,
            Err(err) => return Err(AuthReason::InvalidCredentials(err)),
        };

        Ok(Authentication {
            id: Some(token_data.claims.sub),
            auth_type: FrontendAuthType::Token,
            scopes: token_data.claims.scopes,
        })
    }
}

fn load_ec_decoding_key(filename: &str) -> DecodingKey<'static> {
    let secret = fs::read(filename).unwrap_or_else(|_| panic!("Unable to read file public key"));
    DecodingKey::from_ec_pem(&secret)
        .map(DecodingKey::into_static)
        .unwrap()
}
