use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SourceRepository {
    pub owner: String,
    pub name: String,
    pub git_ref: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct DestinationRepository {
    pub owner: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]

pub struct MoveArgs {
    pub before: String,
    pub after: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub enum Transformation {
    Move { args: MoveArgs },
}
