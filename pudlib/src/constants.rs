// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Constants

use const_format::concatcp;

// Constants building blocks
const COULD_NOT: &str = "Could not ";
const CONFIG_FILE: &str = " config file!";

/// The configuration file base path for puds
pub(crate) const CONFIG_FILE_BASE_PATH_PUDS: &str = "puds";
/// The configuration file base name for puds
pub(crate) const CONFIG_FILE_NAME_PUDS: &str = concatcp!(CONFIG_FILE_BASE_PATH_PUDS, ".toml");
/// The configuration file base path for pudw
pub(crate) const CONFIG_FILE_BASE_PATH_PUDW: &str = "pudw";
/// The configuration file base name for pudw
pub(crate) const CONFIG_FILE_NAME_PUDW: &str = concatcp!(CONFIG_FILE_BASE_PATH_PUDW, ".toml");
#[cfg(test)]
/// The configuration file base path for test
pub(crate) const CONFIG_FILE_BASE_PATH_TEST: &str = "pudw";
#[cfg(test)]
/// The configuration file base name for test
pub(crate) const CONFIG_FILE_NAME_TEST: &str = concatcp!(CONFIG_FILE_BASE_PATH_TEST, ".toml");
/// Context if the config file is unable to be parsed
pub(crate) const UNABLE: &str = concatcp!(COULD_NOT, "parse", CONFIG_FILE);
/// Context if the config file is unable to be read
pub(crate) const READ: &str = concatcp!(COULD_NOT, "read", CONFIG_FILE);
/// Context if the config file is unable to be opened
pub(crate) const FILE_OPEN: &str = concatcp!(COULD_NOT, "open", CONFIG_FILE);

#[cfg(test)]
pub(crate) const TEST_PATH: &str = "test/config.toml";
