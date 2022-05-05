use anyhow::{Context, Result};
use clap::Parser;
use regex::RegexSet;
use serde::{Deserialize, Serialize};
use serde_yaml;

#[derive(Parser)]
pub struct Args {
    #[clap(short, long, parse(from_os_str))]
    config: std::path::PathBuf,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceRepository {
    pub owner: String,
    pub name: String,
    pub git_ref: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DestinationRepository {
    pub owner: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParsedConfig {
    pub version: String,
    pub source: SourceRepository,
    pub destinations: Vec<DestinationRepository>,
    pub token: String,
    pub destination_files: Option<String>,
    pub origin_files: Option<String>,
}

#[derive(Clone, Debug)]
pub struct EnhancedParsedConfig {
    pub version: String,
    pub source: SourceRepository,
    pub destinations: Vec<DestinationRepository>,
    pub token: String,
    pub destination_files: Option<GlobExpression>,
    pub origin_files: Option<GlobExpression>,
}

pub fn run() -> Result<EnhancedParsedConfig, Box<dyn std::error::Error>> {
    let args = Args::parse();

    let result = std::fs::read_to_string(&args.config)
        .with_context(|| format!("could not read file `{:?}`", &args.config))?;

    let content = parse_config(result).expect("Can't parse config");

    let enhanced_config = enhance_config(content);

    Ok(enhanced_config)
}

fn parse_config(config: std::string::String) -> Result<ParsedConfig, Box<dyn std::error::Error>> {
    let deserialized_config: serde_yaml::Result<ParsedConfig> = serde_yaml::from_str(&config);

    let result = match deserialized_config {
        Ok(content) => content,
        Err(error) => {
            return Err(error.into());
        }
    };

    Ok(result)
}

#[derive(Debug, Clone)]
pub enum GlobExpression {
    Single(glob::Pattern),
    SingleWithExclude(glob::Pattern, glob::Pattern),
}

fn enhance_config(config: ParsedConfig) -> EnhancedParsedConfig {
    let origin_files = config.origin_files.as_ref().unwrap();
    let destination_files = config.destination_files.as_ref().unwrap();

    let origin_files_glob = parse_glob(&origin_files);
    let destination_files_glob = parse_glob(&destination_files);

    EnhancedParsedConfig {
        version: config.version,
        source: config.source,
        destinations: config.destinations,
        token: config.token,
        destination_files: Some(destination_files_glob),
        origin_files: Some(origin_files_glob),
    }
}

fn parse_glob(val: &str) -> GlobExpression {
    let re_set = RegexSet::new(&["glob\\(\".*?\", \".*?\"\\)", "glob\\(\".*?\"\\)"]).unwrap();
    let result = re_set.matches(&val);

    let matched_any = result.matched_any();
    let single_with_exclude = result.matched(0);
    let single = result.matched(1);

    let len = &val.len();

    if (matched_any && single_with_exclude) {
        let comma_position = &val.find(",").unwrap();

        let first_glob_end = comma_position - 1;
        let glob_pattern = glob::Pattern::new(&val[6..first_glob_end]).unwrap();

        let start_second_glob = comma_position + 3;
        let end_second_glob = len - 2;

        let second_glob_pattern =
            glob::Pattern::new(&val[start_second_glob..end_second_glob]).unwrap();

        return GlobExpression::SingleWithExclude(glob_pattern, second_glob_pattern);
    }

    if (matched_any && single) {
        let end = len - 2;
        let pattern = &val[6..end];

        let glob_pattern = glob::Pattern::new(pattern).unwrap();
        return GlobExpression::Single(glob_pattern);
    }

    panic!("invalid glob string");
}
