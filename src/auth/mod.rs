use crate::config::{Backend, Config};
use crate::scope::ScopeEntry;
use hyper::Request;
use jsonwebtoken::errors::Error as JWTError;
use serde::Deserialize;
use uuid::Uuid;

mod noauth;
mod token;

pub enum AuthReason {
    BadRequest(&'static str),
    InvalidCredentials(JWTError),
    NotImplemented(&'static str),
    InsufficientScope(String),
}

#[derive(Clone, Deserialize, Debug)]
pub enum FrontendAuthType {
    NoAuth,
    Token,
}

impl FrontendAuthType {
    pub fn default() -> Self {
        FrontendAuthType::Token
    }
}

#[derive(Debug)]
pub struct Authentication {
    id: Option<Uuid>,
    auth_type: FrontendAuthType,
    scopes: Vec<ScopeEntry>,
}

impl Authentication {
    pub fn authorize(&self, backend: &Backend) -> Result<ScopeEntry, AuthReason> {
        for token_scope in &self.scopes {
            if token_scope > &backend.scope {
                return Ok(token_scope.clone());
            }
        }

        Err(AuthReason::InsufficientScope(format!(
            "{:?} is insufficient scope to reach {}",
            self.scopes, backend.scope
        )))
    }
}

pub trait Authenticator {
    fn authenticate<B>(&self, req: &Request<B>) -> Result<Authentication, AuthReason>;
}

pub fn request_is_authorized<B>(
    req: &Request<B>,
    backend: &Backend,
    config: &Config,
) -> Result<ScopeEntry, AuthReason> {
    let authentication = match &backend.frontend_auth {
        FrontendAuthType::Token => {
            let authenticator = token::TokenAuthenticator::new(&config.auth);
            authenticator.authenticate(req)?
        }
        FrontendAuthType::NoAuth => {
            let authenticator = noauth::NoAuthAuthenticator::new();
            authenticator.authenticate(req)?
        }
    };
    authentication.authorize(&backend)
}
