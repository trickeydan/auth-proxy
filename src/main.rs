#![deny(warnings)]
#[macro_use]
extern crate log;

use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
use std::process;

use demogorgon::config::Config;
use demogorgon::service_handler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    pretty_env_logger::init();

    let config = Config::load("demogorgon.toml").unwrap_or_else(|err| {
        error!("Config Error: {}", err);
        process::exit(1);
    });

    // TODO: Print config in safe manner

    let service = make_service_fn(move |conn: &AddrStream| {
        // first move it into the closure
        // closure can be called multiple times, so for each call, we must
        // clone it and move that clone into the async block
        let config = config.clone();
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

    let config = Config::load("demogorgon.toml").unwrap_or_else(|err| {
        error!("Config Error: {}", err);
        process::exit(1);
    });

    info!("Starting Demogorgon {}", env!("CARGO_PKG_VERSION"));
    info!("Listening on http://{}", config.address);

    let server = Server::bind(&config.address).serve(service);

    server.await?;

    Ok(())
}
