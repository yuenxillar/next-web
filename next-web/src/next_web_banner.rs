use std::io;

use next_web_core::env::Environment;

use crate::{Banner, NextWebVersion};

const BANNER: &str = r#"
 _    _              _                           _
|  \ | |            | |                         | |
|   \| |  ___ __  __| |_  ______ __      __ ___ | |__
|  . ` | / _ \\ \/ /| __||______|\ \ /\ / // _ \| '_ \
|  |\  ||  __/ >  < | |_          \ V  V /|  __/| |_) |
\__| \_/ \___|/_/\_\ \__|          \_/\_/  \___||_.__/
"#;

/// Default Banner implementation which writes the 'Next-Web' banner.
#[derive(Default, Clone)]
pub struct NextWebBanner;

impl NextWebBanner {
    fn _print_banner(
        &self,
        _environment: &dyn Environment,
        out: &mut dyn std::io::prelude::Write,
    ) -> io::Result<()> {
        // The banner is rendered into a buffer and written once: every write to
        // a console, a pipe or a file costs a system call, so writing the
        // banner line by line is needlessly expensive during the startup.
        let mut banner = String::with_capacity(BANNER.len() + 256);

        // Leading blank line.
        banner.push('\n');

        // Banner text (no trailing newline added by the caller).
        banner.push_str(BANNER);

        // Green title and version.
        banner.push_str(&format!(
            "\n{} \t\t\t\t (v{}.Develop)",
            Self::green(":: Next Web ::"),
            NextWebVersion::get_version()
        ));
        banner.push('\n');

        // Bold home page with underlined URL.
        banner.push_str(&format!(
            "\n{}",
            Self::bold(&format!(
                "Home Page: \x1b[4m{}\x1b[24m",
                env!("CARGO_PKG_HOMEPAGE")
            ))
        ));
        banner.push('\n');

        // Bold thank-you message.
        banner.push_str(&Self::bold("Thank you for using it."));
        banner.push('\n');

        // Trailing blank line.
        banner.push('\n');

        out.write_all(banner.as_bytes())
    }
}

impl Banner for NextWebBanner {
    fn print_banner(
        &self,
        environment: &dyn Environment,
        out: &mut dyn std::io::prelude::Write,
    ) -> io::Result<()> {
        self._print_banner(environment, out)
    }
}

#[allow(dead_code)]
impl NextWebBanner {
    const RED: &str = "\x1b[31m";
    const GREEN: &str = "\x1b[32m";
    const YELLOW: &str = "\x1b[33m";
    const BLUE: &str = "\x1b[34m";
    #[allow(unused)]
    const MAGENTA: &str = "\x1b[35m";
    #[allow(unused)]
    const CYAN: &str = "\x1b[36m";
    #[allow(unused)]
    const WHITE: &str = "\x1b[37m";
    const BOLD: &str = "\x1b[1m";
    const RESET: &str = "\x1b[0m";

    pub fn red(msg: &str) -> String {
        format!("{}{}{}", Self::RED, msg, Self::RESET)
    }

    pub fn green(msg: &str) -> String {
        format!("{}{}{}", Self::GREEN, msg, Self::RESET)
    }

    pub fn yellow(msg: &str) -> String {
        format!("{}{}{}", Self::YELLOW, msg, Self::RESET)
    }

    pub fn blue(msg: &str) -> String {
        format!("{}{}{}", Self::BLUE, msg, Self::RESET)
    }

    pub fn bold(msg: &str) -> String {
        format!("{}{}{}", Self::BOLD, msg, Self::RESET)
    }
}
