// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! health endpoint model structs

use serde::Serialize;
#[cfg(test)]
use {getset::Getters, serde::Deserialize};

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(test, derive(Deserialize, Getters))]
#[cfg_attr(test, getset(get = "pub(crate)"))]
pub(crate) struct Response<T>
where
    T: Into<String>,
{
    status: T,
}

impl Response<&'static str> {
    pub(crate) fn healthy() -> Self {
        Response { status: "healthy" }
    }
}
