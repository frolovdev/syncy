use async_recursion::async_recursion;
use octocrab::models::repos::{Content, ContentItems};
use octocrab::{models, params::repos::Reference, Octocrab};
use std::rc::Rc;
use std::sync::Arc;
use std::vec;

use crate::cli::{DestinationRepository, EnhancedParsedConfig, GlobExpression};
use crate::{git_tree, main};

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

    println!("{:#?}", source_repo_content);

    let nodes = into_nodes_from_content_items(
        &instance,
        &config.source.owner,
        &config.source.name,
        &config.source.git_ref,
        &source_repo_content,
    )
    .await;

    let root = git_tree::Node::Root {
        path: Some(main_path.clone()),
        children: nodes,
    };
    let tree = git_tree::Tree::new(root);

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

    // update_destinations(
    //     &instance,
    //     &config.destinations,
    //     &last_ref_commit_from_source,
    //     destination_branch_name,
    //     tree,
    //     config.origin_files,
    //     config.destination_files,
    // )
    // .await
}

pub async fn into_nodes_from_content_items<'a>(
    instance: &'a Arc<Octocrab>,
    owner: &'a str,
    repo: &'a str,
    git_ref: &'a str,
    content_items: &'a ContentItems,
) -> Vec<Rc<git_tree::Node>> {
    let mut node_vec = Vec::new();

    for x in content_items.items.iter() {
        let node = match x {
            Content { r#type, path, .. } => {
                let file_type = "file";
                let folder_type = "dir";
                if r#type == file_type {
                    unwrap_file(&instance, &path, &owner, &repo, &git_ref)
                        .await
                        .unwrap()
                } else if r#type == folder_type {
                    let my_node = unwrap_folder(&instance, &owner, &repo, &git_ref, &x)
                        .await
                        .unwrap();

                    Rc::new(my_node)
                } else {
                    panic!("unexpected content type")
                }
            }
        };
        node_vec.push(node);
    }

    node_vec
}

async fn unwrap_file<'a>(
    instance: &Arc<Octocrab>,
    file_path: &'a str,
    owner: &'a str,
    repo: &'a str,
    git_ref: &'a str,
) -> Result<Rc<git_tree::Node>, octocrab::Error> {
    let content_items = get_repo(&instance, &owner, &repo, &git_ref, &file_path.to_string())
        .await
        .unwrap();

    let content = content_items.items.first().unwrap();
    let decoded_content = content.decoded_content();

    Ok(Rc::new(git_tree::Node::File {
        path: file_path.to_string(),
        content: decoded_content,
        git_url: content.git_url.clone(),
    }))
}

#[async_recursion(?Send)]
async fn unwrap_folder<'a>(
    instance: &'a Arc<Octocrab>,
    owner: &'a str,
    repo: &'a str,
    git_ref: &'a str,
    content: &'a Content,
) -> Result<git_tree::Node, octocrab::Error> {
    let content_items = get_repo(&instance, &owner, &repo, &git_ref, &content.path)
        .await
        .unwrap();

    let nodes =
        into_nodes_from_content_items(instance, &owner, &repo, &git_ref, &content_items).await;

    let cur_node = git_tree::Node::Folder {
        path: content.path.clone(),
        children: nodes,
    };

    Ok(cur_node)
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
    octocrab: &Arc<Octocrab>,
    destinations: &Vec<DestinationRepository>,
    source_commit_ref: &str,
    destination_branch_name: String,
    tree: git_tree::Tree,
    origin_files: Option<GlobExpression>,
    destination_files: Option<GlobExpression>,
) {
    let transformed_tree = transform_tree(tree, &origin_files, &destination_files);
    for destination in destinations.iter() {
        let destination_branch = create_branch(
            &octocrab,
            &destination.owner,
            &destination.name,
            &destination_branch_name,
            &source_commit_ref,
        )
        .await
        .unwrap();
    }
}

fn transform_tree(
    git_tree: git_tree::Tree,
    origin_files: &Option<GlobExpression>,
    destination_files: &Option<GlobExpression>,
) -> git_tree::Tree {
    let unwrapped_origin = origin_files.as_ref().unwrap();
    git_tree.apply_transformation(|node| match node {
        git_tree::Node::Root { path, .. } => {
            let unwrapped_path = path.as_ref().unwrap();
            match unwrapped_origin {
                GlobExpression::Single(pattern) => pattern.matches(&unwrapped_path),
                GlobExpression::SingleWithExclude(include_pattern, exclude_pattern) => {
                    include_pattern.matches(&unwrapped_path)
                        && !exclude_pattern.matches(&unwrapped_path)
                }
            }
        }
        git_tree::Node::File { path, .. } | git_tree::Node::Folder { path, .. } => {
            match unwrapped_origin {
                GlobExpression::Single(pattern) => pattern.matches(path),
                GlobExpression::SingleWithExclude(include_pattern, exclude_pattern) => {
                    include_pattern.matches(path) && !exclude_pattern.matches(path)
                }
            }
        }
    })
}

fn create_file() {
    todo!()
}

fn update_file() {
    todo!()
}

fn delete_file() {
    todo!()
}
