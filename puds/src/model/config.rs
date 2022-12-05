// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Configuration Models

use crate::{
    error::Error::{self, AddrParse},
    runtime::log::LogConfig,
};
use getset::{Getters, Setters};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};
use tracing::Level;

/// The configuration
#[derive(Clone, Debug, Eq, Getters, PartialEq, Setters)]
#[getset(get = "pub(crate)")]
pub(crate) struct Config {
    #[getset(set = "pub(crate)")]
    quiet: u8,
    #[getset(set = "pub(crate)")]
    verbose: u8,
    workers: u8,
    socket_addr: SocketAddr,
    cert_file_path: String,
    key_file_path: String,
    hostlist: BTreeMap<String, Hosts>,
    level: Option<Level>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            quiet: u8::default(),
            verbose: u8::default(),
            workers: u8::default(),
            socket_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 32276),
            cert_file_path: String::default(),
            key_file_path: String::default(),
            hostlist: BTreeMap::default(),
            level: Option::default(),
        }
    }
}

impl LogConfig for Config {
    fn quiet(&self) -> u8 {
        self.quiet
    }

    fn verbose(&self) -> u8 {
        self.verbose
    }

    fn set_level(&mut self, level: Level) -> &mut Self {
        self.level = Some(level);
        self
    }
}

impl TryFrom<TomlConfig> for Config {
    type Error = Error;

    fn try_from(config: TomlConfig) -> Result<Self, Self::Error> {
        let workers = *config.actix().workers();
        let ip = config.actix().ip();
        let port = config.actix().port();
        let ip_addr: IpAddr = ip.parse().map_err(|e| AddrParse {
            source: e,
            addr: ip.clone(),
        })?;
        let socket_addr = SocketAddr::from((ip_addr, *port));
        let (tls, hostlist) = config.take();
        let (cert_file_path, key_file_path) = tls.take();
        Ok(Config {
            verbose: 0,
            quiet: 0,
            workers,
            socket_addr,
            cert_file_path,
            key_file_path,
            hostlist,
            level: None,
        })
    }
}

/// The TOML configuration.
#[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
#[getset(get = "pub(crate)")]
pub(crate) struct TomlConfig {
    /// The actix server configuration
    actix: Actix,
    /// The TLS configuration
    tls: Tls,
    /// A list of hosts.
    #[serde(serialize_with = "toml::ser::tables_last")]
    hostlist: BTreeMap<String, Hosts>,
}

impl TomlConfig {
    fn take(self) -> (Tls, BTreeMap<String, Hosts>) {
        (self.tls, self.hostlist)
    }
}

/// hosts configuration
#[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
#[getset(get = "pub(crate)")]
pub(crate) struct Actix {
    /// The number of workers to start
    workers: u8,
    /// The IP address to listen on
    ip: String,
    /// The port to listen on
    port: u16,
}

/// TLS configuration
#[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
#[getset(get = "pub(crate)")]
pub(crate) struct Tls {
    /// The number of workers to start
    cert_file_path: String,
    /// The IP address to listen on
    key_file_path: String,
}

impl Tls {
    fn take(self) -> (String, String) {
        (self.cert_file_path, self.key_file_path)
    }
}

/// hosts configuration
#[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
#[getset(get = "pub(crate)")]
pub(crate) struct Hosts {
    /// The hostnames.
    hostnames: Vec<String>,
}
