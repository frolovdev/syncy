use std::collections::HashMap;

use crate::cli::GlobExpression;

#[derive(Debug)]
pub struct Node {
    pub path: String,
    pub content: Option<String>,
    pub git_url: String,
}

pub type Tree = HashMap<String, Node>;

pub trait GitTree {
    fn transform_tree(self, origin_files_glob: &Option<GlobExpression>, root_path: &str) -> Tree;
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
}

#[cfg(test)]
mod tests {

    use crate::fixtures::globs::{create_glob_single};

    use super::{GitTree, Node, Tree};

    #[test]
    fn test_success() {
        let mut tree = Tree::new();

        tree.insert(
            "folder/file1".to_string(),
            Node {
                path: "".to_string(),
                content: Some("".to_string()),
                git_url: "".to_string(),
            },
        );
        tree.insert(
            "folder/folder2/file2".to_string(),
            Node {
                path: "".to_string(),
                content: Some("".to_string()),
                git_url: "".to_string(),
            },
        );
        tree.insert(
            "folder/file3".to_string(),
            Node {
                path: "".to_string(),
                content: Some("".to_string()),
                git_url: "".to_string(),
            },
        );

        let glob = create_glob_single("**");

        let new_tree = tree.transform_tree(&glob, "");
    }
}
