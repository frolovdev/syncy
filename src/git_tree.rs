use std::rc::Rc;

use std::collections::HashMap;

#[derive(Debug)]
pub struct Node {
    pub path: String,
    pub content: Option<String>,
    pub git_url: String,
}

#[derive(Debug)]
pub struct Tree(pub HashMap<String, Node>);

impl Tree {
    pub fn new() -> Self {
        Tree(HashMap::new())
    }
}
