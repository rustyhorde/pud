// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// configuration structs

use crate::error::Error;
use getset::{Getters, Setters};
use pudlib::{LogConfig, Verbosity};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::Level;

/// The configuration
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Eq, Getters, PartialEq, Setters)]
#[getset(get = "pub(crate)")]
pub(crate) struct Config {
    #[getset(set = "pub")]
    quiet: u8,
    #[getset(set = "pub")]
    verbose: u8,
    path: PathBuf,
    target: bool,
    thread_id: bool,
    thread_names: bool,
    line_numbers: bool,
    retry_count: usize,
    server_addr: String,
    server_port: u16,
    name: String,
    level: Option<Level>,
    use_tokio: bool,
}

impl Config {
    pub(crate) fn server_url(&self) -> String {
        format!(
            "https://{}:{}/v1/ws/manager?name={}",
            self.server_addr, self.server_port, self.name
        )
    }
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

    fn set_config_file_path(&mut self, config_file_path: PathBuf) -> &mut Self {
        self.path = config_file_path;
        self
    }
}

impl LogConfig for Config {
    fn quiet(&self) -> u8 {
        self.quiet
    }

    fn verbose(&self) -> u8 {
        self.verbose
    }

    fn level(&self) -> Option<Level> {
        self.level
    }

    fn set_level(&mut self, level: Level) -> &mut Self {
        self.level = Some(level);
        self
    }

    fn target(&self) -> bool {
        self.target
    }

    fn thread_id(&self) -> bool {
        self.thread_id
    }

    fn thread_names(&self) -> bool {
        self.thread_names
    }

    fn line_numbers(&self) -> bool {
        self.line_numbers
    }

    fn use_tokio(&self) -> bool {
        self.use_tokio
    }
}

impl TryFrom<TomlConfig> for Config {
    type Error = Error;

    fn try_from(config: TomlConfig) -> Result<Self, Self::Error> {
        let name = config.name().clone();
        let server_addr = config.actix().ip().clone();
        let server_port = *config.actix().port();
        let retry_count = *config.retry_count();
        let (target, thread_id, thread_names, line_numbers) =
            if let Some(tracing) = config.tracing() {
                (
                    *tracing.target(),
                    *tracing.thread_id(),
                    *tracing.thread_names(),
                    *tracing.line_numbers(),
                )
            } else {
                (false, false, false, false)
            };
        Ok(Config {
            verbose: 0,
            quiet: 0,
            path: PathBuf::new(),
            target,
            thread_id,
            thread_names,
            line_numbers,
            retry_count,
            server_addr,
            server_port,
            name,
            level: None,
            use_tokio: false,
        })
    }
}

/// The TOML configuration.
#[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
#[getset(get = "pub(crate)")]
pub(crate) struct TomlConfig {
    /// The actix client configuration
    actix: Actix,
    /// The tracing configuration
    tracing: Option<Tracing>,
    /// The number of time we should try reconnecting
    retry_count: usize,
    /// The name of this worker
    name: String,
}

/// actix client configuration
#[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
#[getset(get = "pub(crate)")]
pub(crate) struct Actix {
    /// The IP address to connect to
    ip: String,
    /// The port to connect to
    port: u16,
}

/// tracing configuration
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
#[getset(get = "pub(crate)")]
pub(crate) struct Tracing {
    /// Should we trace the event target
    target: bool,
    /// Should we trace the thread id
    thread_id: bool,
    /// Should we trace the thread names
    thread_names: bool,
    /// Should we trace the line numbers
    line_numbers: bool,
}
