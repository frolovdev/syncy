use crate::cli::{GlobExpression, WorkDirExpression};

pub fn create_glob_single(val: &str) -> WorkDirExpression {
    let glob_pattern = glob::Pattern::new(val).unwrap();

    WorkDirExpression::Glob(GlobExpression::Single(glob_pattern))
}

pub fn create_glob_single_with_exclude(val: &str, exclude: &str) -> WorkDirExpression {
    let glob_pattern = glob::Pattern::new(val).unwrap();
    let glob_pattern_exclude = glob::Pattern::new(exclude).unwrap();

    WorkDirExpression::Glob(GlobExpression::SingleWithExclude(
        glob_pattern,
        glob_pattern_exclude,
    ))
}

pub fn create_workdir_path(val: &str) -> WorkDirExpression {
    WorkDirExpression::Path(val.to_string())
}
