// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Runtime

use crate::{
    actor::CommandLine,
    model::{
        cli::{Cli, Subcommands},
        config::{Config, TomlConfig},
    },
};
use actix::{io::SinkWrite, spawn, Actor, StreamHandler};
use actix_rt::System;
use anyhow::{Context, Result};
use awc::{http::Version, Client};
use clap::Parser;
use futures::StreamExt;
use pudlib::{initialize, load, ManagerClientToManagerSession, PudxBinary};
#[cfg(unix)]
use rustls::crypto::aws_lc_rs;
use std::ffi::OsString;
use tokio::sync::mpsc::unbounded_channel;
#[cfg(unix)]
use tracing::info;
use tracing::{debug, error};

#[allow(tail_expr_drop_order)]
pub(crate) fn run<I, T>(args: Option<I>) -> Result<()>
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
        PudxBinary::Pudcli,
    )?;

    // Setup logging
    initialize(&mut config)?;

    install_provider();

    // Pull values out of config
    let url = config.server_url();

    let command_to_run = match args.sub_cmd() {
        Subcommands::Reload => ManagerClientToManagerSession::Reload,
        Subcommands::ListWorkers => ManagerClientToManagerSession::ListWorkers,
        Subcommands::Schedules(schedule) => {
            ManagerClientToManagerSession::Schedules(schedule.name().clone())
        }
        Subcommands::Query(query) => ManagerClientToManagerSession::Query(query.query().clone()),
    };

    if !args.dry_run() {
        let (tx, mut rx) = unbounded_channel();
        let sys = System::new();

        sys.block_on(async move {
            let client = Client::builder()
                .max_http_version(Version::HTTP_11)
                .finish();
            match client.ws(&url).connect().await.map_err(|e| {
                error!("Error: {e}");
            }) {
                Ok((response, framed)) => {
                    debug!("{response:?}");
                    let (sink, stream) = framed.split();
                    let addr = CommandLine::create(|ctx| {
                        _ = CommandLine::add_stream(stream, ctx);
                        CommandLine::builder()
                            .addr(SinkWrite::new(sink, ctx))
                            .tx(tx.clone())
                            .command_to_run(command_to_run)
                            .build()
                    });

                    let _handle = spawn(async move {
                        while let Some(status) = rx.recv().await {
                            addr.do_send(status);
                        }
                    });
                }
                _ => {
                    System::current().stop();
                }
            }
        });

        if let Err(e) = sys.run().context("run failed") {
            error!("{e:?}");
            error!("should kill sys");
        }
    }
    Ok(())
}

#[cfg(unix)]
fn install_provider() {
    match aws_lc_rs::default_provider().install_default() {
        Ok(()) => info!("aws lc provider initialized"),
        Err(e) => error!("unable to initialize aws lc provider: {e:?}"),
    }
}

#[cfg(windows)]
fn install_provider() {}
