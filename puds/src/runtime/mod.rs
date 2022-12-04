// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Runtime

mod cli;
mod config;

use self::{cli::Cli, config::load};
use anyhow::Result;
use clap::Parser;
use std::ffi::OsString;

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
    load(&args)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::constants::TEST_PATH;

    use super::run;

    #[tokio::test]
    async fn success() {
        assert!(run(Some(&[env!("CARGO_PKG_NAME"), "-c", TEST_PATH]))
            .await
            .is_ok())
    }
}
