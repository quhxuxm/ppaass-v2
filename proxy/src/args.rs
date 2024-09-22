use clap::Parser;
use std::path::{Path, PathBuf};
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// The path of configuration
    #[arg(short, long)]
    configuration_path: PathBuf,
}

impl Args {
    pub fn configuration_path(&self) -> &Path {
        &self.configuration_path
    }
}
