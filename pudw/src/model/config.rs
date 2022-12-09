// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// configuration structs

use crate::error::Error::{self, AddrParse};
use getset::{Getters, Setters};
use pudlib::Verbosity;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use tracing::Level;

/// The configuration
#[derive(Clone, Debug, Eq, Getters, PartialEq, Setters)]
#[getset(get = "pub(crate)")]
pub(crate) struct Config {
    #[getset(set = "pub")]
    quiet: u8,
    #[getset(set = "pub")]
    verbose: u8,
    retry_count: usize,
    socket_addr: SocketAddr,
    level: Option<Level>,
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

// impl LogConfig for Config {
//     fn quiet(&self) -> u8 {
//         self.quiet
//     }

//     fn verbose(&self) -> u8 {
//         self.verbose
//     }

//     fn set_level(&mut self, level: Level) -> &mut Self {
//         self.level = Some(level);
//         self
//     }
// }

impl TryFrom<TomlConfig> for Config {
    type Error = Error;

    fn try_from(config: TomlConfig) -> Result<Self, Self::Error> {
        let ip = config.actix().ip();
        let port = config.actix().port();
        let retry_count = *config.retry_count();
        let ip_addr: IpAddr = ip.parse().map_err(|e| AddrParse {
            source: e,
            addr: ip.clone(),
        })?;
        let socket_addr = SocketAddr::from((ip_addr, *port));
        Ok(Config {
            verbose: 0,
            quiet: 0,
            retry_count,
            socket_addr,
            level: None,
        })
    }
}

/// The TOML configuration.
#[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
#[getset(get = "pub(crate)")]
pub(crate) struct TomlConfig {
    /// The actix client configuration
    actix: Actix,
    /// The number of time we should try reconnecting
    retry_count: usize,
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
