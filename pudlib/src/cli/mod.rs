// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! command line interface for pudx binaries

use clap::{ArgAction::Count, Parser};
use getset::Getters;

const CONFIG_FILE_PATH: &str = "config_file_path";

/// command line interface for pudx binaries
#[derive(Parser, Debug, Getters)]
#[command(author, version, about, long_about = None)]
#[getset(get = "pub")]
pub struct Cli {
    /// Set logging verbosity.  More v's, more verbose.
    #[clap(
        short,
        long,
        action = Count,
        help = "Turn up logging verbosity (multiple will turn it up more)",
        conflicts_with = "quiet"
    )]
    verbose: u8,
    /// Set logging quietness.  More q's, more quiet.
    #[clap(
        short,
        long,
        action = Count,
        help = "Turn down logging verbosity (multiple will turn it down more)",
        conflicts_with = "verbose"
    )]
    quiet: u8,
    /// Is this a configuration dry run?
    #[clap(
        long,
        help = "Just test configuration, don't actually run server",
        default_value_t = false
    )]
    dry_run: bool,
    /// Specify the configuration file path explicitly.  Otherwise, defaults are used.
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
        let args = Cli::try_parse_from(&[env!("CARGO_PKG_NAME"), "-qqq"])?;
        assert_eq!(*args.quiet(), 3);
        assert_eq!(*args.verbose(), 0);
        assert!(!*args.dry_run());
        assert!(args.config_file_path().is_none());
        Ok(())
    }

    #[test]
    fn verbose_works() -> Result<()> {
        let args = Cli::try_parse_from(&[env!("CARGO_PKG_NAME"), "-vvv"])?;
        assert_eq!(*args.quiet(), 0);
        assert_eq!(*args.verbose(), 3);
        assert!(!*args.dry_run());
        assert!(args.config_file_path().is_none());
        Ok(())
    }

    #[test]
    fn dry_run_works() -> Result<()> {
        let args = Cli::try_parse_from(&[env!("CARGO_PKG_NAME"), "-vvv", "--dry-run"])?;
        assert_eq!(*args.quiet(), 0);
        assert_eq!(*args.verbose(), 3);
        assert!(*args.dry_run());
        assert!(args.config_file_path().is_none());
        Ok(())
    }

    #[test]
    fn config_file_path_works() -> Result<()> {
        let args = Cli::try_parse_from(&[env!("CARGO_PKG_NAME"), "-c", "a/path/to.toml"])?;
        assert_eq!(*args.quiet(), 0);
        assert_eq!(*args.verbose(), 0);
        assert!(!*args.dry_run());
        assert!(args.config_file_path().is_some());
        assert_eq!(
            args.config_file_path()
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
