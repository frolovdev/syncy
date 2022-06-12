use std::collections::HashMap;

use crate::{
    cli::{GlobExpression, Transformation, WorkDirExpression},
    event::Event,
};

#[derive(Debug, PartialEq)]
pub struct Node {
    pub path: String,
    pub content: Option<String>,
    pub git_url: String,
    pub sha: String,
}

pub type Tree = HashMap<String, Node>;

pub trait GitTree {
    fn transform_tree(self, origin_files_glob: &WorkDirExpression, root_path: &str) -> Tree;

    fn generate_events(&self, destination_tree: &Tree) -> Vec<Event>;

    fn apply_transformations(self, transformations: &Option<Vec<Transformation>>) -> Tree;
}

impl GitTree for HashMap<String, Node> {
    fn transform_tree(self, origin_files_glob: &WorkDirExpression, root_path: &str) -> Self {
        if let WorkDirExpression::Path(_) = origin_files_glob {
            return self;
        }

        let mut new_tree = Tree::new();
        for (key, node) in self {
            match origin_files_glob {
                WorkDirExpression::Glob(glob_expression) => match glob_expression {
                    GlobExpression::Single(pattern) => {
                        if pattern.matches(&key) {
                            let new_val = key.trim_start_matches(&format!(
                                "{root_path}/",
                                root_path = root_path
                            ));

                            new_tree.insert(new_val.to_string(), node);
                        }
                    }
                    GlobExpression::SingleWithExclude(include_pattern, exclude_pattern) => {
                        if include_pattern.matches(&key) && !exclude_pattern.matches(&key) {
                            let new_val = key.trim_start_matches(&format!(
                                "{root_path}/",
                                root_path = root_path
                            ));

                            new_tree.insert(new_val.to_string(), node);
                        }
                    }
                },
                _ => {}
            }
        }

        new_tree
    }

    fn generate_events(&self, destination_tree: &Tree) -> Vec<Event> {
        let mut events = Vec::new();
        for (source_key, source_node) in self.iter() {
            let destination_node = destination_tree.get(source_key);
            match destination_node {
                Some(destination_node) => events.push(Event::Update {
                    sha: destination_node.sha.to_string(),
                    path: source_key.to_string(),
                    content: source_node.content.clone(),
                }),
                None => events.push(Event::Create {
                    path: source_key.to_string(),
                    content: source_node.content.clone(),
                }),
            }
        }

        for (dest_key, dest_node) in destination_tree.iter() {
            if !self.contains_key(dest_key) {
                events.push(Event::Delete {
                    path: dest_key.to_string(),
                    sha: dest_node.sha.to_string(),
                })
            }
        }

        events
    }

    fn apply_transformations(self, transformations: &Option<Vec<Transformation>>) -> Tree {
        if let None = transformations {
            return self;
        }
        let mut new_tree = Tree::new();

        for (path, node) in self {
            let mut new_path = "".to_string();
            for t in transformations.as_ref().unwrap().iter() {
                match t {
                    Transformation::Move { args } => {
                        if path.starts_with(&args.before) {
                            let trimmed_before_val = path.trim_start_matches(&format!(
                                "{prefix_path}/",
                                prefix_path = args.before
                            ));

                            new_path = format!(
                                "{prefix}/{val}",
                                val = trimmed_before_val,
                                prefix = args.after
                            );
                        }
                    }
                };
            }

            new_tree.insert(new_path, node);
        }

        new_tree
    }
}

#[cfg(test)]
mod tests {

    use super::{GitTree, Node, Tree};
    use crate::fixtures::{content::get_content_json, workdir_path::create_glob_single};

    #[test]
    fn test_success() {
        let mut tree = Tree::new();

        tree.insert(
            "folder/file1".to_string(),
            Node {
                path: "folder/file1".to_string(),
                content: Some("".to_string()),
                git_url: "".to_string(),
                sha: "x132".to_string(),
            },
        );
        tree.insert(
            "folder/folder2/file2".to_string(),
            Node {
                path: "folder/folder2/file2".to_string(),
                content: Some("".to_string()),
                git_url: "".to_string(),
                sha: "x132".to_string(),
            },
        );
        tree.insert(
            "folder/file3".to_string(),
            Node {
                path: "folder/file3".to_string(),
                content: Some("".to_string()),
                git_url: "".to_string(),
                sha: "x132".to_string(),
            },
        );

        let glob = create_glob_single("folder/folder2/**");

        let new_tree = tree.transform_tree(&glob, "");

        let mut expected_tree = Tree::new();
        expected_tree.insert(
            "folder/folder2/file2".to_string(),
            Node {
                path: "folder/folder2/file2".to_string(),
                content: Some("".to_string()),
                git_url: "".to_string(),
                sha: "x132".to_string(),
            },
        );

        assert_eq!(new_tree, expected_tree);
    }
}
