// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Runtime

pub(crate) mod cli;
pub(crate) mod config;
mod header;
pub(crate) mod log;

use self::{cli::Cli, config::load, header::header, log::initialize};
use crate::{endpoints::insecure::insecure_config, server::Server};
use actix::Actor;
use actix_web::{
    middleware::Compress,
    web::{scope, Data},
    App, HttpServer,
};
use anyhow::Result;
use clap::Parser;
use std::{
    collections::HashMap,
    ffi::OsString,
    io::Write,
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
    let mut config = load(&args)?;

    // Setup logging
    initialize(&mut config)?;

    // Output the pretty header
    header::<dyn Write>(&config, None)?;

    // Setup and start the server actor
    let worker_count = Arc::new(AtomicUsize::new(0));
    // let manager_count = Arc::new(AtomicUsize::new(0));
    let socket_addr = *config.socket_addr();
    let workers = usize::from(*config.workers());
    let server = Server::builder()
        // .config(config)
        .workers(HashMap::new())
        .worker_count(worker_count)
        // .manager_count(manager_count)
        .build();
    let server_data = Data::new(server.start());

    // Startup the server
    info!("puds configured!");
    info!("puds starting!");

    if !args.dry_run() {
        HttpServer::new(move || {
            App::new()
                .app_data(server_data.clone())
                .wrap(Compress::default())
                .wrap(TracingLogger::default())
                // .wrap(Timing)
                .service(scope("/v1").configure(insecure_config))
        })
        .workers(workers)
        // .bind_rustls(listen_addr, server_config)?
        .bind(socket_addr)?
        .run()
        .await?;
    }

    Ok(())
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
