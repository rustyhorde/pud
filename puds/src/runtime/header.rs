// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Header

use anyhow::Result;
use console::Style;
use indexmap::IndexSet;
use lazy_static::lazy_static;
use rand::Rng;
use std::io::Write;

lazy_static! {
    static ref VERGEN_MAP: IndexSet<(&'static str, &'static str, &'static str)> = {
        let mut vergen_set = IndexSet::new();
        let _ = vergen_set.insert(("Timestamp", "build", env!("VERGEN_BUILD_TIMESTAMP")));
        let _ = vergen_set.insert(("SemVer", "build", env!("VERGEN_BUILD_SEMVER")));
        let _ = vergen_set.insert(("Branch", "git", env!("VERGEN_GIT_BRANCH")));
        let _ = vergen_set.insert(("Commit SHA", "git", env!("VERGEN_GIT_SHA")));
        let _ = vergen_set.insert((
            "Commit Timestamp",
            "git",
            env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
        ));
        let _ = vergen_set.insert(("SemVer", "git", env!("VERGEN_GIT_SEMVER")));
        let _ = vergen_set.insert(("Channel", "rustc", env!("VERGEN_RUSTC_CHANNEL")));
        let _ = vergen_set.insert(("Commit Date", "rustc", env!("VERGEN_RUSTC_COMMIT_DATE")));
        let _ = vergen_set.insert(("Commit SHA", "rustc", env!("VERGEN_RUSTC_COMMIT_HASH")));
        let _ = vergen_set.insert(("Host Triple", "rustc", env!("VERGEN_RUSTC_HOST_TRIPLE")));
        if let Some(llvm_version) = option_env!("VERGEN_RUSTC_LLVM_VERSION") {
            let _ = vergen_set.insert(("LLVM Version", "rustc", llvm_version));
        }
        let _ = vergen_set.insert(("SemVer", "rustc", env!("VERGEN_RUSTC_SEMVER")));
        let _ = vergen_set.insert(("Features", "cargo", env!("VERGEN_CARGO_FEATURES")));
        let _ = vergen_set.insert(("Profile", "cargo", env!("VERGEN_CARGO_PROFILE")));
        let _ = vergen_set.insert(("Target Triple", "cargo", env!("VERGEN_CARGO_TARGET_TRIPLE")));
        let _ = vergen_set.insert(("Name", "sysinfo", env!("VERGEN_SYSINFO_NAME")));
        let _ = vergen_set.insert(("OS Version", "sysinfo", env!("VERGEN_SYSINFO_OS_VERSION")));
        if let Some(user) = option_env!("VERGEN_SYSINFO_USER") {
            let _ = vergen_set.insert(("User", "sysinfo", user));
        }
        let _ = vergen_set.insert(("Memory", "sysinfo", env!("VERGEN_SYSINFO_TOTAL_MEMORY")));
        let _ = vergen_set.insert(("CPU Vendor", "sysinfo", env!("VERGEN_SYSINFO_CPU_VENDOR")));
        let _ = vergen_set.insert((
            "CPU Cores",
            "sysinfo",
            env!("VERGEN_SYSINFO_CPU_CORE_COUNT"),
        ));
        let _ = vergen_set.insert(("CPU Names", "sysinfo", env!("VERGEN_SYSINFO_CPU_NAME")));
        let _ = vergen_set.insert(("CPU Brand", "sysinfo", env!("VERGEN_SYSINFO_CPU_BRAND")));
        let _ = vergen_set.insert((
            "CPU Frequency",
            "sysinfo",
            env!("VERGEN_SYSINFO_CPU_FREQUENCY"),
        ));
        vergen_set
    };
}

fn from_u8(val: u8) -> Style {
    let style = Style::new();
    match val {
        0 => style.green(),
        1 => style.yellow(),
        2 => style.blue(),
        3 => style.magenta(),
        4 => style.cyan(),
        5 => style.white(),
        _ => style.red(),
    }
}

pub(crate) fn header<T>(writer: &mut T) -> Result<()>
where
    T: Write,
{
    let mut rng = rand::thread_rng();
    let app_style = from_u8(rng.gen_range(0..7));
    let bold_blue = Style::new().bold().blue();
    let bold_green = Style::new().bold().green();

    writeln!(writer, "{}", app_style.apply_to("puds"))?;
    writeln!(writer)?;
    writeln!(writer, "{}", bold_green.apply_to("4a61736f6e204f7a696173"))?;
    writeln!(writer)?;
    for (prefix, kind, value) in &*VERGEN_MAP {
        let key = format!("{:>16} ({:>7})", *prefix, *kind);
        let blue_key = bold_blue.apply_to(key);
        let green_val = bold_green.apply_to(*value);
        writeln!(writer, "{blue_key}: {green_val}")?;
    }
    writeln!(writer)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::{from_u8, header};
    use anyhow::Result;
    use console::Style;
    use lazy_static::lazy_static;
    use regex::Regex;

    lazy_static! {
        static ref BUILD_TIMESTAMP: Regex = Regex::new(r#"Timestamp \(  build\)"#).unwrap();
        static ref BUILD_SEMVER: Regex = Regex::new(r#"SemVer \(  build\)"#).unwrap();
        static ref GIT_BRANCH: Regex = Regex::new(r#"Branch \(    git\)"#).unwrap();
    }

    #[test]
    fn from_u8_works() {
        assert_eq!(from_u8(0), Style::new().green());
        assert_eq!(from_u8(1), Style::new().yellow());
        assert_eq!(from_u8(2), Style::new().blue());
        assert_eq!(from_u8(3), Style::new().magenta());
        assert_eq!(from_u8(4), Style::new().cyan());
        assert_eq!(from_u8(5), Style::new().white());
        assert_eq!(from_u8(6), Style::new().red());
        assert_eq!(from_u8(7), Style::new().red());
    }

    #[test]
    #[cfg(debug_assertions)]
    fn header_writes() -> Result<()> {
        let mut buf = vec![];
        assert!(header(&mut buf).is_ok());
        assert!(!buf.is_empty());
        let header_str = String::from_utf8_lossy(&buf);
        assert!(BUILD_TIMESTAMP.is_match(&header_str));
        assert!(BUILD_SEMVER.is_match(&header_str));
        assert!(GIT_BRANCH.is_match(&header_str));
        Ok(())
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn header_writes() -> Result<()> {
        let mut buf = vec![];
        assert!(header(&matches, &mut buf).is_ok());
        assert!(!buf.is_empty());
        let header_str = String::from_utf8_lossy(&buf);
        assert!(BUILD_TIMESTAMP.is_match(&header_str));
        assert!(BUILD_SEMVER.is_match(&header_str));
        assert!(GIT_BRANCH.is_match(&header_str));
        Ok(())
    }
}
