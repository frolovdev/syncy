use crate::mocks::github::get_content_mock;
use std::vec;
use syncy::fixtures::content::get_content_json;
use syncy::git_tree;

use glob;
use serde_json::json;
use syncy::{
    cli::{
        DestinationRepository, GlobExpression, ParsedConfig, SourceRepository, WorkDirExpression,
    },
    github_provider::GithubProvider,
    provider::Provider,
};
use wiremock::MockServer;

use syncy::cli::{MoveArgs, Transformation};

mod create_source_tree {

    use syncy::fixtures::workdir_path::{create_glob_single, create_workdir_path};

    use super::*;

    #[tokio::test]
    async fn success() {
        let config = ParsedConfig {
            version: "0.1".to_string(),
            source: SourceRepository {
                owner: "owner".to_string(),
                name: "repo1".to_string(),
                git_ref: "main".to_string(),
            },
            destinations: vec![DestinationRepository {
                owner: "owner".to_string(),
                name: "repo2".to_string(),
            }],
            token: "random_token".to_string(),
            destination_files: create_workdir_path(""),
            origin_files: create_workdir_path(""),
            transformations: None,
        };

        let mock_server = MockServer::start().await;

        let content1 = get_content_json("test1", "test1", Some("my_content"), "file");
        let content2 = get_content_json("test2", "test2", Some("my_content"), "file");
        let content_item_response = json!([&content1, &content2]);

        get_content_mock(
            &config.source.owner,
            &config.source.name,
            None,
            content_item_response,
            None,
        )
        .mount(&mock_server)
        .await;

        get_content_mock(
            &config.source.owner,
            &config.source.name,
            Some("test1"),
            json!(content1),
            None,
        )
        .mount(&mock_server)
        .await;

        get_content_mock(
            &config.source.owner,
            &config.source.name,
            Some("test2"),
            json!(content2),
            None,
        )
        .mount(&mock_server)
        .await;

        let github_provider = GithubProvider { config };

        let instance = github_provider.configure_provider(Some(mock_server.uri()));

        let source_tree = github_provider.create_source_tree(instance.clone()).await;

        let expected_tree = git_tree::Tree::from([
            (
                "test1".to_string(),
                git_tree::Node {
                    path: content1.path.to_string(),
                    content: content1.decoded_content(),
                    git_url: content1.git_url,
                    sha: content1.sha,
                },
            ),
            (
                "test2".to_string(),
                git_tree::Node {
                    path: content2.path.to_string(),
                    content: content2.decoded_content(),
                    git_url: content2.git_url,
                    sha: content2.sha,
                },
            ),
        ]);
        assert_eq!(source_tree, expected_tree);

        mock_server.verify().await;
    }

    #[tokio::test]
    async fn complex_case() {
        let config = ParsedConfig {
            version: "0.1".to_string(),
            source: SourceRepository {
                owner: "owner".to_string(),
                name: "repo1".to_string(),
                git_ref: "main".to_string(),
            },
            destinations: vec![DestinationRepository {
                owner: "owner".to_string(),
                name: "repo2".to_string(),
            }],
            token: "random_token".to_string(),
            destination_files: create_glob_single("folder/**"),
            origin_files: create_glob_single("folder/**"),
            transformations: Some(vec![Transformation::Move {
                args: MoveArgs {
                    before: "".to_string(),
                    after: "repo_one_folder".to_string(),
                },
            }]),
        };

        let mock_server = MockServer::start().await;

        let source_file_root = get_content_json("test1", "test1", Some("my_content"), "file");
        let source_content_folder = get_content_json("folder", "folder", None, "dir");
        let source_content_file =
            get_content_json("folder/test2", "test2", Some("my_content_2"), "file");
        let source_content_file_second =
            get_content_json("folder/test3", "test3", Some("my_content_3"), "file");

        let content_item_response_first = json!([&source_file_root, &source_content_folder]);

        get_content_mock(
            &config.source.owner,
            &config.source.name,
            None,
            content_item_response_first,
            None,
        )
        .mount(&mock_server)
        .await;

        get_content_mock(
            &config.source.owner,
            &config.source.name,
            Some("test1"),
            json!(source_file_root),
            None,
        )
        .mount(&mock_server)
        .await;

        get_content_mock(
            &config.source.owner,
            &config.source.name,
            Some("folder"),
            json!([source_content_file, source_content_file_second]),
            None,
        )
        .mount(&mock_server)
        .await;

        get_content_mock(
            &config.source.owner,
            &config.source.name,
            Some("folder/test2"),
            json!(source_content_file),
            None,
        )
        .mount(&mock_server)
        .await;
        get_content_mock(
            &config.source.owner,
            &config.source.name,
            Some("folder/test3"),
            json!(source_content_file_second),
            None,
        )
        .mount(&mock_server)
        .await;

        let github_provider = GithubProvider { config };

        let instance = github_provider.configure_provider(Some(mock_server.uri()));

        let source_tree = github_provider.create_source_tree(instance.clone()).await;

        let expected_tree = git_tree::Tree::from([
            (
                "repo_one_folder/folder/test2".to_string(),
                git_tree::Node {
                    path: source_content_file.path.to_string(),
                    content: source_content_file.decoded_content(),
                    git_url: source_content_file.git_url,
                    sha: source_content_file.sha,
                },
            ),
            (
                "repo_one_folder/folder/test3".to_string(),
                git_tree::Node {
                    path: source_content_file_second.path.to_string(),
                    content: source_content_file_second.decoded_content(),
                    git_url: source_content_file_second.git_url,
                    sha: source_content_file_second.sha,
                },
            ),
        ]);
        assert_eq!(source_tree, expected_tree);

        mock_server.verify().await;
    }
}
