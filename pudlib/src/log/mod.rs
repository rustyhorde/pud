// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Logging

use anyhow::Result;
use lazy_static::lazy_static;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use time::format_description::well_known::Iso8601;
use tracing::Level;
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{self, time::UtcTime},
    prelude::__tracing_subscriber_SubscriberExt,
    registry,
    util::{SubscriberInitExt, TryInitError},
};

/// Supply quiet, verbose and allow tracing level setting
pub trait Config {
    /// Get the quiet count
    fn quiet(&self) -> u8;
    /// Get the verbose count
    fn verbose(&self) -> u8;
    /// Should we log the event target?
    fn target(&self) -> bool;
    /// Should we log the thread id?
    fn thread_id(&self) -> bool;
    /// Should we log the thread names?
    fn thread_names(&self) -> bool;
    /// Should we log the line numbers?
    fn line_numbers(&self) -> bool;
    /// Shoule we log the level?
    fn with_level(&self) -> bool;
    /// Get the effective tracing level
    fn level(&self) -> Option<Level>;
    /// Allow initialization to set the effective tracing level
    fn set_level(&mut self, level: Level) -> &mut Self;
}

lazy_static! {
    static ref INIT_LOCK: Arc<Mutex<AtomicBool>> = Arc::new(Mutex::new(AtomicBool::new(false)));
}

/// Initialize tracing
///
/// # Errors
/// * An error can be thrown on registry initialization
///
pub fn initialize<T: Config>(config: &mut T) -> Result<()> {
    let init = match INIT_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if init.load(Ordering::SeqCst) {
        // TODO: Should probably return already initialize error?
        Ok(())
    } else {
        let format = fmt::layer()
            .compact()
            .with_level(config.with_level())
            .with_ansi(true)
            .with_target(config.target())
            .with_thread_ids(config.thread_id())
            .with_thread_names(config.thread_names())
            .with_line_number(config.line_numbers())
            .with_timer(UtcTime::new(Iso8601::DEFAULT));
        let level = get_effective_level(config.quiet(), config.verbose());
        let _ = config.set_level(level);
        let filter_layer = LevelFilter::from(level);
        match registry().with(format).with(filter_layer).try_init() {
            Ok(_) => {
                init.store(true, Ordering::SeqCst);
                Ok(())
            }
            Err(e) => ok_on_test(e),
        }
    }
}

#[cfg(not(test))]
fn ok_on_test(e: TryInitError) -> Result<()> {
    Err(e.into())
}

#[cfg(test)]
#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn ok_on_test(_e: TryInitError) -> Result<()> {
    Ok(())
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
