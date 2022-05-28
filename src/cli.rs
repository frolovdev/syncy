use std::vec;

use anyhow::{Context, Result};
use clap::Parser;
use regex::RegexSet;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};
use serde_yaml;

#[derive(Parser)]
pub struct Args {
    #[clap(short, long, parse(from_os_str))]
    config: std::path::PathBuf,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SourceRepository {
    pub owner: String,
    pub name: String,
    pub git_ref: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct DestinationRepository {
    pub owner: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]

pub struct MoveArgs {
    pub before: String,
    pub after: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub enum Transformation {
    Move { args: MoveArgs },
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ParsedConfig {
    pub version: String,
    pub source: SourceRepository,
    pub destinations: Vec<DestinationRepository>,
    pub token: String,
    pub destination_files: Option<String>,
    pub origin_files: Option<String>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_transformations")]
    pub transformations: Option<Vec<Transformation>>,
}

fn deserialize_transformations<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<Transformation>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct TransformationsVisitor;

    impl<'de> Visitor<'de> for TransformationsVisitor {
        type Value = Option<Vec<Transformation>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string containing Vec<Transformation>")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            while let Some(value) = seq.next_element::<serde_json::Value>()? {
                let transformation = &value
                    .get("fn")
                    .and_then(|v| {
                        if v == "builtin.move" {
                            let before = value
                                .get("args")
                                .and_then(|v| v.get("before"))
                                .expect("builtin.move.args should contain before")
                                .as_str()
                                .unwrap()
                                .to_owned();

                            let after = value
                                .get("args")
                                .and_then(|v| v.get("after"))
                                .expect("builtin.move.args should contain after")
                                .as_str()
                                .unwrap()
                                .to_owned();
                            let args = MoveArgs { before, after };
                            Some(Transformation::Move { args })
                        } else {
                            panic!("transformations.fn should be one of reserved functions")
                        }
                    })
                    .expect("transformation should contain fn property");

                return Ok(Some(vec![transformation.to_owned()]));
            }

            Err(de::Error::custom(format!(
                "Didn't find the right sequence of values in Transformations"
            )))
        }
    }

    deserializer.deserialize_any(TransformationsVisitor)
}

#[derive(Clone, Debug)]
pub struct EnhancedParsedConfig {
    pub version: String,
    pub source: SourceRepository,
    pub destinations: Vec<DestinationRepository>,
    pub token: String,
    pub destination_files: Option<GlobExpression>,
    pub origin_files: Option<GlobExpression>,
    pub transformations: Option<Vec<Transformation>>,
}

pub fn run() -> Result<EnhancedParsedConfig, Box<dyn std::error::Error>> {
    let args = Args::parse();

    let result = std::fs::read_to_string(&args.config)
        .with_context(|| format!("could not read file `{:?}`", &args.config))?;

    let content = parse_config(&result).expect("Can't parse config");

    let enhanced_config = enhance_config(content);

    Ok(enhanced_config)
}

fn parse_config(config: &str) -> Result<ParsedConfig, Box<dyn std::error::Error>> {
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
        transformations: config.transformations,
    }
}

fn parse_glob(val: &str) -> GlobExpression {
    let re_set = RegexSet::new(&["glob\\(\".*?\", \".*?\"\\)", "glob\\(\".*?\"\\)"]).unwrap();
    let result = re_set.matches(&val);

    let matched_any = result.matched_any();
    let single_with_exclude = result.matched(0);
    let single = result.matched(1);

    let len = &val.len();

    if matched_any && single_with_exclude {
        let comma_position = &val.find(",").unwrap();

        let first_glob_end = comma_position - 1;
        let glob_pattern = glob::Pattern::new(&val[6..first_glob_end]).unwrap();

        let start_second_glob = comma_position + 3;
        let end_second_glob = len - 2;

        let second_glob_pattern =
            glob::Pattern::new(&val[start_second_glob..end_second_glob]).unwrap();

        return GlobExpression::SingleWithExclude(glob_pattern, second_glob_pattern);
    }

    if matched_any && single {
        let end = len - 2;
        let pattern = &val[6..end];

        let glob_pattern = glob::Pattern::new(pattern).unwrap();
        return GlobExpression::Single(glob_pattern);
    }

    panic!("invalid glob string");
}

#[cfg(test)]
mod tests {

    mod deserialize_transformations {

        use crate::cli::{MoveArgs, Transformation};

        use super::super::{parse_config, DestinationRepository, ParsedConfig, SourceRepository};
        use indoc::indoc;

        #[test]
        fn test_success() {
            let doc = indoc! {r#"
            version: 0.0.1

            source:
              owner: my_name
              name: test1
              git_ref: main
            
            destinations:
              - owner: my_name
                name: test2
            
            token: random_token
            
            origin_files: glob("**")
            
            destination_files: glob("my_folder/**")
            
            transformations:
              - fn: builtin.move
                args:
                  before: ""
                  after: my_folder  
            "#};

            let parsed_config = parse_config(&doc).unwrap();

            let expected_source = SourceRepository {
                owner: "my_name".to_string(),
                name: "test1".to_string(),
                git_ref: "main".to_string(),
            };

            let expected_destination = DestinationRepository {
                owner: "my_name".to_string(),
                name: "test2".to_string(),
            };

            let expected_transformation_args = MoveArgs {
                before: "".to_string(),
                after: "my_folder".to_string(),
            };
            let expected_transformation = Transformation::Move {
                args: expected_transformation_args,
            };
            let expected_config = ParsedConfig {
                version: "0.0.1".to_string(),
                source: expected_source,
                destinations: vec![expected_destination],
                token: "random_token".to_string(),
                origin_files: Some("glob(\"**\")".to_string()),
                destination_files: Some("glob(\"my_folder/**\")".to_string()),
                transformations: Some(vec![expected_transformation]),
            };

            assert_eq!(parsed_config, expected_config);
        }

        #[test]
        fn test_when_no_transformation_given() {
            let doc = indoc! {r#"
            version: 0.0.1

            source:
              owner: my_name
              name: test1
              git_ref: main
            
            destinations:
              - owner: my_name
                name: test2
            
            token: random_token
            
            origin_files: glob("**")
            
            destination_files: glob("my_folder/**")
            
            "#};

            let parsed_config = parse_config(&doc).unwrap();

            let expected_source = SourceRepository {
                owner: "my_name".to_string(),
                name: "test1".to_string(),
                git_ref: "main".to_string(),
            };

            let expected_destination = DestinationRepository {
                owner: "my_name".to_string(),
                name: "test2".to_string(),
            };

            let expected_config = ParsedConfig {
                version: "0.0.1".to_string(),
                source: expected_source,
                destinations: vec![expected_destination],
                token: "random_token".to_string(),
                origin_files: Some("glob(\"**\")".to_string()),
                destination_files: Some("glob(\"my_folder/**\")".to_string()),
                transformations: None,
            };

            assert_eq!(parsed_config, expected_config);
        }

        #[test]
        #[should_panic(expected = "transformations.fn should be one of reserved functions")]
        fn test_transformations_wrong_fn() {
            let doc = indoc! {r#"
            version: 0.0.1

            source:
              owner: my_name
              name: test1
              git_ref: main
            
            destinations:
              - owner: my_name
                name: test2
            
            token: random_token
            
            origin_files: glob("**")
            
            destination_files: glob("my_folder/**")
            transformations:
                - fn: random
            "#};

            parse_config(&doc);
        }

        #[test]
        #[should_panic(expected = "builtin.move.args should contain after")]
        fn test_transformations_move_after() {
            let doc = indoc! {r#"
            version: 0.0.1

            source:
              owner: my_name
              name: test1
              git_ref: main
            
            destinations:
              - owner: my_name
                name: test2
            
            token: random_token
            
            origin_files: glob("**")
            
            destination_files: glob("my_folder/**")
            transformations:
              -  fn: builtin.move
                 args:
                   before: random  
            "#};

            parse_config(&doc);
        }

        #[test]
        #[should_panic(expected = "builtin.move.args should contain before")]
        fn test_transformations_move_before() {
            let doc = indoc! {r#"
            version: 0.0.1

            source:
              owner: my_name
              name: test1
              git_ref: main
            
            destinations:
              - owner: my_name
                name: test2
            
            token: random_token
            
            origin_files: glob("**")
            
            destination_files: glob("my_folder/**")
            transformations:
                - fn: builtin.move
                  args:
                    after: random  
            "#};

            parse_config(&doc);
        }
    }
}
