// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Logging

use anyhow::Result;
use tracing::Level;
#[cfg(not(test))]
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{self, time::UtcTime},
    prelude::__tracing_subscriber_SubscriberExt,
    registry,
    util::SubscriberInitExt,
};

pub(crate) trait LogConfig {
    fn quiet(&self) -> u8;
    fn verbose(&self) -> u8;
    fn set_level(&mut self, level: Level) -> &mut Self;
}

pub(crate) fn initialize<T: LogConfig>(config: &mut T) -> Result<()> {
    register(config)
}

#[cfg(test)]
fn register<T: LogConfig>(config: &mut T) -> Result<()> {
    let level = get_effective_level(config.quiet(), config.verbose());
    let _ = config.set_level(level);
    Ok(())
}

#[cfg(not(test))]
fn register<T: LogConfig>(config: &mut T) -> Result<()> {
    let format = fmt::layer()
        .with_level(true)
        .with_ansi(true)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_line_number(true)
        .with_timer(UtcTime::rfc_3339());
    let level = get_effective_level(config.quiet(), config.verbose());
    let _ = config.set_level(level);
    let filter_layer = LevelFilter::from(level);
    Ok(registry().with(format).with(filter_layer).try_init()?)
}

#[cfg(debug_assertions)]
fn get_effective_level(quiet: u8, verbosity: u8) -> Level {
    let mut level = match verbosity {
        0 => Level::INFO,
        1 => Level::DEBUG,
        _ => Level::TRACE,
    };
    level = match quiet {
        0 => level,
        1 => Level::WARN,
        _ => Level::ERROR,
    };
    level
}

#[cfg(not(debug_assertions))]
fn get_effective_level(_quiet: u8, verbosity: u8) -> Level {
    match verbosity {
        0 => Level::ERROR,
        1 => Level::WARN,
        2 => Level::INFO,
        3 => Level::DEBUG,
        4 | _ => Level::TRACE,
    }
}

#[cfg(test)]
mod test {
    use super::get_effective_level;
    use tracing::Level;

    #[cfg(debug_assertions)]
    #[test]
    fn get_effective_level_works() {
        assert_eq!(Level::INFO, get_effective_level(0, 0));
        assert_eq!(Level::DEBUG, get_effective_level(0, 1));
        assert_eq!(Level::TRACE, get_effective_level(0, 2));
        assert_eq!(Level::TRACE, get_effective_level(0, 3));
        assert_eq!(Level::WARN, get_effective_level(1, 0));
        assert_eq!(Level::WARN, get_effective_level(1, 1));
        assert_eq!(Level::WARN, get_effective_level(1, 2));
        assert_eq!(Level::WARN, get_effective_level(1, 3));
        assert_eq!(Level::ERROR, get_effective_level(2, 0));
        assert_eq!(Level::ERROR, get_effective_level(2, 1));
        assert_eq!(Level::ERROR, get_effective_level(2, 2));
        assert_eq!(Level::ERROR, get_effective_level(2, 3));
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn get_effective_level_works() {
        assert_eq!(Level::ERROR, get_effective_level(0, 0));
        assert_eq!(Level::WARN, get_effective_level(0, 1));
        assert_eq!(Level::INFO, get_effective_level(0, 2));
        assert_eq!(Level::DEBUG, get_effective_level(0, 3));
        assert_eq!(Level::TRACE, get_effective_level(0, 4));
        assert_eq!(Level::TRACE, get_effective_level(0, 5));
    }
}
