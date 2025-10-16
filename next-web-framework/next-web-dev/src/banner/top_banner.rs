pub static DEFAULT_TOP_BANNER: &str = r#"
 _    _              _                           _
|  \ | |            | |                         | |
|   \| |  ___ __  __| |_  ______ __      __ ___ | |__
|  . ` | / _ \\ \/ /| __||______|\ \ /\ / // _ \| '_ \
|  |\  ||  __/ >  < | |_          \ V  V /|  __/| |_) |
\__| \_/ \___|/_/\_\ \__|          \_/\_/  \___||_.__/
"#;

#[derive(Debug)]
pub struct TopBanner;

impl TopBanner {
    pub fn show(banner: &str) {
        print!("{}", banner);
        println!(
            "\n{} \t\t\t\t (v{}.Develop)",
            Self::green(":: Next Web ::"),
            env!("CARGO_PKG_VERSION")
        );
        println!(
            "\n{}",
            Self::bold(&format!(
                "Home Page: \x1b[4m{}\x1b[24m",
                env!("CARGO_PKG_HOMEPAGE")
            ))
        );
        println!("{}", Self::bold("Thank you for using it."));
        print!("\n");
    }
}

impl TopBanner {
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
