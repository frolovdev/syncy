use std::vec;

use anyhow::Result;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};
use serde_yaml;

use super::common::{DestinationRepository, MoveArgs, SourceRepository, Transformation};

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Config {
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

        use crate::cli::common::{MoveArgs, Transformation};

        use super::super::{read_config, Config, DestinationRepository, SourceRepository};
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

            let expected_transformation_args = MoveArgs {
                before: "".to_string(),
                after: "my_folder".to_string(),
            };
            let expected_transformation = Transformation::Move {
                args: expected_transformation_args,
            };
            let expected_config = Config {
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

            read_config(&doc);
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

            read_config(&doc);
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

            read_config(&doc);
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

            let expected_transformation_args = MoveArgs {
                before: "".to_string(),
                after: "my_folder".to_string(),
            };
            let expected_transformation = Transformation::Move {
                args: expected_transformation_args,
            };
            let expected_config = Config {
                version: "0.0.1".to_string(),
                source: expected_source,
                destinations: vec![expected_destination],
                token: "random_token".to_string(),
                origin_files: Some("path".to_string()),
                destination_files: Some("another_path".to_string()),
                transformations: Some(vec![expected_transformation]),
            };

            assert_eq!(parsed_config, expected_config);
        }
    }
}
