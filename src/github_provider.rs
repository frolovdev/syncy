use async_recursion::async_recursion;
use octocrab::models::repos::{Content, ContentItems};
use octocrab::{models, params::repos::Reference, Octocrab};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::cli::{DestinationRepository, EnhancedParsedConfig, GlobExpression};
use crate::git_tree;

pub async fn call(config: EnhancedParsedConfig) {
    let octacrab_builder = octocrab::Octocrab::builder().personal_token(config.token);

    octocrab::initialise(octacrab_builder).unwrap();

    let instance: Arc<octocrab::Octocrab> = octocrab::instance();

    let main_path = "".to_string();

    let source_repo_content = get_repo(
        &instance,
        &config.source.owner,
        &config.source.name,
        &config.source.git_ref,
        &main_path,
    )
    .await
    .unwrap();

    let mut tree = git_tree::Tree::new();
    into_nodes_from_content_items(
        &instance,
        &config.source.owner,
        &config.source.name,
        &config.source.git_ref,
        &source_repo_content,
        &mut tree,
    )
    .await;

    let source_branch = get_branch(
        &instance,
        &config.source.owner,
        &config.source.name,
        &config.source.git_ref,
    )
    .await
    .unwrap();

    let last_ref_commit_from_source = get_sha(&source_branch.object).unwrap();

    let destination_branch_name = get_destination_branch_name(
        &config.source.owner,
        &config.source.name,
        &last_ref_commit_from_source,
    );

    let val = instance.clone();
    let destinations = Arc::new(config.destinations);
    update_destinations(
        val,
        destinations,
        destination_branch_name,
        tree,
        config.origin_files,
        config.destination_files,
        &main_path,
    )
    .await
}

pub async fn into_nodes_from_content_items<'a>(
    instance: &Arc<Octocrab>,
    owner: &str,
    repo: &str,
    git_ref: &str,
    content_items: &ContentItems,
    tree: &mut git_tree::Tree,
) {
    for x in content_items.items.iter() {
        match x {
            Content { r#type, path, .. } => {
                let file_type = "file";
                let folder_type = "dir";
                if r#type == file_type {
                    unwrap_file(&instance, &path, &owner, &repo, &git_ref, tree).await;
                } else if r#type == folder_type {
                    unwrap_folder(&instance, &owner, &repo, &git_ref, &x, tree).await;
                } else {
                    panic!("unexpected content type")
                }
            }
        };
    }
}

async fn unwrap_file(
    instance: &Arc<Octocrab>,
    file_path: &str,
    owner: &str,
    repo: &str,
    git_ref: &str,
    tree: &mut git_tree::Tree,
) -> () {
    let content_items = get_repo(&instance, &owner, &repo, &git_ref, &file_path.to_string())
        .await
        .unwrap();

    let content = content_items.items.first().unwrap();
    let decoded_content = content.decoded_content();

    let created_node = git_tree::Node {
        path: file_path.to_string(),
        content: decoded_content,
        git_url: content.git_url.clone(),
    };
    tree.0.insert(file_path.to_string(), created_node);
}

#[async_recursion()]
async fn unwrap_folder(
    instance: &Arc<Octocrab>,
    owner: &str,
    repo: &str,
    git_ref: &str,
    content: &Content,
    tree: &mut git_tree::Tree,
) {
    let content_items = get_repo(&instance, &owner, &repo, &git_ref, &content.path)
        .await
        .unwrap();

    into_nodes_from_content_items(instance, &owner, &repo, &git_ref, &content_items, tree).await;
}

async fn get_repo(
    octocrab: &Arc<Octocrab>,
    owner: &str,
    repo: &str,
    git_ref: &str,
    path: &str,
) -> Result<octocrab::models::repos::ContentItems, octocrab::Error> {
    octocrab
        .repos(owner, repo)
        .get_content()
        .path(path)
        .r#ref(git_ref)
        .send()
        .await
}

async fn get_branch(
    octocrab: &Arc<Octocrab>,
    owner: &str,
    repo: &str,
    git_ref: &str,
) -> Result<octocrab::models::repos::Ref, octocrab::Error> {
    octocrab
        .repos(owner, repo)
        .get_ref(&Reference::Branch(git_ref.to_string()))
        .await
}

async fn create_branch(
    octocrab: &Arc<Octocrab>,
    destination_owner: &str,
    destination_repo: &str,
    destination_branch_name: &str,
    source_commit_ref: &str,
) -> Result<octocrab::models::repos::Ref, octocrab::Error> {
    octocrab
        .repos(destination_owner, destination_repo)
        .create_ref(
            &Reference::Branch(destination_branch_name.to_string()),
            source_commit_ref,
        )
        .await
}

fn get_destination_branch_name(owner: &str, repo: &str, source_commit_ref: &str) -> String {
    format!(
        "syncy/{owner}/{repo}/{source_commit_ref}",
        owner = owner,
        repo = repo,
        source_commit_ref = source_commit_ref
    )
}

fn get_pull_request_name() {
    // Update from {owner}/{repo} branch: ${branch} ref: 4da4b22ac75d363d168ce109d51c80921cacebcb
}

fn get_sha(object: &models::repos::Object) -> Option<String> {
    match object {
        models::repos::Object::Commit { sha, .. } => Some(sha.to_string()),
        _ => None,
    }
}

async fn update_destinations(
    octocrab: Arc<Octocrab>,
    destinations: Arc<Vec<DestinationRepository>>,
    destination_branch_name: String,
    tree: git_tree::Tree,
    origin_files: Option<GlobExpression>,
    _destination_files: Option<GlobExpression>,
    root_path: &str,
) {
    let transformed_tree = transform_tree(tree, &origin_files, root_path);

    for destination in destinations.iter() {
        let main_ref = "main";
        let destination_main =
            get_branch(&octocrab, &destination.owner, &destination.name, &main_ref)
                .await
                .unwrap();

        let commit_ref = get_sha(&destination_main.object).unwrap();

        create_branch(
            &octocrab,
            &destination.owner,
            &destination.name,
            &destination_branch_name,
            &commit_ref,
        )
        .await
        .unwrap();

        for (path, node) in &transformed_tree.0 {
            if let Some(content_value) = &node.content {
                create_file(
                    &octocrab,
                    &destination.owner,
                    &destination.name,
                    &path,
                    &content_value,
                    &destination_branch_name,
                )
                .await;
            }
        }
    }
}

fn transform_tree(
    git_tree: git_tree::Tree,
    origin_files: &Option<GlobExpression>,
    main_path: &str,
) -> git_tree::Tree {
    let unwrapped_origin = origin_files.as_ref().unwrap();

    let mut new_tree = git_tree::Tree::new();
    for (key, node) in git_tree.0 {
        match unwrapped_origin {
            GlobExpression::Single(pattern) => {
                if pattern.matches(&key) {
                    let new_val =
                        key.trim_start_matches(&format!("{main_path}/", main_path = main_path));

                    new_tree.0.insert(new_val.to_string(), node);
                }
            }
            GlobExpression::SingleWithExclude(include_pattern, exclude_pattern) => {
                if include_pattern.matches(&key) && !exclude_pattern.matches(&key) {
                    let new_val =
                        key.trim_start_matches(&format!("{main_path}/", main_path = main_path));

                    new_tree.0.insert(new_val.to_string(), node);
                }
            }
        }
    }

    new_tree
}

#[derive(Debug, Serialize)]
struct CreateFileBody {
    message: String,
    // committer: Committer,
    content: String,
    branch: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct CreateFileResponse {
    content: octocrab::models::repos::Content,
}

async fn create_file(
    octocrab: &Arc<Octocrab>,
    owner: &str,
    repo: &str,
    file_name: &str,
    content: &str,
    branch: &str,
) -> CreateFileResponse {
    let encoded_content = base64::encode(content);

    let body = CreateFileBody {
        message: file_name.to_string(),
        content: encoded_content,
        branch: branch.to_string(),
    };

    let route = format!(
        "/repos/{owner}/{repo}/contents/{file_name}",
        owner = owner,
        repo = repo,
        file_name = file_name
    );

    octocrab
        .put::<CreateFileResponse, _, _>(route, Some(&body))
        .await
        .unwrap()
}
