// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Runtime

use crate::model::config::{Config, TomlConfig};
use anyhow::Result;
use clap::Parser;
use pudlib::{header, initialize, load, Cli, PudxBinary};
use std::{ffi::OsString, io::Write};

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
    let mut config = load::<TomlConfig, Config>(&args, PudxBinary::Pudw)?;

    // Setup logging
    initialize(&mut config)?;

    // Output the pretty header
    header::<Config, dyn Write>(&config, HEADER_PREFIX, None)?;
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

    #[test]
    fn error() {
        assert!(run::<Vec<&str>, &str>(None).is_err());
    }
}
