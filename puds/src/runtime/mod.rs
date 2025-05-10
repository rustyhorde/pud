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
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use pudlib::{header, initialize, load, Cli, PudxBinary};
use ruarango::ConnectionBuilder;
use rustls::{
    crypto::aws_lc_rs::default_provider,
    pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
    ServerConfig,
};
use std::{
    ffi::OsString,
    fs::File,
    io::{self, BufReader, Write},
};
use tracing::{debug, error, info};

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
        args.config_file_path().as_ref(),
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

        match default_provider().install_default() {
            Ok(()) => info!("aws lc provider initialized"),
            Err(e) => error!("unable to initialize aws lc provider: {e:?}"),
        }

        // Load the TLS Keys
        let server_config = load_tls_config(&config)?;

        // Startup the server
        debug!("Starting puds on {socket_addr:?}");
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
        .bind_rustls_0_23(socket_addr, server_config)?
        .run()
        .await?;
    }
    Ok(())
}

fn load_tls_config(config: &Config) -> Result<ServerConfig> {
    let cert_file_path = config.cert_file_path();
    let key_file_path = config.key_file_path();
    debug!("cert file path: {cert_file_path}");
    debug!("key file path: {key_file_path}");

    let cert_file = &mut BufReader::new(
        File::open(cert_file_path).with_context(|| "Unable to read cert file")?,
    );

    let cert_chain: Vec<CertificateDer<'_>> = CertificateDer::pem_reader_iter(cert_file)
        .flatten()
        .collect();

    let key_file =
        &mut BufReader::new(File::open(key_file_path).with_context(|| "Unable to read key file")?);
    debug!("key file: {key_file:?}");

    let mut private_keys: Vec<PrivateKeyDer<'_>> = PrivateKeyDer::pem_reader_iter(key_file)
        .inspect(|v| match v {
            Ok(_) => debug!("valid key file: {key_file_path}"),
            Err(e) => error!("invalid key file: {e}"),
        })
        .filter_map(Result::ok)
        .collect();
    debug!("private keys: {private_keys:?}");

    if private_keys.is_empty() {
        return Err(anyhow!("No valid private keys found"));
    }
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, private_keys.remove(0))?;

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
