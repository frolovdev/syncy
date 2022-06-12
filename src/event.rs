#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum Event {
    Create {
        path: String,
        content: Option<String>,
    },
    Update {
        path: String,
        content: Option<String>,
        sha: String,
    },
    Delete {
        path: String,
        sha: String,
    },
}
