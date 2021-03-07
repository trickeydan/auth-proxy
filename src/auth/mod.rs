use crate::config::{Backend, Config, FrontendAuthType};
use crate::scope::ScopeEntry;
use hyper::Request;
use jsonwebtoken::errors::Error as JWTError;
use std::convert::TryFrom;

mod token;

pub enum AuthReason {
    BadRequest(&'static str),
    InvalidCredentials(JWTError),
    NotImplemented(&'static str),
    InsufficientScope(String),
}

pub fn request_is_authorized<B>(
    req: &Request<B>,
    backend: &Backend,
    config: &Config,
) -> Result<ScopeEntry, AuthReason> {
    match &backend.frontend_auth {
        FrontendAuthType::Token => token::check_token_auth(req, backend, config),
        FrontendAuthType::NoAuth => Ok(ScopeEntry::try_from("*:*").unwrap()),
    }
}
