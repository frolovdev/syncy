use std::{cell::RefCell, pin::Pin, rc::Rc};

use async_recursion::async_recursion;

#[derive(Debug)]
pub enum Node {
    Root {
        path: Option<RefCell<String>>,
        children: Vec<Rc<Node>>,
    },
    File {
        path: RefCell<String>,
        content: Option<String>,
        git_url: String,
    },
    Folder {
        path: RefCell<String>,
        children: Vec<Rc<Node>>,
    },
}

#[derive(Debug)]
pub struct Tree {
    pub root: Rc<Node>,
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
        self.apply_transformation_traverse_helper(&self.root, &predicate, &mut nodes);

        let root = Node::Root {
            path: None,
            children: nodes,
        };
        Tree::new(root)
    }

    fn apply_transformation_traverse_helper(
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
                        self.apply_transformation_traverse_helper(&child, &predicate, result_nodes);
                    }
                }
            }
            Node::Root { children, .. } => {
                if predicate(&node) {
                    result_nodes.push(Rc::clone(&node));
                } else {
                    for child in children {
                        self.apply_transformation_traverse_helper(child, &predicate, result_nodes);
                    }
                }
            }
        }
    }

    pub async fn traverse(
        &self,
        predicate: Box<dyn Fn(&Node) -> Pin<Box<(dyn std::future::Future<Output = ()>)>>>,
    ) {
        self.traverse_helper(&self.root, &predicate).await;
    }

    #[async_recursion(?Send)]
    async fn traverse_helper(
        &self,
        node: &Rc<Node>,
        predicate: &Box<dyn Fn(&Node) -> Pin<Box<(dyn std::future::Future<Output = ()>)>>>,
    ) -> () {
        match node.as_ref() {
            Node::File { .. } => {
                predicate(&node).await;
            }
            Node::Folder { children, .. } => {
                predicate(&node).await;
                for child in children {
                    self.traverse_helper(&child, &predicate).await;
                }
            }
            Node::Root { children, .. } => {
                for child in children {
                    self.traverse_helper(child, &predicate).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Node, Tree};
    use std::{cell::RefCell, rc::Rc};
    #[test]
    fn it_works() {
        let child3 = Rc::new(Node::Folder {
            path: RefCell::new("a/b/c".to_string()),
            children: Vec::new(),
        });
        let child1 = Rc::new(Node::Folder {
            path: RefCell::new("a/b".to_string()),
            children: vec![child3],
        });

        let child2 = Rc::new(Node::Folder {
            path: RefCell::new("a/c".to_string()),
            children: Vec::new(),
        });
        let root = Node::Root {
            path: Some(RefCell::new("a".to_string())),
            children: vec![child1, child2],
        };
        let tree = Tree::new(root);

        let new_tree = tree.apply_transformation(|n| match n {
            Node::Folder { path, .. } => *path.borrow() == "a/b",
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
