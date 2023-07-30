use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = mimiron::MimironArgs::parse();
    mimiron::run(args)
}