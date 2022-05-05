#[derive(Debug)]
pub enum Node {
    File {
        path: String,
        content: Option<String>,
        git_url: String,
    },
    Folder {
        path: String,
        children: Vec<Node>,
    },
}

pub struct Tree<'a> {
    root: &'a Node,
}

impl<'a> Tree<'a> {
    pub fn new(node: &Node) -> Tree {
        Tree { root: node }
    }

    pub fn apply_transformation(&self, f: FnOnce | FnMut) {}
}
