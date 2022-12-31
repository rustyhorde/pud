// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! info endpoint model

use serde::Serialize;
#[cfg(test)]
use {getset::Getters, serde::Deserialize};

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(test, derive(Deserialize, Getters))]
#[cfg_attr(test, getset(get = "pub(crate)"))]
pub(crate) struct Info<T>
where
    T: Into<String>,
{
    build_timestamp: T,
    build_semver: T,
    git_branch: T,
    git_commit_date: T,
    git_describe: T,
    git_sha: T,
    rustc_channel: T,
    rustc_commit_date: T,
    rustc_commit_sha: T,
    rustc_host_triple: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    rustc_llvm_version: Option<T>,
    rustc_semver: T,
    cargo_debug: T,
    cargo_features: T,
    cargo_opt_level: T,
    cargo_target_triple: T,
    sysinfo_name: T,
    sysinfo_os_version: T,
    sysinfo_total_memory: T,
    sysinfo_cpu_vendor: T,
    sysinfo_cpu_core_count: T,
    sysinfo_cpu_name: T,
    sysinfo_cpu_brand: T,
    sysinfo_cpu_frequency: T,
}

impl Info<&'static str> {
    pub(crate) fn new() -> Self {
        Info {
            build_timestamp: env!("VERGEN_BUILD_TIMESTAMP"),
            build_semver: env!("CARGO_PKG_VERSION"),
            git_branch: env!("VERGEN_GIT_BRANCH"),
            git_commit_date: env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
            git_describe: env!("VERGEN_GIT_DESCRIBE"),
            git_sha: env!("VERGEN_GIT_SHA"),
            rustc_channel: env!("VERGEN_RUSTC_CHANNEL"),
            rustc_commit_sha: env!("VERGEN_RUSTC_COMMIT_HASH"),
            rustc_commit_date: env!("VERGEN_RUSTC_COMMIT_DATE"),
            rustc_host_triple: env!("VERGEN_RUSTC_HOST_TRIPLE"),
            rustc_llvm_version: option_env!("VERGEN_RUSTC_LLVM_VERSION"),
            rustc_semver: env!("VERGEN_RUSTC_SEMVER"),
            cargo_debug: env!("VERGEN_CARGO_DEBUG"),
            cargo_features: env!("VERGEN_CARGO_FEATURES"),
            cargo_opt_level: env!("VERGEN_CARGO_OPT_LEVEL"),
            cargo_target_triple: env!("VERGEN_CARGO_TARGET_TRIPLE"),
            sysinfo_name: env!("VERGEN_SYSINFO_NAME"),
            sysinfo_os_version: env!("VERGEN_SYSINFO_OS_VERSION"),
            sysinfo_total_memory: env!("VERGEN_SYSINFO_TOTAL_MEMORY"),
            sysinfo_cpu_vendor: env!("VERGEN_SYSINFO_CPU_VENDOR"),
            sysinfo_cpu_core_count: env!("VERGEN_SYSINFO_CPU_CORE_COUNT"),
            sysinfo_cpu_name: env!("VERGEN_SYSINFO_CPU_NAME"),
            sysinfo_cpu_brand: env!("VERGEN_SYSINFO_CPU_BRAND"),
            sysinfo_cpu_frequency: env!("VERGEN_SYSINFO_CPU_FREQUENCY"),
        }
    }
}
