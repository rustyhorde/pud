// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Configuration Models

use crate::error::Error::{self, AddrParse};
use getset::{Getters, Setters};
use pudlib::{Command, LogConfig, Schedules, Verbosity};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};
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
    workers: u8,
    socket_addr: SocketAddr,
    cert_file_path: String,
    key_file_path: String,
    hostlist: BTreeMap<String, Hosts>,
    level: Option<Level>,
    default: BTreeMap<String, Command>,
    overrides: BTreeMap<String, BTreeMap<String, Command>>,
    schedules: BTreeMap<String, Schedules>,
    log_file_path: PathBuf,
    log_file_name: String,
    db_url: String,
    db_user: String,
    db_pass: String,
    db_name: String,
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

    fn log_file_path(&self) -> PathBuf {
        self.log_file_path.clone()
    }

    fn log_file_name(&self) -> String {
        self.log_file_name.clone()
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
        let db_url = config.arangodb().url().clone();
        let db_user = config.arangodb().user().clone();
        let db_pass = config.arangodb().password().clone();
        let db_name = config.arangodb().name().clone();

        let (target, thread_id, thread_names, line_numbers, log_file_path, log_file_name) =
            if let Some(tracing) = config.tracing() {
                (
                    *tracing.target(),
                    *tracing.thread_id(),
                    *tracing.thread_names(),
                    *tracing.line_numbers(),
                    PathBuf::from(tracing.log_file_path()),
                    tracing.log_file_name().clone(),
                )
            } else {
                (
                    false,
                    false,
                    false,
                    false,
                    PathBuf::from("."),
                    "puds.log".to_string(),
                )
            };
        let socket_addr = SocketAddr::from((ip_addr, *port));
        let (tls, hostlist, default, overrides, schedules) = config.take();
        let (cert_file_path, key_file_path) = tls.take();
        Ok(Config {
            verbose: 0,
            quiet: 0,
            path: PathBuf::new(),
            target,
            thread_id,
            thread_names,
            line_numbers,
            workers,
            socket_addr,
            cert_file_path,
            key_file_path,
            hostlist,
            level: None,
            default,
            overrides,
            schedules,
            log_file_path,
            log_file_name,
            db_url,
            db_user,
            db_pass,
            db_name,
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
    /// The ArangoDB configuration
    arangodb: Arangodb,
    /// The tracing configuration
    tracing: Option<Tracing>,
    /// A list of hosts.
    #[serde(serialize_with = "toml::ser::tables_last")]
    hostlist: BTreeMap<String, Hosts>,
    /// The defaults commands
    default: BTreeMap<String, Command>,
    /// The overrides for specific workers
    overrides: BTreeMap<String, BTreeMap<String, Command>>,
    /// The schedules for specific workers
    #[serde(serialize_with = "toml::ser::tables_last")]
    schedules: BTreeMap<String, Schedules>,
}

type TomlConfigTake = (
    Tls,
    BTreeMap<String, Hosts>,
    BTreeMap<String, Command>,
    BTreeMap<String, BTreeMap<String, Command>>,
    BTreeMap<String, Schedules>,
);

impl TomlConfig {
    fn take(self) -> TomlConfigTake {
        (
            self.tls,
            self.hostlist,
            self.default,
            self.overrides,
            self.schedules,
        )
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

/// hosts configuration
#[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
#[getset(get = "pub(crate)")]
pub(crate) struct Arangodb {
    /// The ArangoDB url
    url: String,
    /// The user
    user: String,
    /// The password
    password: String,
    /// The database name
    name: String,
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
    /// Log file path
    log_file_path: String,
    /// Log file name
    log_file_name: String,
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

/// actix client configuration
#[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
#[getset(get = "pub(crate)")]
pub(crate) struct Override {
    cmds: HashMap<String, Command>,
}
