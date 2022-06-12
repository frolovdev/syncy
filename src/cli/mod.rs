pub mod common;
pub mod parser;
pub mod reader;

pub use common::*;
pub use parser::{GlobExpression, MoveArgs, ParsedConfig, Transformation, WorkDirExpression};

use anyhow::{Context, Result};
use clap::Parser;
use parser::parse_config;
use reader::read_config;
#[derive(Parser)]
pub struct Args {
    #[clap(short, long, parse(from_os_str))]
    config: std::path::PathBuf,
}

pub fn run() -> Result<ParsedConfig, Box<dyn std::error::Error>> {
    let args = Args::parse();

    let result = std::fs::read_to_string(&args.config)
        .with_context(|| format!("could not read file `{:?}`", &args.config))?;

    let content = read_config(&result).expect("Can't parse config");
    let enhanced_config = parse_config(content);

    Ok(enhanced_config)
}
