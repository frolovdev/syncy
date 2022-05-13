use async_recursion::async_recursion;
use octocrab::models::repos::{Content, ContentItems, Ref};
use octocrab::{models, params::repos::Reference, Octocrab};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::cli::{DestinationRepository, EnhancedParsedConfig, GlobExpression};
use crate::git_tree;

pub async fn call(config: EnhancedParsedConfig) {
    let octacrab_builder = octocrab::Octocrab::builder().personal_token(config.token);

    octocrab::initialise(octacrab_builder).unwrap();

    let instance: Arc<octocrab::Octocrab> = octocrab::instance();

    let main_path = "folder".to_string();

    let source_repo_content = get_repo(
        &instance,
        &config.source.owner,
        &config.source.name,
        &config.source.git_ref,
        &main_path,
    )
    .await
    .unwrap();

    // println!("{:#?}", source_repo_content);

    let nodes = into_nodes_from_content_items(
        &instance,
        &config.source.owner,
        &config.source.name,
        &config.source.git_ref,
        &source_repo_content,
    )
    .await;

    let root = git_tree::Node::Root {
        path: Some(RefCell::new(main_path.clone())),
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

    update_destinations(
        &instance,
        &config.destinations,
        destination_branch_name,
        tree,
        config.origin_files,
        config.destination_files,
    )
    .await
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
        path: RefCell::new(file_path.to_string()),
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
        path: RefCell::new(content.path.clone()),
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
    destination_branch_name: String,
    tree: git_tree::Tree,
    origin_files: Option<GlobExpression>,
    destination_files: Option<GlobExpression>,
) {
    let transformed_tree = transform_tree(tree, &origin_files);

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

        transformed_tree.traverse(Box::new(|node| {
            Box::pin(async {
                if let git_tree::Node::File { path, content, .. } = node {
                    if let Some(content_value) = content.as_ref() {
                        println!("we are don {}, content: {}", &*path.borrow(), content_value);
                        let local_path = String::from(&*path.borrow());
                        let instance = octocrab.clone();

                        let local_content_value = String::from(content_value);
                        let local_owner = String::from(&destination.owner);
                        let local_name = String::from(&destination.name);
                        let local_destination_branch_name = String::from(&destination_branch_name);

                        create_file(
                            &instance,
                            &local_owner,
                            &local_name,
                            &local_path,
                            &local_content_value,
                            &local_destination_branch_name,
                        )
                        .await;
                    }
                };
            })
        }));
    }
}

fn transform_tree(
    git_tree: git_tree::Tree,
    origin_files: &Option<GlobExpression>,
) -> git_tree::Tree {
    let unwrapped_origin = origin_files.as_ref().unwrap();

    let root_path = match git_tree.root.as_ref() {
        git_tree::Node::Root { path, .. } => path,
        git_tree::Node::File { .. } | git_tree::Node::Folder { .. } => {
            panic!("Root can't be not root")
        }
    };

    git_tree.apply_transformation(|node| match node {
        git_tree::Node::Root { .. } => true,
        git_tree::Node::File { path, .. } | git_tree::Node::Folder { path, .. } => {
            let mut unwrapped_path = path.borrow_mut();
            match unwrapped_origin {
                GlobExpression::Single(pattern) => {
                    if pattern.matches(&unwrapped_path) {
                        let new_val = unwrapped_path
                            .trim_start_matches(&*root_path.as_ref().unwrap().borrow());

                        *unwrapped_path = new_val.to_string();
                        return true;
                    }

                    false
                }
                GlobExpression::SingleWithExclude(include_pattern, exclude_pattern) => {
                    if include_pattern.matches(&unwrapped_path)
                        && !exclude_pattern.matches(&unwrapped_path)
                    {
                        let new_val = unwrapped_path
                            .trim_start_matches(&*root_path.as_ref().unwrap().borrow());

                        *unwrapped_path = new_val.to_string();
                        return true;
                    }
                    false
                }
            }
        }
    })
}

// #[derive(Debug, Serialize)]
// struct Committer {
//     name: String,
//     email: String,
// }

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

    println!("route: {}", &route);

    octocrab
        .put::<CreateFileResponse, _, _>(route, Some(&body))
        .await
        .unwrap()
}

fn delete_file() {
    todo!()
}
