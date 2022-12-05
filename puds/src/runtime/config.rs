// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Configuration

use super::cli::Cli;
use crate::{
    constants::{CONFIG_FILE_BASE_PATH, CONFIG_FILE_NAME, FILE_OPEN, READ, UNABLE},
    error::Error::ConfigDir,
    model::config::{Config, TomlConfig},
};
use anyhow::{Context, Result};
use std::{fs::File, io::Read, path::PathBuf};

pub(crate) fn load(args: &Cli) -> Result<Config> {
    // Determine the configuration file path
    let config_file_path = config_file_path(args)?;
    // Setup error handling
    let path = config_file_path.clone();
    let ctx = |msg: &'static str| -> String { format!("{msg} {}", path.display()) };
    // Read the config file
    let config_file = read_config_file(config_file_path, ctx)?;
    // Parse the config file
    let config: TomlConfig = toml::from_str(&config_file).with_context(|| ctx(UNABLE))?;
    // Convert the toml config to base config
    transform(config, *args.verbose(), *args.quiet())
}

fn config_file_path(args: &Cli) -> Result<PathBuf> {
    args.config_file_path()
        .as_ref()
        .map_or_else(default_config_file_path, to_path_buf)
}

fn default_config_file_path() -> Result<PathBuf> {
    let mut config_file_path = dirs2::config_dir().ok_or(ConfigDir)?;
    config_file_path.push(CONFIG_FILE_BASE_PATH);
    config_file_path.push(CONFIG_FILE_NAME);
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

fn transform(config: TomlConfig, verbose: u8, quiet: u8) -> Result<Config> {
    let mut config: Config = config.try_into()?;
    let _ = config.set_verbose(verbose);
    let _ = config.set_quiet(quiet);
    Ok(config)
}

#[cfg(test)]
mod test {
    use super::{config_file_path, default_config_file_path, load, read_config_file};
    use crate::{constants::TEST_PATH, runtime::cli::Cli};
    use anyhow::{anyhow, Result};
    use clap::Parser;
    use std::path::PathBuf;

    const BAD_PATH: &'static str = "this/path/is/bad/config.toml";
    const BAD_TOML_TEST_PATH: &str = "test/bad.toml";
    const BAD_IP_TOML_TEST_PATH: &str = "test/bad_ip.toml";
    #[cfg(windows)]
    const BAD_CONFIG_ERROR: &str = "Could not open config file! this/path/is/bad/config.toml\n\nCaused by:\n    The system cannot find the path specified. (os error 3)";
    #[cfg(not(windows))]
    const BAD_CONFIG_ERROR: &str = "Could not open config file! this/path/is/bad/config.toml\n\nCaused by:\n    No such file or directory (os error 2)";
    const BAD_PARSE_ERROR: &str =
        "Could not parse config file! test/bad.toml\n\nCaused by:\n    missing field `actix`";
    const BAD_IP_PARSE_ERROR: &str =
        "Failed to parse 'this!ip/jkrfj;isbad'\n\nCaused by:\n    invalid IP address syntax";
    const TEST_CONFIG: &'static str = r#"[actix]
workers = 8
ip = "127.0.0.1"
port = 32276

[tls]
cert_file_path = "fullchain.pem"
key_file_path = "privkey.pem"

[hostlist.most]
hostnames = ["one","two","three"]"#;

    #[test]
    fn config_file_path_is_default() -> Result<()> {
        let args = Cli::try_parse_from(&[env!("CARGO_PKG_NAME")])?;
        let path = config_file_path(&args)?;
        assert_eq!(path, default_config_file_path()?);
        Ok(())
    }

    #[test]
    fn config_file_path_is_set() -> Result<()> {
        let args = Cli::try_parse_from(&[env!("CARGO_PKG_NAME"), "-c", TEST_PATH])?;
        let path = config_file_path(&args)?;
        let expected = PathBuf::from(TEST_PATH);
        assert_eq!(path, expected);
        Ok(())
    }

    #[test]
    fn read_config_works() -> Result<()> {
        let args = Cli::try_parse_from(&[env!("CARGO_PKG_NAME"), "-c", TEST_PATH])?;
        let path = config_file_path(&args)?;
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
        let path = config_file_path(&args)?;
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
        match load(&args) {
            Ok(_) => Err(anyhow!("This load should fail!")),
            Err(e) => {
                assert_eq!(format!("{e:?}"), BAD_PARSE_ERROR);
                Ok(())
            }
        }
    }

    #[test]
    fn bad_config_ip_fails() -> Result<()> {
        let args = Cli::try_parse_from(&[env!("CARGO_PKG_NAME"), "-c", BAD_IP_TOML_TEST_PATH])?;
        match load(&args) {
            Ok(_) => Err(anyhow!("This load should fail!")),
            Err(e) => {
                assert_eq!(format!("{e:?}"), BAD_IP_PARSE_ERROR);
                Ok(())
            }
        }
    }
}
