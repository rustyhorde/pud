// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Errors

use clap::error::ErrorKind;
use serde::{ser::SerializeStruct, Serialize, Serializer};
use std::{error::Error as StdError, net::AddrParseError};

#[allow(variant_size_differences)]
#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error("actix error: {}", msg)]
    Actix { msg: String },
    #[error("Failed to parse '{addr}'")]
    AddrParse {
        #[source]
        source: AddrParseError,
        addr: String,
    },
    #[error("There is no valid config directory")]
    ConfigDir,
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Error", 2)?;
        state.serialize_field("reason", &format!("{self}"))?;
        if let Some(source) = self.source() {
            state.serialize_field("source", &format!("{source}"))?;
        }
        state.end()
    }
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn clap_or_error(err: anyhow::Error) -> i32 {
    let disp_err = || {
        eprint!("{err:?}");
        1
    };
    match err.downcast_ref::<clap::Error>() {
        Some(e) => match e.kind() {
            ErrorKind::DisplayHelp => {
                eprint!("{e}");
                0
            }
            ErrorKind::DisplayVersion => 0,
            _ => disp_err(),
        },
        None => disp_err(),
    }
}

pub(crate) fn success(_: ()) -> i32 {
    0
}

#[cfg(test)]
mod test {
    use super::{clap_or_error, success};
    use anyhow::{anyhow, Error};
    use clap::{
        error::ErrorKind::{self, DisplayHelp, DisplayVersion},
        Command,
    };

    #[test]
    fn success_works() {
        assert_eq!(0, success(()));
    }

    #[test]
    fn clap_or_error_is_error() {
        assert_eq!(1, clap_or_error(anyhow!("test")));
    }

    #[test]
    fn clap_or_error_is_help() {
        let mut cmd = Command::new(env!("CARGO_PKG_NAME"));
        let error = cmd.error(DisplayHelp, "help");
        let clap_error = Error::new(error);
        assert_eq!(0, clap_or_error(clap_error));
    }

    #[test]
    fn clap_or_error_is_version() {
        let mut cmd = Command::new(env!("CARGO_PKG_NAME"));
        let error = cmd.error(DisplayVersion, "1.0");
        let clap_error = Error::new(error);
        assert_eq!(0, clap_or_error(clap_error));
    }

    #[test]
    fn clap_or_error_is_other_clap_error() {
        let mut cmd = Command::new(env!("CARGO_PKG_NAME"));
        let error = cmd.error(ErrorKind::InvalidValue, "Some failure case");
        let clap_error = Error::new(error);
        assert_eq!(1, clap_or_error(clap_error));
    }
}
