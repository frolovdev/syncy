use anyhow::Result;
use serde::Deserialize;
use serde_yaml;

use super::common::{DestinationRepository, SourceRepository};

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Config {
    pub version: String,
    pub source: SourceRepository,
    pub destinations: Vec<DestinationRepository>,
    pub token: String,
    pub destination_files: Option<String>,
    pub origin_files: Option<String>,
    pub transformations: Option<Vec<serde_json::Value>>,
    pub update_fns: Option<Vec<serde_json::Value>>,
}

pub fn read_config(config: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let deserialized_config: serde_yaml::Result<Config> = serde_yaml::from_str(&config);

    let result = match deserialized_config {
        Ok(content) => content,
        Err(error) => {
            return Err(error.into());
        }
    };

    Ok(result)
}

#[cfg(test)]
mod tests {

    mod reader {

        use super::super::{read_config, Config, DestinationRepository, SourceRepository};
        use indoc::indoc;
        use serde_json::json;

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

            let parsed_config = read_config(&doc).unwrap();

            let expected_source = SourceRepository {
                owner: "my_name".to_string(),
                name: "test1".to_string(),
                git_ref: "main".to_string(),
            };

            let expected_destination = DestinationRepository {
                owner: "my_name".to_string(),
                name: "test2".to_string(),
            };

            let expected_transformation_args = json!({
                "before": "",
                "after": "my_folder",
            });
            let expected_transformation = json!({
                "fn": "builtin.move",
                "args": expected_transformation_args,
            });
            let expected_config = Config {
                version: "0.0.1".to_string(),
                source: expected_source,
                destinations: vec![expected_destination],
                token: "random_token".to_string(),
                origin_files: Some("glob(\"**\")".to_string()),
                destination_files: Some("glob(\"my_folder/**\")".to_string()),
                transformations: Some(vec![expected_transformation]),
                update_fns: None,
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

            kekos:
                - fn: builtin.move
                  args:
                    before: ''
                    after: my_code

            "#};

            let parsed_config = read_config(&doc).unwrap();

            let expected_source = SourceRepository {
                owner: "my_name".to_string(),
                name: "test1".to_string(),
                git_ref: "main".to_string(),
            };

            let expected_destination = DestinationRepository {
                owner: "my_name".to_string(),
                name: "test2".to_string(),
            };

            let expected_config = Config {
                version: "0.0.1".to_string(),
                source: expected_source,
                destinations: vec![expected_destination],
                token: "random_token".to_string(),
                origin_files: Some("glob(\"**\")".to_string()),
                destination_files: Some("glob(\"my_folder/**\")".to_string()),
                transformations: None,
                update_fns: None,
            };

            assert_eq!(parsed_config, expected_config);
        }

        #[test]
        fn work_dir_path() {
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
            
            origin_files: path
            
            destination_files: another_path
            
            transformations:
              - fn: builtin.move
                args:
                  before: ""
                  after: my_folder  
            "#};

            let parsed_config = read_config(&doc).unwrap();

            let expected_source = SourceRepository {
                owner: "my_name".to_string(),
                name: "test1".to_string(),
                git_ref: "main".to_string(),
            };

            let expected_destination = DestinationRepository {
                owner: "my_name".to_string(),
                name: "test2".to_string(),
            };

            let expected_transformation_args = json!({
                "before": "",
                "after": "my_folder",
            });
            let expected_transformation = json!({
                "fn": "builtin.move",
                "args": expected_transformation_args,
            });
            let expected_config = Config {
                version: "0.0.1".to_string(),
                source: expected_source,
                destinations: vec![expected_destination],
                token: "random_token".to_string(),
                origin_files: Some("path".to_string()),
                destination_files: Some("another_path".to_string()),
                transformations: Some(vec![expected_transformation]),
                update_fns: None,
            };

            assert_eq!(parsed_config, expected_config);
        }
    }
}
