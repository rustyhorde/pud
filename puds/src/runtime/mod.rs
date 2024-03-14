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
use ruarango::ConnectionBuilder;
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer},
    ServerConfig,
};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::{
    ffi::OsString,
    fs::File,
    io::{self, BufReader, Write},
};
use tracing::info;

const HEADER_PREFIX: &str = r"██████╗ ██╗   ██╗██████╗ ███████╗
██╔══██╗██║   ██║██╔══██╗██╔════╝
██████╔╝██║   ██║██║  ██║███████╗
██╔═══╝ ██║   ██║██║  ██║╚════██║
██║     ╚██████╔╝██████╔╝███████║
 ╚═╝      ╚═════╝ ╚═════╝ ╚══════╝";

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
    let mut config = load::<TomlConfig, Config>(
        args.config_file_path(),
        *args.verbose(),
        *args.quiet(),
        PudxBinary::Puds,
    )?;

    // Setup logging
    initialize(&mut config)?;

    // Output the pretty header
    header::<Config, dyn Write>(&config, HEADER_PREFIX, Some(&mut io::stdout()))?;

    // Setup and start the server actor
    let socket_addr = *config.socket_addr();
    let workers = usize::from(*config.workers());
    let server = Server::builder().config(config.clone()).build();
    let server_data = Data::new(server.start());

    // Add config to app data
    let config_c = config.clone();
    let config_data = Data::new(config_c);

    if !args.dry_run() {
        // Setup connection to the database
        let conn = ConnectionBuilder::default()
            .url(config.db_url())
            .username(config.db_user())
            .password(config.db_pass())
            .database(config.db_name())
            .build()
            .await?;

        // Add connection to app data
        let conn_data = Data::new(conn);

        // Load the TLS Keys
        let server_config = load_tls_config(&config)?;

        // Startup the server
        info!("puds configured!");
        info!("puds starting!");

        HttpServer::new(move || {
            App::new()
                .app_data(server_data.clone())
                .app_data(config_data.clone())
                .app_data(conn_data.clone())
                .wrap(Compress::default())
                .service(scope("/v1").configure(insecure_config))
        })
        .workers(workers)
        .bind_rustls_0_22(socket_addr, server_config)?
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
    let cert_chain: Vec<CertificateDer<'_>> = certs(cert_file).filter_map(Result::ok).collect();

    let key_file =
        &mut BufReader::new(File::open(key_file_path).with_context(|| "Unable to read key file")?);
    let mut keys: Vec<PrivateKeyDer<'_>> = pkcs8_private_keys(key_file)
        .filter_map(Result::ok)
        .map(PrivateKeyDer::from)
        .collect();

    let config = ServerConfig::builder()
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
        .is_ok());
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
        .is_ok());
    }

    #[actix_rt::test]
    async fn error() {
        assert!(run::<Vec<&str>, &str>(None).await.is_err());
    }
}
