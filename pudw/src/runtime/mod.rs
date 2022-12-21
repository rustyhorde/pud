// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Runtime

use crate::{
    actor::Worker,
    model::config::{Config, TomlConfig},
};
use actix::{io::SinkWrite, spawn, Actor, StreamHandler, System};
use anyhow::{Context, Result};
use awc::{http::Version, Client};
use clap::Parser;
use futures::StreamExt;
use pudlib::{header, initialize, load, Cli, PudxBinary};
use std::{ffi::OsString, io::Write, thread::sleep, time::Duration};
use tokio::sync::mpsc::unbounded_channel;
use tracing::{debug, error, info};

const HEADER_PREFIX: &str = r#"██████╗ ██╗   ██╗██████╗ ██╗    ██╗
██╔══██╗██║   ██║██╔══██╗██║    ██║
██████╔╝██║   ██║██║  ██║██║ █╗ ██║
██╔═══╝ ██║   ██║██║  ██║██║███╗██║
██║     ╚██████╔╝██████╔╝╚███╔███╔╝
╚═╝      ╚═════╝ ╚═════╝  ╚══╝╚══╝ "#;

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
        args.config_file_path(),
        *args.verbose(),
        *args.quiet(),
        PudxBinary::Pudw,
    )?;

    // Setup logging
    let guard = initialize(&mut config)?;

    // Output the pretty header
    header::<Config, dyn Write>(&config, HEADER_PREFIX, None)?;

    // Pull values out of config
    let url = config.server_url();
    let mut retry_count = *config.retry_count();
    let mut error_count = 0;

    if !args.dry_run() {
        while retry_count > 0 {
            let sys = System::new();
            let url_c = url.clone();
            let (tx_stdout, mut rx_stdout) = unbounded_channel();
            let (tx_stderr, mut rx_stderr) = unbounded_channel();
            let (tx_status, mut rx_status) = unbounded_channel();
            sys.block_on(async move {
                let awc = Client::builder()
                    .max_http_version(Version::HTTP_11)
                    .finish();

                if let Ok((response, framed)) = awc.ws(&url_c).connect().await.map_err(|e| {
                    error!("Error: {e}");
                }) {
                    debug!("{response:?}");
                    let (sink, stream) = framed.split();
                    let addr = Worker::create(|ctx| {
                        let _ = Worker::add_stream(stream, ctx);
                        Worker::builder()
                            .addr(SinkWrite::new(sink, ctx))
                            .tx_stdout(tx_stdout.clone())
                            .tx_stderr(tx_stderr.clone())
                            .tx_status(tx_status.clone())
                            .build()
                    });

                    let stdout_addr = addr.clone();
                    let _handle = spawn(async move {
                        while let Some(line) = rx_stdout.recv().await {
                            stdout_addr.do_send(line);
                        }
                    });

                    let stderr_addr = addr.clone();
                    let _handle = spawn(async move {
                        while let Some(line) = rx_stderr.recv().await {
                            stderr_addr.do_send(line);
                        }
                    });

                    let status_addr = addr;
                    let _handle = spawn(async move {
                        while let Some(status) = rx_status.recv().await {
                            status_addr.do_send(status);
                        }
                    });
                } else {
                    error!("unable to connect");
                    System::current().stop();
                }
            });

            if let Err(e) = sys.run().context("run failed") {
                error!("{e}");
            }
            info!("worker disconnected!");
            info!("Trying to reconnect...");
            retry_count -= 1;
            sleep(Duration::from_secs(2u64.pow(error_count)));
            error_count += 1;
        }
    }

    if let Some(guard) = guard {
        drop(guard);
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::run;
    use crate::constants::TEST_PATH;

    #[test]
    fn success() {
        assert!(run(Some(&[
            env!("CARGO_PKG_NAME"),
            "--dry-run",
            "-c",
            TEST_PATH
        ]))
        .is_ok())
    }

    #[test]
    fn success_with_header() {
        assert!(run(Some(&[
            env!("CARGO_PKG_NAME"),
            "--dry-run",
            "-v",
            "-c",
            TEST_PATH
        ]))
        .is_ok())
    }
}
