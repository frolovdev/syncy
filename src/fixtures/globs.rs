use glob::glob;

use crate::cli::GlobExpression;

pub fn create_glob_single(val: &str) -> GlobExpression {
    let glob_pattern = glob::Pattern::new(val).unwrap();

    GlobExpression::Single(glob_pattern)
}

pub fn create_glob_single_with_exclude(val: &str, exclude: &str) -> GlobExpression {
  let glob_pattern = glob::Pattern::new(val).unwrap();
  let glob_pattern_exclude = glob::Pattern::new(exclude).unwrap();

  GlobExpression::SingleWithExclude(glob_pattern, glob_pattern_exclude)
}
