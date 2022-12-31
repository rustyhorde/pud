// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Header

use crate::log::Config as LogConfig;
use anyhow::Result;
use console::Style;
use indexmap::IndexSet;
use lazy_static::lazy_static;
use rand::Rng;
use std::io::Write;
use tracing::{info, Level};

lazy_static! {
    static ref VERGEN_MAP: IndexSet<(&'static str, &'static str, &'static str)> = {
        let mut vergen_set = IndexSet::new();
        let _ = vergen_set.insert(("Timestamp", "build", env!("VERGEN_BUILD_TIMESTAMP")));
        let _ = vergen_set.insert(("SemVer", "build", env!("CARGO_PKG_VERSION")));
        let _ = vergen_set.insert(("Branch", "git", env!("VERGEN_GIT_BRANCH")));
        let _ = vergen_set.insert(("Commit SHA", "git", env!("VERGEN_GIT_SHA")));
        let _ = vergen_set.insert((
            "Commit Timestamp",
            "git",
            env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
        ));
        let _ = vergen_set.insert(("Describe", "git", env!("VERGEN_GIT_DESCRIBE")));
        let _ = vergen_set.insert(("Channel", "rustc", env!("VERGEN_RUSTC_CHANNEL")));
        let _ = vergen_set.insert(("Commit Date", "rustc", env!("VERGEN_RUSTC_COMMIT_DATE")));
        let _ = vergen_set.insert(("Commit SHA", "rustc", env!("VERGEN_RUSTC_COMMIT_HASH")));
        let _ = vergen_set.insert(("Host Triple", "rustc", env!("VERGEN_RUSTC_HOST_TRIPLE")));
        if let Some(llvm_version) = option_env!("VERGEN_RUSTC_LLVM_VERSION") {
            let _ = vergen_set.insert(("LLVM Version", "rustc", llvm_version));
        }
        let _ = vergen_set.insert(("SemVer", "rustc", env!("VERGEN_RUSTC_SEMVER")));
        let _ = vergen_set.insert(("Debug", "cargo", env!("VERGEN_CARGO_DEBUG")));
        let _ = vergen_set.insert(("Features", "cargo", env!("VERGEN_CARGO_FEATURES")));
        let _ = vergen_set.insert(("OptLevel", "cargo", env!("VERGEN_CARGO_OPT_LEVEL")));
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

/// Generate a pretty header
///
/// # Errors
///
pub fn header<T, U>(config: &T, prefix: &'static str, writer: Option<&mut U>) -> Result<()>
where
    T: LogConfig,
    U: Write + ?Sized,
{
    let mut rng = rand::thread_rng();
    let app_style = from_u8(rng.gen_range(0..7));
    let bold_blue = Style::new().bold().blue();
    let bold_green = Style::new().bold().green();
    if let Some(writer) = writer {
        output_to_writer(writer, &app_style, &bold_blue, &bold_green, prefix)?;
    } else if let Some(level) = config.level() {
        if level >= Level::INFO {
            trace(&app_style, &bold_blue, &bold_green, prefix);
        }
    }
    Ok(())
}

fn output_to_writer<T>(
    writer: &mut T,
    app_style: &Style,
    bold_blue: &Style,
    bold_green: &Style,
    prefix: &'static str,
) -> Result<()>
where
    T: Write + ?Sized,
{
    for line in prefix.lines() {
        writeln!(writer, "{}", app_style.apply_to(line))?;
    }
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

fn trace(app_style: &Style, bold_blue: &Style, bold_green: &Style, prefix: &'static str) {
    for line in prefix.lines() {
        info!("{}", app_style.apply_to(line));
    }
    info!("");
    info!("{}", bold_green.apply_to("4a61736f6e204f7a696173"));
    info!("");
    for (prefix, kind, value) in &*VERGEN_MAP {
        let key = format!("{:>16} ({:>7})", *prefix, *kind);
        let blue_key = bold_blue.apply_to(key);
        let green_val = bold_green.apply_to(*value);
        info!("{blue_key}: {green_val}");
    }
    info!("");
}

#[cfg(test)]
mod test {
    use super::{from_u8, header};
    use crate::log::Config as LogConfig;
    use anyhow::Result;
    use console::Style;
    use lazy_static::lazy_static;
    use regex::Regex;
    use tracing::Level;

    const HEADER_PREFIX: &str = r#"██████╗ ██╗   ██╗██████╗ ██╗    ██╗
██╔══██╗██║   ██║██╔══██╗██║    ██║
██████╔╝██║   ██║██║  ██║██║ █╗ ██║
██╔═══╝ ██║   ██║██║  ██║██║███╗██║
██║     ╚██████╔╝██████╔╝╚███╔███╔╝
╚═╝      ╚═════╝ ╚═════╝  ╚══╝╚══╝ "#;

    struct TestConfig {
        verbose: u8,
        quiet: u8,
        level: Option<Level>,
    }

    impl Default for TestConfig {
        fn default() -> Self {
            Self {
                verbose: 3,
                quiet: 0,
                level: Some(Level::INFO),
            }
        }
    }

    impl LogConfig for TestConfig {
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
            false
        }

        fn thread_id(&self) -> bool {
            false
        }

        fn thread_names(&self) -> bool {
            false
        }

        fn line_numbers(&self) -> bool {
            false
        }

        fn with_level(&self) -> bool {
            true
        }
    }

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
        assert!(header(&TestConfig::default(), HEADER_PREFIX, Some(&mut buf)).is_ok());
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
        assert!(header(&TestConfig::default(), HEADER_PREFIX, Some(&mut buf)).is_ok());
        assert!(!buf.is_empty());
        let header_str = String::from_utf8_lossy(&buf);
        assert!(BUILD_TIMESTAMP.is_match(&header_str));
        assert!(BUILD_SEMVER.is_match(&header_str));
        assert!(GIT_BRANCH.is_match(&header_str));
        Ok(())
    }
}
