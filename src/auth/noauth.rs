use super::scope::ScopeEntry;
use super::{AuthReason, Authentication, Authenticator, FrontendAuthType};
use hyper::Request;
use std::convert::TryFrom;

pub struct NoAuthAuthenticator {}

impl NoAuthAuthenticator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Authenticator for NoAuthAuthenticator {
    fn authenticate<B>(&self, _: &Request<B>) -> Result<Authentication, AuthReason> {
        Ok(Authentication {
            id: None,
            auth_type: FrontendAuthType::NoAuth,
            scopes: vec![ScopeEntry::try_from("*:*").unwrap()],
        })
    }
}
