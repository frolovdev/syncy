pub enum Event {
    Create {
        path: String,
        content: Option<String>,
    },
    Update {
        path: String,
        content: Option<String>,
    },
    Delete {
        path: String,
    },
}
