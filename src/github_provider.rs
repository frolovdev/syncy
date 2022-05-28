use async_recursion::async_recursion;
use core::panic;
use git_tree::GitTree;
use octocrab::models::repos::{Commit, Content, ContentItems};
use octocrab::{models, params::repos::Reference, Octocrab};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cli::{EnhancedParsedConfig};
use crate::event::Event;
use crate::git_tree;

pub async fn call(config: EnhancedParsedConfig) {
    let octacrab_builder = octocrab::Octocrab::builder().personal_token(config.token.clone());

    octocrab::initialise(octacrab_builder).unwrap();

    let instance: Arc<octocrab::Octocrab> = octocrab::instance();

    let root_path = "".to_string();

    let source_repo_content = get_repo(
        &instance,
        &config.source.owner,
        &config.source.name,
        &config.source.git_ref,
        &root_path,
    )
    .await
    .unwrap();

    let mut tree = git_tree::Tree::new();

    fill_tree_with_nodes(
        &instance,
        &config.source.owner,
        &config.source.name,
        &config.source.git_ref,
        &source_repo_content,
        &mut tree,
    )
    .await;

    update_destinations(&instance, config, tree, &root_path).await
}

pub async fn fill_tree_with_nodes<'a>(
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
        sha: content.sha.clone(),
    };
    tree.insert(file_path.to_string(), created_node);
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

    fill_tree_with_nodes(instance, &owner, &repo, &git_ref, &content_items, tree).await;
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

fn get_destination_branch_name(owner: &str, repo: &str) -> String {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    let in_ms = since_the_epoch.as_millis();

    format!(
        "syncy/{owner}/{repo}/{timestamp}",
        owner = owner,
        repo = repo,
        timestamp = in_ms
    )
}

fn get_sha(object: &models::repos::Object) -> Option<String> {
    match object {
        models::repos::Object::Commit { sha, .. } => Some(sha.to_string()),
        _ => None,
    }
}

async fn update_destinations(
    octocrab: &Arc<Octocrab>,
    config: EnhancedParsedConfig,
    tree: git_tree::Tree,
    root_path: &str,
) {
    let destination_branch_name =
        get_destination_branch_name(&config.source.owner, &config.source.name);

    let transformed_source_tree = tree.transform_tree(&config.origin_files, root_path);
    let transformed_source_tree_with_applied_transformations =
        transformed_source_tree.apply_transformations(&config.transformations);

    for destination in config.destinations.iter() {
        let main_ref = "main";
        let source_repo_content = get_repo(
            &octocrab,
            &destination.owner,
            &destination.name,
            &main_ref,
            &root_path,
        )
        .await
        .unwrap();

        let mut destination_tree = git_tree::Tree::new();
        fill_tree_with_nodes(
            &octocrab,
            &destination.owner,
            &destination.name,
            &main_ref,
            &source_repo_content,
            &mut destination_tree,
        )
        .await;

        let transformed_destination_tree =
            destination_tree.transform_tree(&config.destination_files, root_path);

        let events = transformed_source_tree_with_applied_transformations
            .generate_events(&transformed_destination_tree);

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

        for event in events.iter() {
            match &event {
                Event::Create { path, content } => {
                    create_file(
                        &octocrab,
                        &destination.owner,
                        &destination.name,
                        &path,
                        content.as_ref(),
                        &destination_branch_name,
                    )
                    .await;
                }
                Event::Update { path, content, sha } => {
                    update_file(
                        &octocrab,
                        &destination.owner,
                        &destination.name,
                        path,
                        content.as_ref(),
                        sha,
                        &destination_branch_name,
                    )
                    .await;
                }
                Event::Delete { path, sha } => {
                    delete_file(
                        &octocrab,
                        &destination.owner,
                        &destination.name,
                        &path,
                        &sha,
                        &destination_branch_name,
                    )
                    .await;
                }
            };
        }
        create_pull_request(
            &octocrab,
            &destination.owner,
            &destination.name,
            &config.source.owner,
            &config.source.name,
            &config.source.git_ref,
            &destination_branch_name,
            &main_ref,
        )
        .await
        .unwrap();
    }
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
    path: &str,
    content: Option<&String>,
    branch: &str,
) -> CreateFileResponse {
    let mapped_content = match content {
        Some(value) => value,
        None => "",
    };

    let encoded_content = base64::encode(mapped_content);

    let body = CreateFileBody {
        message: path.to_string(),
        content: encoded_content,
        branch: branch.to_string(),
    };

    let route = format!(
        "/repos/{owner}/{repo}/contents/{path}",
        owner = owner,
        repo = repo,
        path = path
    );

    octocrab
        .put::<CreateFileResponse, _, _>(route, Some(&body))
        .await
        .unwrap()
}

#[derive(Debug, Serialize)]
struct DeleteFileBody {
    message: String,
    sha: String,
    branch: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct DeleteFileResponse {
    content: Option<String>,
    commit: Commit,
}

async fn delete_file(
    octocrab: &Arc<Octocrab>,
    owner: &str,
    repo: &str,
    path: &str,
    sha: &str,
    branch: &str,
) -> DeleteFileResponse {
    let route = format!(
        "/repos/{owner}/{repo}/contents/{path}",
        owner = owner,
        repo = repo,
        path = path
    );

    let body = DeleteFileBody {
        sha: sha.to_string(),
        message: path.to_string(),
        branch: branch.to_string(),
    };

    octocrab
        .delete::<DeleteFileResponse, _, _>(route, Some(&body))
        .await
        .unwrap()
}

#[derive(Debug, Serialize)]
struct UpdateFileBody {
    message: String,
    // committer: Committer,
    content: String,
    branch: String,
    sha: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct UpdateFileResponse {
    content: octocrab::models::repos::Content,
    commit: Commit,
}

async fn update_file(
    octocrab: &Arc<Octocrab>,
    owner: &str,
    repo: &str,
    path: &str,
    content: Option<&String>,
    sha: &str,
    branch: &str,
) {
    let mapped_content = match content {
        Some(value) => value,
        None => "",
    };

    let encoded_content = base64::encode(mapped_content);

    let body = UpdateFileBody {
        message: path.to_string(),
        content: encoded_content,
        branch: branch.to_string(),
        sha: sha.to_string(),
    };

    let route = format!(
        "/repos/{owner}/{repo}/contents/{path}",
        owner = owner,
        repo = repo,
        path = path
    );

    octocrab
        .put::<UpdateFileResponse, _, _>(route, Some(&body))
        .await
        .unwrap();
}

fn get_pull_request_name(owner: &str, repo: &str, source_branch: &str) -> String {
    format!(
        "Update from {owner}/{repo} branch: {branch}",
        owner = owner,
        repo = repo,
        branch = source_branch
    )
}

fn get_pull_request_body(owner: &str, repo: &str, source_branch: &str) -> String {
    let link = format!(
        "https://github.com/{owner}/{repo}/{branch}",
        owner = owner,
        repo = repo,
        branch = source_branch
    );
    format!(
        "Update from {owner}/{repo} branch: {branch}\n\nlink to the original repo: {link}",
        owner = owner,
        repo = repo,
        branch = source_branch,
        link = link
    )
}

async fn create_pull_request(
    octocrab: &Arc<Octocrab>,
    owner: &str,
    repo: &str,
    source_owner: &str,
    source_repo: &str,
    source_branch: &str,
    destination_branch_name: &str,
    base_ref: &str,
) -> Result<octocrab::models::pulls::PullRequest, octocrab::Error> {
    octocrab
        .pulls(owner, repo)
        .create(
            get_pull_request_name(source_owner, source_repo, source_branch),
            destination_branch_name,
            base_ref,
        )
        .body(get_pull_request_body(
            source_owner,
            source_repo,
            source_branch,
        ))
        .send()
        .await
}

#[cfg(test)]
mod tests {}
