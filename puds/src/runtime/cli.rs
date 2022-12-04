// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! command line interface

use clap::{ArgAction::Count, Parser};
use getset::Getters;

const CONFIG_FILE_PATH: &str = "config_file_path";

#[derive(Parser, Debug, Getters)]
#[command(author, version, about, long_about = None)]
#[getset(get = "pub(crate)")]
pub(crate) struct Cli {
    #[clap(
        short,
        long,
        action = Count,
        help = "Turn up logging verbosity (multiple will turn it up more)",
        conflicts_with = "quiet"
    )]
    verbose: u8,
    #[clap(
        short,
        long,
        action = Count,
        help = "Turn down logging verbosity (multiple will turn it down more)",
        conflicts_with = "verbose"
    )]
    quiet: u8,
    #[arg(
        short = 'c',
        long,
        value_name = CONFIG_FILE_PATH,
        help = "Set the path to a valid config file"
    )]
    config_file_path: Option<String>,
}

#[cfg(test)]
mod test {
    use super::Cli;
    use anyhow::{anyhow, Result};
    use clap::{error::ErrorKind, CommandFactory, Parser};

    #[test]
    fn verify_app() {
        Cli::command().debug_assert();
    }

    #[test]
    fn quiet_works() -> Result<()> {
        let matches = Cli::try_parse_from(&[env!("CARGO_PKG_NAME"), "-qqq"])?;
        assert_eq!(*matches.quiet(), 3);
        assert_eq!(*matches.verbose(), 0);
        assert!(matches.config_file_path().is_none());
        Ok(())
    }

    #[test]
    fn verbose_works() -> Result<()> {
        let matches = Cli::try_parse_from(&[env!("CARGO_PKG_NAME"), "-vvv"])?;
        assert_eq!(*matches.quiet(), 0);
        assert_eq!(*matches.verbose(), 3);
        assert!(matches.config_file_path().is_none());
        Ok(())
    }

    #[test]
    fn config_file_path_works() -> Result<()> {
        let matches = Cli::try_parse_from(&[env!("CARGO_PKG_NAME"), "-c", "a/path/to.toml"])?;
        assert_eq!(*matches.quiet(), 0);
        assert_eq!(*matches.verbose(), 0);
        assert!(matches.config_file_path().is_some());
        assert_eq!(
            matches
                .config_file_path()
                .as_deref()
                .unwrap_or_else(|| "error"),
            "a/path/to.toml"
        );
        Ok(())
    }

    #[test]
    fn quiet_and_verbose_dont_coexist() -> Result<()> {
        match Cli::try_parse_from(&[env!("CARGO_PKG_NAME"), "-q", "-v"]) {
            Ok(_) => Err(anyhow!("This command line should fail!")),
            Err(e) => {
                assert_eq!(e.kind(), ErrorKind::ArgumentConflict);
                Ok(())
            }
        }
    }
}
