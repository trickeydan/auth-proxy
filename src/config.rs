use hyper::client::connect::HttpConnector;
use hyper::Client;
use jsonwebtoken::Algorithm;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::net::SocketAddr;

use crate::tls::ClientCertAuth;

use crate::scope::ScopeEntry;

#[derive(Clone, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Backend {
    cert_auth: Option<ClientCertAuth>,
    pub headers: Option<HashMap<String, String>>,
    pub url: String,
    pub scope: ScopeEntry,

    #[serde(default = "default_scope_header")]
    pub scope_header: String,
}

fn default_scope_header() -> String {
    "X-Demogorgon-Scope".to_string()
}

impl Backend {
    fn get_client_config() -> rustls::ClientConfig {
        let mut tls = rustls::ClientConfig::new();
        tls.root_store = match rustls_native_certs::load_native_certs() {
            Ok(store) => store,
            Err((Some(store), err)) => {
                log::warn!("Could not load all certificates: {:?}", err);
                store
            }
            Err((None, err)) => Err(err).expect("cannot access native cert store"),
        };
        tls
    }

    pub fn get_client(&self) -> Client<hyper_rustls::HttpsConnector<HttpConnector>, hyper::Body> {
        let https = match &self.cert_auth {
            Some(ca) => {
                log::debug!("Creating HTTPS client with Cert Auth");
                let mut http = hyper::client::HttpConnector::new();
                http.enforce_http(false);
                let mut tls = Backend::get_client_config();

                let (cert_chain, privkey) =
                    ca.get_client_cert().expect("Unable to load client cert.");
                tls.set_single_client_cert(cert_chain, privkey)
                    .expect("TLS Error when loading client cert.");
                hyper_rustls::HttpsConnector::from((http, tls))
            }
            None => {
                log::debug!("Creating HTTPS client");
                let mut http = hyper::client::HttpConnector::new();
                http.enforce_http(false);
                let tls = Backend::get_client_config();
                hyper_rustls::HttpsConnector::from((http, tls))
            }
        };
        Client::builder().build(https)
    }
}

#[derive(Clone, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Auth {
    pub algorithm: Algorithm,
    pub keyfile: String,
    pub issuer: String,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub address: SocketAddr,
    pub auth: Auth,
    pub backends: HashMap<String, Backend>,
}

impl Config {
    pub fn load(filename: &str) -> Result<Config, Box<dyn Error>> {
        log::debug!("Loading config file from {}", filename);
        let contents = fs::read_to_string(filename)?;
        let config: Config = toml::from_str(&contents)?;
        log::debug!("Loaded configuration: {:?}", config);
        Ok(config)
    }
}
