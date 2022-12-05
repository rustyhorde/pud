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
use anyhow::Result;
use clap::Parser;
use std::{ffi::OsString, io::Write};

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

    Ok(())
}

#[cfg(test)]
mod test {
    use super::run;
    use crate::constants::TEST_PATH;

    #[tokio::test]
    async fn success() {
        assert!(run(Some(&[env!("CARGO_PKG_NAME"), "-c", TEST_PATH]))
            .await
            .is_ok())
    }

    #[tokio::test]
    async fn success_with_header() {
        assert!(run(Some(&[env!("CARGO_PKG_NAME"), "-v", "-c", TEST_PATH]))
            .await
            .is_ok())
    }

    #[tokio::test]
    async fn error() {
        assert!(run::<Vec<&str>, &str>(None).await.is_err());
    }
}
