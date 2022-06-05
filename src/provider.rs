use async_trait::async_trait;

use crate::{cli::DestinationRepository, git_tree};

#[async_trait]
pub trait Provider<T> {
    fn configure_provider(&self, base_url: Option<String>) -> T;

    async fn create_source_tree(&self, instance: T) -> git_tree::Tree;

    async fn create_destination_branch(
        &self,
        instance: T,
        destination: &DestinationRepository,
        destination_branch_name: &str,
    ) -> ();

    async fn create_destination_tree(
        &self,
        instance: T,
        destination: &DestinationRepository,
    ) -> git_tree::Tree;

    async fn create_file(
        instance: T,
        destination: &DestinationRepository,
        path: &str,
        content: &Option<String>,
        destination_branch_name: &str,
    ) -> ();

    async fn update_file(
        instance: T,
        destination: &DestinationRepository,
        path: &str,
        content: &Option<String>,
        sha: &str,
        destination_branch_name: &str,
    ) -> ();

    async fn delete_file(
        instance: T,
        destination: &DestinationRepository,
        path: &str,
        sha: &str,
        destination_branch_name: &str,
    ) -> ();

    fn get_destination_branch(&self) -> String;

    async fn create_pull_request_destination(
        &self,
        instance: T,
        destination: &DestinationRepository,
        destination_branch_name: &str,
    ) -> ();
}
