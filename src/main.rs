use event::Event;
use git_tree::GitTree;
use github_provider::GithubProvider;
use provider::Provider;

mod cli;
mod event;
mod fixtures;
mod git_tree;
mod github_provider;
mod provider;

#[tokio::main]
async fn main() {
    let config = cli::run().unwrap();

    let github_provider = github_provider::GithubProvider { config };

    let instance = github_provider.configure_provider(None);

    let source_tree = github_provider.create_source_tree(instance.clone()).await;

    let destination_branch_name = github_provider.get_destination_branch();

    for destination in github_provider.config.destinations.iter() {
        let destination_tree = github_provider
            .create_destination_tree(instance.clone(), &destination)
            .await;

        github_provider
            .create_destination_branch(instance.clone(), &destination, &destination_branch_name)
            .await;

        let events = source_tree.generate_events(&destination_tree);

        for event in events.iter() {
            match &event {
                Event::Create { path, content } => {
                    GithubProvider::create_file(
                        instance.clone(),
                        &destination,
                        &path,
                        content,
                        &destination_branch_name,
                    )
                    .await;
                }
                Event::Update { path, content, sha } => {
                    GithubProvider::update_file(
                        instance.clone(),
                        &destination,
                        path,
                        content,
                        sha,
                        &destination_branch_name,
                    )
                    .await;
                }
                Event::Delete { path, sha } => {
                    GithubProvider::delete_file(
                        instance.clone(),
                        &destination,
                        &path,
                        &sha,
                        &destination_branch_name,
                    )
                    .await;
                }
            };
        }

        github_provider
            .create_pull_request_destination(
                instance.clone(),
                &destination,
                &destination_branch_name,
            )
            .await;
    }
}
