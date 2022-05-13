use std::rc::Rc;

#[derive(Debug)]
pub enum Node {
    Root {
        path: Option<String>,
        children: Vec<Rc<Node>>,
    },
    File {
        path: String,
        content: Option<String>,
        git_url: String,
    },
    Folder {
        path: String,
        children: Vec<Rc<Node>>,
    },
}

#[derive(Debug)]
pub struct Tree {
    root: Rc<Node>,
}

impl Tree {
    pub fn new(node: Node) -> Tree {
        Tree {
            root: Rc::new(node),
        }
    }

    pub fn apply_transformation<F>(&self, predicate: F) -> Tree
    where
        F: Fn(&Node) -> bool,
    {
        let mut nodes: Vec<Rc<Node>> = Vec::new();
        self.df_traverse_helper(&self.root, &predicate, &mut nodes);

        let root = Node::Root {
            path: None,
            children: nodes,
        };
        Tree::new(root)
    }

    fn df_traverse_helper(
        &self,
        node: &Rc<Node>,
        predicate: &dyn Fn(&Node) -> bool,
        result_nodes: &mut Vec<Rc<Node>>,
    ) -> () {
        match node.as_ref() {
            Node::File { .. } => {
                if predicate(&node) {
                    result_nodes.push(Rc::clone(&node));
                }
            }
            Node::Folder { children, .. } => {
                if predicate(&node) {
                    result_nodes.push(Rc::clone(&node));
                } else {
                    for child in children {
                        self.df_traverse_helper(&child, &predicate, result_nodes);
                    }
                }
            }
            Node::Root { children, .. } => {
                if predicate(&node) {
                    result_nodes.push(Rc::clone(&node));
                } else {
                    for child in children {
                        self.df_traverse_helper(child, &predicate, result_nodes);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Node, Tree};
    use std::rc::Rc;
    #[test]
    fn it_works() {
        let child3 = Rc::new(Node::Folder {
            path: "a/b/c".to_string(),
            children: Vec::new(),
        });
        let child1 = Rc::new(Node::Folder {
            path: "a/b".to_string(),
            children: vec![child3],
        });

        let child2 = Rc::new(Node::Folder {
            path: "a/c".to_string(),
            children: Vec::new(),
        });
        let root = Node::Root {
            path: Some("a".to_string()),
            children: vec![child1, child2],
        };
        let tree = Tree::new(root);

        let new_tree = tree.apply_transformation(|n| match n {
            Node::Folder { path, .. } => path == "a/b",
            _ => false,
        });

        match tree.root.as_ref() {
            Node::Root { path, children } => {
                println!("mems {}", Rc::strong_count(children.last().unwrap()));
            }
            Node::File {
                path,
                content,
                git_url,
            } => {}
            Node::Folder { path, children } => {}
        }

        match new_tree.root.as_ref() {
            Node::Root { path, children } => {
                println!("kekos {}", Rc::strong_count(children.first().unwrap()));
            }
            Node::File {
                path,
                content,
                git_url,
            } => {}
            Node::Folder { path, children } => {}
        }

        println!("{:?}", new_tree);
    }
}
