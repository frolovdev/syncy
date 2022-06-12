use syncy::{
    event::Event,
    fixtures::{content::get_content_json, workdir_path::create_glob_single},
};

use crate::mocks::github::get_content_mock;

use glob;
use serde_json::json;
use syncy::{
    cli::{DestinationRepository, GlobExpression, ParsedConfig, SourceRepository},
    github_provider::GithubProvider,
    provider::Provider,
};
use wiremock::MockServer;

use syncy::cli::{
    common::{MoveArgs, Transformation},
    parser::WorkDirExpression,
};
use syncy::git_tree::GitTree;

#[tokio::test]
async fn generate_events_success() {
    let destination_repository = DestinationRepository {
        owner: "owner".to_string(),
        name: "repo2".to_string(),
    };
    let config = ParsedConfig {
        version: "0.1".to_string(),
        source: SourceRepository {
            owner: "owner".to_string(),
            name: "repo1".to_string(),
            git_ref: "main".to_string(),
        },
        destinations: vec![destination_repository.clone()],
        token: "random_token".to_string(),
        destination_files: create_glob_single("repo_one_folder/**"),
        origin_files: create_glob_single("folder/**"),
        transformations: Some(vec![Transformation::Move {
            args: MoveArgs {
                before: "".to_string(),
                after: "repo_one_folder".to_string(),
            },
        }]),
    };

    let mock_server = MockServer::start().await;

    let source_file_root = get_content_json("test1", "test1", Some("source_my_content"), "file");
    let source_content_folder = get_content_json("folder", "folder", None, "dir");
    let source_content_file =
        get_content_json("folder/test2", "test2", Some("source_my_content_2"), "file");
    let source_content_file_second =
        get_content_json("folder/test3", "test3", Some("source_my_content_3"), "file");

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

    let destination_root_folder =
        get_content_json("repo_one_folder", "repo_one_folder", None, "dir");
    let destination_file_root = get_content_json(
        "repo_one_folder/test1",
        "test1",
        Some("dest_my_content"),
        "file",
    );
    let destination_content_folder =
        get_content_json("repo_one_folder/folder", "folder", None, "dir");
    let destination_content_file = get_content_json(
        "repo_one_folder/folder/test2",
        "test2",
        Some("dest_my_content_2"),
        "file",
    );
    let destination_content_file_second = get_content_json(
        "repo_one_folder/folder/test4",
        "test4",
        Some("dest_my_content_3"),
        "file",
    );

    get_content_mock(
        &destination_repository.owner,
        &destination_repository.name,
        None,
        json!(&destination_root_folder),
        Some("main"),
    )
    .mount(&mock_server)
    .await;

    get_content_mock(
        &destination_repository.owner,
        &destination_repository.name,
        Some("repo_one_folder"),
        json!([&destination_file_root, &destination_content_folder]),
        Some("main"),
    )
    .mount(&mock_server)
    .await;

    get_content_mock(
        &destination_repository.owner,
        &destination_repository.name,
        Some("repo_one_folder/test1"),
        json!([&destination_file_root]),
        Some("main"),
    )
    .mount(&mock_server)
    .await;

    get_content_mock(
        &destination_repository.owner,
        &destination_repository.name,
        Some("repo_one_folder/folder"),
        json!([&destination_content_file, &destination_content_file_second]),
        Some("main"),
    )
    .mount(&mock_server)
    .await;

    get_content_mock(
        &destination_repository.owner,
        &destination_repository.name,
        Some("repo_one_folder/folder/test2"),
        json!([&destination_content_file]),
        Some("main"),
    )
    .mount(&mock_server)
    .await;
    get_content_mock(
        &destination_repository.owner,
        &destination_repository.name,
        Some("repo_one_folder/folder/test4"),
        json!([&destination_content_file_second]),
        Some("main"),
    )
    .mount(&mock_server)
    .await;

    let github_provider = GithubProvider { config };

    let instance = github_provider.configure_provider(Some(mock_server.uri()));

    let source_tree = github_provider.create_source_tree(instance.clone()).await;

    let dest_tree = github_provider
        .create_destination_tree(instance.clone(), &destination_repository)
        .await;

    let mut events = source_tree.generate_events(&dest_tree);
    events.sort();

    let expected_events = vec![
        Event::Create {
            path: "repo_one_folder/folder/test3".to_string(),
            content: source_content_file_second.decoded_content(),
        },
        Event::Update {
            path: "repo_one_folder/folder/test2".to_string(),
            content: source_content_file.decoded_content(),
            sha: destination_content_file.sha,
        },
        Event::Delete {
            path: "repo_one_folder/folder/test4".to_string(),
            sha: destination_content_file_second.sha,
        },
        Event::Delete {
            path: "repo_one_folder/test1".to_string(),
            sha: destination_file_root.sha,
        },
    ];
    assert_eq!(events, expected_events);

    mock_server.verify().await;
}
