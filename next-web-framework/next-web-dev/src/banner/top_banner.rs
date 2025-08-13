
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
        println!("\nversion: {}", env!("CARGO_PKG_VERSION"));
        print!("\n\n");
    }
}
