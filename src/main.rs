use std::path::PathBuf;

use clap::Parser;
use color_eyre::eyre::Result;

use slynx::context::SlynxContext;

#[derive(Debug, Parser)]
struct Cli {
    #[arg(short, long)]
    target: String,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    let path = PathBuf::from(cli.target);
    let ctx = SlynxContext::new(path.into())?;
    ctx.start_compilation()?;
    Ok(())
}
