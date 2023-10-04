use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    config_path: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let process = goalnet_injector::inject(&args.config_path)?;

    unsafe {
        process.run()?;
    }

    Ok(())
}
