#[derive(Debug)]
pub enum Node<'a> {
    Root {
      path: Option<String>,
      children: Vec<&'a Node<'a>>
    },
    File {
        path: String,
        content: Option<String>,
        git_url: String,
    },
    Folder {
        path: String,
        children: Vec<Node<'a>>,
    },
}

pub struct Tree<'a> {
    root: Node<'a>,
}

impl<'a> Tree<'a> {
    pub fn new(node: Node<'a>) -> Tree<'a> {
        Tree { root: node }
    }

    pub fn apply_transformation<F>(&self, predicate: F) -> Tree
    where
        F: Fn(&Node) -> bool,
    {
        let mut nodes: Vec<&Node> = Vec::new();
        self.df_traverse_helper(self.root, &predicate, &mut nodes);

        let node = Node::Root {
            path: None,
            children: nodes
        };
        Tree::new(node)
    }

    fn df_traverse_helper<'b, F>(
        &self,
        node: Node<'b>,
        predicate: &F,
        result_nodes: &mut Vec<&'b Node<'b>>,
    ) -> ()
    where
        F: Fn(&Node) -> bool,
    {
        match node {
            Node::File { path, .. } => {
                if predicate(&node) {
                    result_nodes.push(&node);
                }
            }
            Node::Folder { children, .. }  => {
                if predicate(&node) {
                    result_nodes.push(&node);
                } else {
                    for child in children {
                        self.df_traverse_helper(child, &predicate, result_nodes);
                    }
                }
            }
            Node::Root { children, .. } => {
                if predicate(&node) {
                    result_nodes.push(&node);
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
    use super::Cell;
    fn bad() {
        // use std::sync::Arc;
        // let x = Arc::new(Cell::new(42));
        // let x1 = Arc::clone(&x);

        // std::thread::spawn(|| {
        //   x1._set(43);
        // });

        // let x2 = Arc::clone(&x);
        // std::thread::spawn(|| {
        //   x2._set(44)
        // });
    }
}
