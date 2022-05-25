mod cli;
mod fixtures;
mod git_tree;
mod github_provider;

#[tokio::main]
async fn main() {
    let config = cli::run().unwrap();

    github_provider::call(config).await
}
