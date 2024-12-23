use clap::Parser;
use std::path::PathBuf;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CommandArgs {
    /// The configuration file path of the proxy
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    /// The rsa folder path of the proxy
    #[arg(short, long)]
    pub rsa: Option<PathBuf>,
}
