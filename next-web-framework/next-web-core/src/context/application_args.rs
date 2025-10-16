use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
pub struct ApplicationArgs {
    #[arg(long)]
    pub config_location: Option<String>,

    #[arg(long)]
    pub decrypt_password: Option<String>,
}

impl Default for ApplicationArgs {
    fn default() -> Self {
        Self::parse()
    }
}
