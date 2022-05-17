use std::{cell::RefCell, rc::Rc};

use futures::future::{join_all, BoxFuture};

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
        F: Fn(&Rc<Node>) -> bool,
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
        predicate: &dyn Fn(&Rc<Node>) -> bool,
        result_nodes: &mut Vec<Rc<Node>>,
    ) -> () {
        match node.as_ref() {
            Node::File { .. } => {
                if predicate(&node) {
                    result_nodes.push(node.clone());
                }
            }
            Node::Folder { children, .. } => {
                if predicate(&node) {
                    result_nodes.push(node.clone());
                } else {
                    for child in children {
                        self.apply_transformation_traverse_helper(&child, &predicate, result_nodes);
                    }
                }
            }
            Node::Root { children, .. } => {
                if predicate(&node) {
                    result_nodes.push(node.clone());
                } else {
                    for child in children {
                        self.apply_transformation_traverse_helper(child, &predicate, result_nodes);
                    }
                }
            }
        }
    }

    pub async fn traverse(&self, predicate: Box<dyn Fn(&Node) -> BoxFuture<'static, ()>>) {
        let node = self.root.clone();
        let mut futures: Vec<BoxFuture<'static, ()>> = vec![];
        self.traverse_helper(node, &predicate, &mut futures);

        join_all(futures).await;
    }

    fn traverse_helper(
        &self,
        node: Rc<Node>,
        predicate: &Box<dyn Fn(&Node) -> BoxFuture<'static, ()>>,
        joins: &mut Vec<BoxFuture<'static, ()>>,
    ) -> () {
        match node.as_ref() {
            Node::File { .. } => {
                let future = predicate(&node);
                joins.push(future);
            }
            Node::Folder { children, .. } => {
                let future = predicate(&node);
                joins.push(future);

                for child in children {
                    let forked = Rc::clone(&child);
                    self.traverse_helper(forked, &predicate, joins);
                }
            }
            Node::Root { children, .. } => {
                for child in children {
                    let forked = Rc::clone(&child);
                    self.traverse_helper(forked, &predicate, joins);
                }
            }
        }
    }
}
