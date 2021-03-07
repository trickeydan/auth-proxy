#![deny(warnings)]
#[macro_use]
extern crate log;

#[macro_use]
extern crate clap;

use crate::config::Config;
use crate::service_handler;
use clap::{crate_version, App};
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
use std::process;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    pretty_env_logger::init();

    info!("🔦🚲 Demogorgon {} 🔦🚲", env!("CARGO_PKG_VERSION"));

    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).version(crate_version!()).get_matches();

    let config = matches.value_of("config").unwrap_or("demogorgon.toml");

    let config = Config::load(config).unwrap_or_else(|err| {
        error!("Config Error: {}", err);
        process::exit(1);
    });

    log::info!("Loaded {} backends", config.backends.len());
    for (name, backend) in &config.backends {
        log::info!("\t /{} -> {}", name, backend.url);
        log::info!("\t\tAuthentication: {:?}", backend.frontend_auth);
        log::info!("\t\tAuthorization Scope: {}", backend.scope);
    }

    let config2 = config.clone();
    let service = make_service_fn(move |conn: &AddrStream| {
        // first move it into the closure
        // closure can be called multiple times, so for each call, we must
        // clone it and move that clone into the async block
        let config = config2.clone();
        let remote_addr = conn.remote_addr().ip();
        async move {
            // async block is only executed once, so just pass it on to the closure
            Ok::<_, hyper::Error>(service_fn(move |_req| {
                // but this closure may also be called multiple times, so make
                // a clone for each call, and move the clone into the async block
                let config = config.clone();
                async move { service_handler(_req, remote_addr, config).await }
            }))
        }
    });

    info!("Listening on http://{}", config.address);

    let server = Server::bind(&config.address).serve(service);

    server.await?;

    Ok(())
}
