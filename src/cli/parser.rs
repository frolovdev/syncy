use regex::RegexSet;

use super::{
    common::{DestinationRepository, SourceRepository, Transformation},
    reader,
};

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedConfig {
    pub version: String,
    pub source: SourceRepository,
    pub destinations: Vec<DestinationRepository>,
    pub token: String,
    pub destination_files: Option<WorkDirExpression>,
    pub origin_files: Option<WorkDirExpression>,
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
        destination_files: Some(destination_files_glob),
        origin_files: Some(origin_files_glob),
        transformations: config.transformations,
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
    use super::{parse_config, GlobExpression, ParsedConfig, WorkDirExpression};
    use crate::fixtures::globs::create_glob_single;
    use crate::{
        cli::{
            common::{DestinationRepository, MoveArgs, SourceRepository, Transformation},
            reader::Config,
        },
        fixtures::globs::create_glob_single_with_exclude,
    };

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

        let expected_transformation_args = MoveArgs {
            before: "".to_string(),
            after: "my_folder".to_string(),
        };
        let expected_transformation = Transformation::Move {
            args: expected_transformation_args,
        };
        let config = Config {
            version: "0.0.1".to_string(),
            source: expected_source.clone(),
            destinations: vec![expected_destination.clone()],
            token: "random_token".to_string(),
            origin_files: Some("glob(\"**\")".to_string()),
            destination_files: Some("glob(\"my_folder/**\")".to_string()),
            transformations: Some(vec![expected_transformation.clone()]),
        };

        let parsed_config = parse_config(config.clone());

        let expected_config = ParsedConfig {
            version: "0.0.1".to_string(),
            source: expected_source,
            destinations: vec![expected_destination],
            token: "random_token".to_string(),
            origin_files: Some(create_glob_single("**")),
            destination_files: Some(create_glob_single("my_folder/**")),
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

        let expected_transformation_args = MoveArgs {
            before: "".to_string(),
            after: "my_folder".to_string(),
        };
        let expected_transformation = Transformation::Move {
            args: expected_transformation_args,
        };
        let config = Config {
            version: "0.0.1".to_string(),
            source: expected_source.clone(),
            destinations: vec![expected_destination.clone()],
            token: "random_token".to_string(),
            origin_files: Some("glob(\"**\", \"readme\")".to_string()),
            destination_files: Some("glob(\"my_folder/**\", \"my_folder/dist/**\")".to_string()),
            transformations: Some(vec![expected_transformation.clone()]),
        };

        let parsed_config = parse_config(config.clone());

        let expected_config = ParsedConfig {
            version: "0.0.1".to_string(),
            source: expected_source,
            destinations: vec![expected_destination],
            token: "random_token".to_string(),
            origin_files: Some(create_glob_single_with_exclude("**", "readme")),
            destination_files: Some(create_glob_single_with_exclude(
                "my_folder/**",
                "my_folder/dist/**",
            )),
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

        let expected_transformation_args = MoveArgs {
            before: "".to_string(),
            after: "my_folder".to_string(),
        };
        let expected_transformation = Transformation::Move {
            args: expected_transformation_args,
        };
        let config = Config {
            version: "0.0.1".to_string(),
            source: expected_source.clone(),
            destinations: vec![expected_destination.clone()],
            token: "random_token".to_string(),
            origin_files: None,
            destination_files: None,
            transformations: Some(vec![expected_transformation.clone()]),
        };

        let parsed_config = parse_config(config.clone());

        let expected_config = ParsedConfig {
            version: "0.0.1".to_string(),
            source: expected_source,
            destinations: vec![expected_destination],
            token: "random_token".to_string(),
            origin_files: Some(WorkDirExpression::Path("".to_string())),
            destination_files: Some(WorkDirExpression::Path("".to_string())),
            transformations: Some(vec![expected_transformation]),
        };

        assert_eq!(parsed_config, expected_config)
    }

    #[test]
    fn success_workdir_path() {}
}
