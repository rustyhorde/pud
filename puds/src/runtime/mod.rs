// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Runtime

use crate::{
    endpoints::insecure::insecure_config,
    error::Error::{Certs, PrivKey},
    model::config::{Config, TomlConfig},
    server::Server,
};
use actix::Actor;
use actix_web::{
    middleware::Compress,
    web::{scope, Data},
    App, HttpServer,
};
use anyhow::{Context, Result};
use clap::Parser;
use pudlib::{header, initialize, load, Cli, PudxBinary};
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::{
    collections::HashMap,
    ffi::OsString,
    fs::File,
    io::{BufReader, Write},
    sync::{atomic::AtomicUsize, Arc},
};
use tracing::info;
use tracing_actix_web::TracingLogger;

#[allow(clippy::unused_async)]
pub(crate) async fn run<I, T>(args: Option<I>) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    // Parse the command line
    let args = if let Some(args) = args {
        Cli::try_parse_from(args)?
    } else {
        Cli::try_parse()?
    };

    // Load the configuration
    let mut config = load::<TomlConfig, Config>(&args, PudxBinary::Puds)?;

    // Setup logging
    initialize(&mut config)?;

    // Output the pretty header
    header::<Config, dyn Write>(&config, None)?;

    // Setup and start the server actor
    let worker_count = Arc::new(AtomicUsize::new(0));
    let manager_count = Arc::new(AtomicUsize::new(0));
    let socket_addr = *config.socket_addr();
    let workers = usize::from(*config.workers());
    let server = Server::builder()
        .workers(HashMap::new())
        .managers(HashMap::new())
        .worker_count(worker_count)
        .manager_count(manager_count)
        .build();
    let server_data = Data::new(server.start());

    if !args.dry_run() {
        // Load the TLS Keys
        let server_config = load_tls_config(&config)?;

        // Startup the server
        info!("puds configured!");
        info!("puds starting!");

        HttpServer::new(move || {
            App::new()
                .app_data(server_data.clone())
                .wrap(Compress::default())
                .wrap(TracingLogger::default())
                // .wrap(Timing)
                .service(scope("/v1").configure(insecure_config))
        })
        .workers(workers)
        .bind_rustls(socket_addr, server_config)?
        .run()
        .await?;
    }

    Ok(())
}

fn load_tls_config(config: &Config) -> Result<ServerConfig> {
    let cert_file_path = config.cert_file_path();
    let key_file_path = config.key_file_path();
    let cert_file = &mut BufReader::new(
        File::open(cert_file_path).with_context(|| "Unable to read cert file")?,
    );
    let cert_chain: Vec<Certificate> = certs(cert_file)
        .map_err(|_| Certs)?
        .into_iter()
        .map(Certificate)
        .collect();

    let key_file =
        &mut BufReader::new(File::open(key_file_path).with_context(|| "Unable to read key file")?);
    let mut keys: Vec<PrivateKey> = pkcs8_private_keys(key_file)
        .map_err(|_| PrivKey)?
        .into_iter()
        .map(PrivateKey)
        .collect();

    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain, keys.remove(0))?;

    Ok(config)
}

#[cfg(test)]
mod test {
    use super::run;
    use crate::constants::TEST_PATH;

    #[actix_rt::test]
    async fn success() {
        assert!(run(Some(&[
            env!("CARGO_PKG_NAME"),
            "--dry-run",
            "-c",
            TEST_PATH
        ]))
        .await
        .is_ok())
    }

    #[actix_rt::test]
    async fn success_with_header() {
        assert!(run(Some(&[
            env!("CARGO_PKG_NAME"),
            "--dry-run",
            "-v",
            "-c",
            TEST_PATH
        ]))
        .await
        .is_ok())
    }

    #[actix_rt::test]
    async fn error() {
        assert!(run::<Vec<&str>, &str>(None).await.is_err());
    }
}
