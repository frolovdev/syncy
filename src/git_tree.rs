use std::collections::HashMap;

use crate::{cli::GlobExpression, event::Event};

#[derive(Debug, PartialEq)]
pub struct Node {
    pub path: String,
    pub content: Option<String>,
    pub git_url: String,
}

pub type Tree = HashMap<String, Node>;

pub trait GitTree {
    fn transform_tree(self, origin_files_glob: &Option<GlobExpression>, root_path: &str) -> Tree;

    fn generate_events(&self, destination_tree: &Tree) -> Vec<Event>;
}

impl GitTree for HashMap<String, Node> {
    fn transform_tree(self, origin_files_glob: &Option<GlobExpression>, root_path: &str) -> Self {
        let unwrapped_origin_glob = origin_files_glob.as_ref().unwrap();

        let mut new_tree = Tree::new();
        for (key, node) in self {
            match unwrapped_origin_glob {
                GlobExpression::Single(pattern) => {
                    if pattern.matches(&key) {
                        let new_val =
                            key.trim_start_matches(&format!("{root_path}/", root_path = root_path));

                        new_tree.insert(new_val.to_string(), node);
                    }
                }
                GlobExpression::SingleWithExclude(include_pattern, exclude_pattern) => {
                    if include_pattern.matches(&key) && !exclude_pattern.matches(&key) {
                        let new_val =
                            key.trim_start_matches(&format!("{root_path}/", root_path = root_path));

                        new_tree.insert(new_val.to_string(), node);
                    }
                }
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
                    path: source_key.to_string(),
                    content: destination_node.content.clone(),
                }),
                None => events.push(Event::Create {
                    path: source_key.to_string(),
                    content: source_node.content.clone(),
                }),
            }
        }

        for (dest_key, _) in destination_tree.iter() {
            if !self.contains_key(dest_key) {
                events.push(Event::Delete {
                    path: dest_key.to_string(),
                })
            }
        }

        events
    }
}

#[cfg(test)]
mod tests {

    use crate::fixtures::globs::create_glob_single;

    use super::{GitTree, Node, Tree};

    #[test]
    fn test_success() {
        let mut tree = Tree::new();

        tree.insert(
            "folder/file1".to_string(),
            Node {
                path: "folder/file1".to_string(),
                content: Some("".to_string()),
                git_url: "".to_string(),
            },
        );
        tree.insert(
            "folder/folder2/file2".to_string(),
            Node {
                path: "folder/folder2/file2".to_string(),
                content: Some("".to_string()),
                git_url: "".to_string(),
            },
        );
        tree.insert(
            "folder/file3".to_string(),
            Node {
                path: "folder/file3".to_string(),
                content: Some("".to_string()),
                git_url: "".to_string(),
            },
        );

        let glob = create_glob_single("folder/folder2/**");

        let new_tree = tree.transform_tree(&Some(glob), "");

        let mut expected_tree = Tree::new();
        expected_tree.insert(
            "folder/folder2/file2".to_string(),
            Node {
                path: "folder/folder2/file2".to_string(),
                content: Some("".to_string()),
                git_url: "".to_string(),
            },
        );

        assert_eq!(new_tree, expected_tree);
    }
}
