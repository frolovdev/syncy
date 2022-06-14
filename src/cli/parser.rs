use regex::RegexSet;

use super::{
    common::{DestinationRepository, SourceRepository},
    reader,
};
use regex::Regex;
use std::fmt::Debug;

#[derive(Clone, Debug, PartialEq)]

pub struct MoveArgs {
    pub before: String,
    pub after: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplaceArgs {
    pub before: CustomRegex,
    pub after: String,
}

#[derive(Debug, Clone)]
pub struct CustomRegex(pub Regex);

impl PartialEq for CustomRegex {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }

    fn ne(&self, other: &Self) -> bool {
        self.0.as_str() != other.0.as_str()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Transformation {
    Move { args: MoveArgs },
    Replace { args: ReplaceArgs },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedConfig {
    pub version: String,
    pub source: SourceRepository,
    pub destinations: Vec<DestinationRepository>,
    pub token: String,
    pub destination_files: WorkDirExpression,
    pub origin_files: WorkDirExpression,
    pub transformations: Option<Vec<Transformation>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkDirExpression {
    Glob(GlobExpression),
    Path(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GlobExpression {
    Single(glob::Pattern),
    SingleWithExclude(glob::Pattern, glob::Pattern),
}

pub fn parse_config(config: reader::Config) -> ParsedConfig {
    let origin_files = config.origin_files.unwrap_or("".to_string());
    let destination_files = config.destination_files.unwrap_or("".to_string());

    let origin_files_glob = parse_work_dir_expression(&origin_files);
    let destination_files_glob = parse_work_dir_expression(&destination_files);

    ParsedConfig {
        version: config.version,
        source: config.source,
        destinations: config.destinations,
        token: config.token,
        destination_files: destination_files_glob,
        origin_files: origin_files_glob,
        transformations: parse_transformations(&config.transformations),
    }
}

fn parse_transformations(
    transformations: &Option<Vec<serde_json::Value>>,
) -> Option<Vec<Transformation>> {
    if let Some(unwrapped_transformations) = transformations {
        let mut parsed_transformations = Vec::new();
        for t in unwrapped_transformations.iter() {
            let parsed_transformation = &t
                .get("fn")
                .and_then(|v| {
                    if v == "builtin.move" {
                        let before = t
                            .get("args")
                            .and_then(|v| v.get("before"))
                            .expect("builtin.move.args should contain before")
                            .as_str()
                            .unwrap()
                            .to_owned();

                        let after = t
                            .get("args")
                            .and_then(|v| v.get("after"))
                            .expect("builtin.move.args should contain after")
                            .as_str()
                            .unwrap()
                            .to_owned();
                        let args = MoveArgs { before, after };
                        Some(Transformation::Move { args })
                    } else if v == "builtin.replace" {
                        let before = t
                            .get("args")
                            .and_then(|v| v.get("before"))
                            .expect("builtin.replace.args should contain before")
                            .as_str()
                            .unwrap()
                            .to_owned();

                        let after = t
                            .get("args")
                            .and_then(|v| v.get("after"))
                            .expect("builtin.replace.args should contain after")
                            .as_str()
                            .unwrap()
                            .to_owned();

                        let args = ReplaceArgs {
                            before: CustomRegex(Regex::new(&before).unwrap()),
                            after,
                        };
                        Some(Transformation::Replace { args })
                    } else {
                        panic!("transformations.fn should be one of reserved functions")
                    }
                })
                .expect("transformation should contain fn property");

            parsed_transformations.push(parsed_transformation.to_owned());
        }

        Some(parsed_transformations)
    } else {
        None
    }
}

fn parse_work_dir_expression(val: &str) -> WorkDirExpression {
    if val.starts_with("glob(") {
        parse_glob_expression(val)
    } else {
        parse_path_expression(val)
    }
}

fn parse_path_expression(val: &str) -> WorkDirExpression {
    WorkDirExpression::Path(val.to_string())
}

fn parse_glob_expression(val: &str) -> WorkDirExpression {
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

        return WorkDirExpression::Glob(GlobExpression::SingleWithExclude(
            glob_pattern,
            second_glob_pattern,
        ));
    }

    if matched_any && single {
        let end = len - 2;
        let pattern = &val[6..end];

        let glob_pattern = glob::Pattern::new(pattern).unwrap();
        return WorkDirExpression::Glob(GlobExpression::Single(glob_pattern));
    }

    panic!("invalid glob string");
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        parse_config, GlobExpression, MoveArgs, ParsedConfig, ReplaceArgs, Transformation,
        WorkDirExpression,
    };
    use crate::cli::reader::read_config;
    use crate::fixtures::workdir_path::create_glob_single;
    use crate::{
        cli::{
            common::{DestinationRepository, SourceRepository},
            reader::Config,
        },
        fixtures::workdir_path::create_glob_single_with_exclude,
    };
    use indoc::indoc;

    #[test]
    fn success_workdir_glob() {
        let expected_source = SourceRepository {
            owner: "my_name".to_string(),
            name: "test1".to_string(),
            git_ref: "main".to_string(),
        };

        let expected_destination = DestinationRepository {
            owner: "my_name".to_string(),
            name: "test2".to_string(),
        };

        let transformation_args = json!({
            "before": "",
            "after": "my_folder",
        });
        let transformation = json!({
            "fn": "builtin.move",
            "args": transformation_args,
        });

        let config = Config {
            version: "0.0.1".to_string(),
            source: expected_source.clone(),
            destinations: vec![expected_destination.clone()],
            token: "random_token".to_string(),
            origin_files: Some("glob(\"**\")".to_string()),
            destination_files: Some("glob(\"my_folder/**\")".to_string()),
            transformations: Some(vec![transformation]),
        };

        let parsed_config = parse_config(config.clone());

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
            origin_files: create_glob_single("**"),
            destination_files: create_glob_single("my_folder/**"),
            transformations: Some(vec![expected_transformation]),
        };

        assert_eq!(parsed_config, expected_config)
    }

    #[test]
    fn success_workdir_glob_with_exclude() {
        let expected_source = SourceRepository {
            owner: "my_name".to_string(),
            name: "test1".to_string(),
            git_ref: "main".to_string(),
        };

        let expected_destination = DestinationRepository {
            owner: "my_name".to_string(),
            name: "test2".to_string(),
        };

        let transformation_args = json!({
            "before": "".to_string(),
            "after": "my_folder".to_string(),
        });
        let transformation = json!({
            "fn": "builtin.move",
            "args": transformation_args,
        });
        let config = Config {
            version: "0.0.1".to_string(),
            source: expected_source.clone(),
            destinations: vec![expected_destination.clone()],
            token: "random_token".to_string(),
            origin_files: Some("glob(\"**\", \"readme\")".to_string()),
            destination_files: Some("glob(\"my_folder/**\", \"my_folder/dist/**\")".to_string()),
            transformations: Some(vec![transformation]),
        };

        let parsed_config = parse_config(config.clone());

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
            origin_files: create_glob_single_with_exclude("**", "readme"),
            destination_files: create_glob_single_with_exclude("my_folder/**", "my_folder/dist/**"),
            transformations: Some(vec![expected_transformation]),
        };

        assert_eq!(parsed_config, expected_config)
    }

    #[test]
    fn success_workdir_when_none() {
        let expected_source = SourceRepository {
            owner: "my_name".to_string(),
            name: "test1".to_string(),
            git_ref: "main".to_string(),
        };

        let expected_destination = DestinationRepository {
            owner: "my_name".to_string(),
            name: "test2".to_string(),
        };

        let transformation_args = json!({
            "before": "",
            "after": "my_folder",
        });
        let transformation = json!({
            "fn": "builtin.move",
            "args": transformation_args,
        });

        let config = Config {
            version: "0.0.1".to_string(),
            source: expected_source.clone(),
            destinations: vec![expected_destination.clone()],
            token: "random_token".to_string(),
            origin_files: None,
            destination_files: None,
            transformations: Some(vec![transformation]),
        };

        let parsed_config = parse_config(config.clone());

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
            origin_files: WorkDirExpression::Path("".to_string()),
            destination_files: WorkDirExpression::Path("".to_string()),
            transformations: Some(vec![expected_transformation]),
        };

        assert_eq!(parsed_config, expected_config)
    }

    #[test]
    fn success_workdir_path() {
        let expected_source = SourceRepository {
            owner: "my_name".to_string(),
            name: "test1".to_string(),
            git_ref: "main".to_string(),
        };

        let expected_destination = DestinationRepository {
            owner: "my_name".to_string(),
            name: "test2".to_string(),
        };

        let transformation_args = json!({
            "before": "".to_string(),
            "after": "my_folder".to_string(),
        });
        let transformation = json!({
            "fn": "builtin.move",
            "args": transformation_args,
        });
        let config = Config {
            version: "0.0.1".to_string(),
            source: expected_source.clone(),
            destinations: vec![expected_destination.clone()],
            token: "random_token".to_string(),
            origin_files: Some("path1".to_string()),
            destination_files: Some("path2".to_string()),
            transformations: Some(vec![transformation]),
        };

        let parsed_config = parse_config(config.clone());

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
            origin_files: WorkDirExpression::Path("path1".to_string()),
            destination_files: WorkDirExpression::Path("path2".to_string()),
            transformations: Some(vec![expected_transformation]),
        };

        assert_eq!(parsed_config, expected_config)
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

        let config = read_config(&doc).unwrap();

        parse_config(config);
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

        let config = read_config(&doc).unwrap();

        parse_config(config);
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

        let config = read_config(&doc).unwrap();

        parse_config(config);
    }

    mod transformations_replace {
        use regex::Regex;

        use crate::cli::parser::CustomRegex;

        use super::*;
        #[test]
        fn success() {
            let expected_source = SourceRepository {
                owner: "my_name".to_string(),
                name: "test1".to_string(),
                git_ref: "main".to_string(),
            };

            let expected_destination = DestinationRepository {
                owner: "my_name".to_string(),
                name: "test2".to_string(),
            };

            let transformation_args = json!({
                "before": "kek".to_string(),
                "after": "lol".to_string(),
            });
            let transformation = json!({
                "fn": "builtin.replace",
                "args": transformation_args,
            });
            let config = Config {
                version: "0.0.1".to_string(),
                source: expected_source.clone(),
                destinations: vec![expected_destination.clone()],
                token: "random_token".to_string(),
                origin_files: Some("path1".to_string()),
                destination_files: Some("path2".to_string()),
                transformations: Some(vec![transformation]),
            };

            let parsed_config = parse_config(config.clone());

            let expected_transformation_args = ReplaceArgs {
                before: CustomRegex(Regex::new("kek").unwrap()),
                after: "lol".to_string(),
            };
            let expected_transformation = Transformation::Replace {
                args: expected_transformation_args,
            };
            let expected_config = ParsedConfig {
                version: "0.0.1".to_string(),
                source: expected_source,
                destinations: vec![expected_destination],
                token: "random_token".to_string(),
                origin_files: WorkDirExpression::Path("path1".to_string()),
                destination_files: WorkDirExpression::Path("path2".to_string()),
                transformations: Some(vec![expected_transformation]),
            };

            assert_eq!(parsed_config, expected_config)
        }
    }
}
