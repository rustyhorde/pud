// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! configuration for pudx binaries

use crate::{
    constants::{
        CONFIG_FILE_BASE_PATH_PUDS, CONFIG_FILE_BASE_PATH_PUDW, CONFIG_FILE_NAME_PUDS,
        CONFIG_FILE_NAME_PUDW, FILE_OPEN, READ, UNABLE,
    },
    error::Error::ConfigDir,
    Cli,
};
use anyhow::{Context, Result};
use getset::CopyGetters;
use serde::de::DeserializeOwned;
use std::{fs::File, io::Read, path::PathBuf};

/// Can store verbosity information
pub trait Verbosity {
    /// Set the level of quiet.
    fn set_quiet(&mut self, quiet: u8) -> &mut Self;
    /// Set the level of verbose.
    fn set_verbose(&mut self, verbose: u8) -> &mut Self;
}

/// The binary we are configuring
#[derive(Clone, Copy, Debug)]
pub enum PudxBinary {
    /// The pud server binary
    Puds,
    /// The pud worker binary
    Pudw,
    #[cfg(test)]
    /// A test binary
    Test,
}

/// The defaults for a given pudx binary
#[derive(Clone, Copy, CopyGetters, Debug)]
#[getset(get_copy = "pub(crate)")]
pub(crate) struct Defaults {
    /// The default base path to the config
    default_base_path: &'static str,
    /// The default config file name
    default_file_name: &'static str,
}

impl Defaults {
    pub(crate) fn puds_defaults() -> Self {
        Defaults {
            default_base_path: CONFIG_FILE_BASE_PATH_PUDS,
            default_file_name: CONFIG_FILE_NAME_PUDS,
        }
    }

    pub(crate) fn pudw_defaults() -> Self {
        Defaults {
            default_base_path: CONFIG_FILE_BASE_PATH_PUDW,
            default_file_name: CONFIG_FILE_NAME_PUDW,
        }
    }

    #[cfg(test)]
    pub(crate) fn test_defaults() -> Self {
        use crate::constants::{CONFIG_FILE_BASE_PATH_TEST, CONFIG_FILE_NAME_TEST};

        Defaults {
            default_base_path: CONFIG_FILE_BASE_PATH_TEST,
            default_file_name: CONFIG_FILE_NAME_TEST,
        }
    }
}

/// Load configuration given command line arguments
///
/// # Errors
/// * I/O error if the default config path cannot be determined (via `dirs2`)
/// * I/O error if the file cannot be read
/// * TOML parse errors
/// * `std::from::TryFrom` error if the TOML cannot be converted to the final config.
///
pub fn load<T, U>(args: &Cli, binary: PudxBinary) -> Result<U>
where
    T: DeserializeOwned,
    U: TryFrom<T> + Verbosity,
    <U as TryFrom<T>>::Error: std::error::Error + 'static,
    <U as TryFrom<T>>::Error: Sync,
    <U as TryFrom<T>>::Error: Send,
{
    // Load the defaults for this binary
    let defaults = match binary {
        PudxBinary::Puds => Defaults::puds_defaults(),
        PudxBinary::Pudw => Defaults::pudw_defaults(),
        #[cfg(test)]
        PudxBinary::Test => Defaults::test_defaults(),
    };
    // Determine the configuration file path
    let config_file_path = config_file_path(args, defaults)?;
    // Setup error handling
    let path = config_file_path.clone();
    let ctx = |msg: &'static str| -> String { format!("{msg} {}", path.display()) };
    // Read the config file
    let config_file = read_config_file(config_file_path, ctx)?;
    // Parse the config file
    let config: T = toml::from_str(&config_file).with_context(|| ctx(UNABLE))?;
    // Convert the toml config to base config
    transform(config, *args.verbose(), *args.quiet())
}

fn config_file_path(args: &Cli, defaults: Defaults) -> Result<PathBuf> {
    let default_fn = || -> Result<PathBuf> { default_config_file_path(defaults) };
    args.config_file_path()
        .as_ref()
        .map_or_else(default_fn, to_path_buf)
}

fn default_config_file_path(defaults: Defaults) -> Result<PathBuf> {
    let mut config_file_path = dirs2::config_dir().ok_or(ConfigDir)?;
    config_file_path.push(defaults.default_base_path());
    config_file_path.push(defaults.default_file_name());
    Ok(config_file_path)
}

#[allow(clippy::unnecessary_wraps)]
fn to_path_buf(path: &String) -> Result<PathBuf> {
    Ok(PathBuf::from(path))
}

fn read_config_file<F>(config_file_path: PathBuf, ctx: F) -> Result<String>
where
    F: FnOnce(&'static str) -> String + Copy,
{
    let mut buf = String::new();
    let mut file = File::open(config_file_path).with_context(|| ctx(FILE_OPEN))?;
    let _ = file.read_to_string(&mut buf).with_context(|| ctx(READ))?;
    Ok(buf)
}

fn transform<T, U>(config: T, verbose: u8, quiet: u8) -> Result<U>
where
    U: TryFrom<T> + Verbosity,
    <U as TryFrom<T>>::Error: std::error::Error + 'static,
    <U as TryFrom<T>>::Error: Sync,
    <U as TryFrom<T>>::Error: Send,
{
    let mut config: U = U::try_from(config)?;
    let _ = config.set_verbose(verbose);
    let _ = config.set_quiet(quiet);
    Ok(config)
}

#[cfg(test)]
mod test {
    use super::{
        config_file_path, default_config_file_path, load, read_config_file, Defaults, PudxBinary,
        Verbosity,
    };
    use crate::{constants::TEST_PATH, error::Error, Cli};
    use anyhow::{anyhow, Result};
    use clap::Parser;
    use getset::Getters;
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;

    /// The TOML configuration.
    #[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
    #[getset(get)]
    pub(crate) struct TomlConfig {
        /// The actix server configuration
        actix: Actix,
    }

    /// hosts configuration
    #[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
    #[getset(get)]
    pub(crate) struct Actix {
        /// The number of workers to start
        workers: u8,
    }

    /// The configuration
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) struct Config {
        quiet: u8,
        verbose: u8,
        workers: u8,
    }

    impl Verbosity for Config {
        fn set_quiet(&mut self, quiet: u8) -> &mut Self {
            self.quiet = quiet;
            self
        }

        fn set_verbose(&mut self, verbose: u8) -> &mut Self {
            self.verbose = verbose;
            self
        }
    }

    impl TryFrom<TomlConfig> for Config {
        type Error = Error;

        fn try_from(config: TomlConfig) -> Result<Self, Self::Error> {
            let workers = *config.actix().workers();
            Ok(Config {
                verbose: 0,
                quiet: 0,
                workers,
            })
        }
    }

    const BAD_PATH: &'static str = "this/path/is/bad/config.toml";
    const BAD_TOML_TEST_PATH: &str = "test/bad.toml";
    const BAD_WORKERS_TOML_TEST_PATH: &str = "test/bad_workers.toml";
    #[cfg(windows)]
    const BAD_CONFIG_ERROR: &str = "Could not open config file! this/path/is/bad/config.toml\n\nCaused by:\n    The system cannot find the path specified. (os error 3)";
    #[cfg(not(windows))]
    const BAD_CONFIG_ERROR: &str = "Could not open config file! this/path/is/bad/config.toml\n\nCaused by:\n    No such file or directory (os error 2)";
    const BAD_PARSE_ERROR: &str =
        "Could not parse config file! test/bad.toml\n\nCaused by:\n    missing field `actix`";
    const BAD_WORKERS_PARSE_ERROR: &str =
        "Could not parse config file! test/bad_workers.toml\n\nCaused by:\n    invalid type: string \"57\", expected u8 for key `actix.workers` at line 2 column 11";
    const TEST_CONFIG: &'static str = r#"[actix]
workers = 8
"#;

    #[test]
    fn config_file_path_is_default() -> Result<()> {
        let args = Cli::try_parse_from(&[env!("CARGO_PKG_NAME")])?;
        let defaults = Defaults::test_defaults();
        let path = config_file_path(&args, defaults)?;
        assert_eq!(path, default_config_file_path(defaults)?);
        Ok(())
    }

    #[test]
    fn config_file_path_is_set() -> Result<()> {
        let args = Cli::try_parse_from(&[env!("CARGO_PKG_NAME"), "-c", TEST_PATH])?;
        let defaults = Defaults::test_defaults();
        let path = config_file_path(&args, defaults)?;
        let expected = PathBuf::from(TEST_PATH);
        assert_eq!(path, expected);
        Ok(())
    }

    #[test]
    fn read_config_works() -> Result<()> {
        let args = Cli::try_parse_from(&[env!("CARGO_PKG_NAME"), "-c", TEST_PATH])?;
        let defaults = Defaults::test_defaults();
        let path = config_file_path(&args, defaults)?;
        let path_c = path.clone();
        let ctx = |msg: &'static str| -> String { format!("{msg} {}", path_c.display()) };
        let file_contents = read_config_file(path, &ctx)?;
        for (actual, expected) in file_contents.lines().zip(TEST_CONFIG.lines()) {
            assert_eq!(actual, expected);
        }
        Ok(())
    }

    #[test]
    fn read_config_fails() -> Result<()> {
        let args = Cli::try_parse_from(&[env!("CARGO_PKG_NAME"), "-c", BAD_PATH])?;
        let defaults = Defaults::test_defaults();
        let path = config_file_path(&args, defaults)?;
        let path_c = path.clone();
        let ctx = |msg: &'static str| -> String { format!("{msg} {}", path_c.display()) };
        match read_config_file(path, &ctx) {
            Ok(_) => Err(anyhow!("This config file shouldn't exist!")),
            Err(e) => {
                assert_eq!(format!("{e:?}"), BAD_CONFIG_ERROR);
                Ok(())
            }
        }
    }

    #[test]
    fn parse_config_fails() -> Result<()> {
        let args = Cli::try_parse_from(&[env!("CARGO_PKG_NAME"), "-c", BAD_TOML_TEST_PATH])?;
        match load::<TomlConfig, Config>(&args, PudxBinary::Test) {
            Ok(_) => Err(anyhow!("This load should fail!")),
            Err(e) => {
                assert_eq!(format!("{e:?}"), BAD_PARSE_ERROR);
                Ok(())
            }
        }
    }

    #[test]
    fn bad_config_workers_fails() -> Result<()> {
        let args =
            Cli::try_parse_from(&[env!("CARGO_PKG_NAME"), "-c", BAD_WORKERS_TOML_TEST_PATH])?;
        match load::<TomlConfig, Config>(&args, PudxBinary::Test) {
            Ok(_) => Err(anyhow!("This load should fail!")),
            Err(e) => {
                assert_eq!(format!("{e:?}"), BAD_WORKERS_PARSE_ERROR);
                Ok(())
            }
        }
    }
}
