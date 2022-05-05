// extern crate base64;

// fn strip_trailing_newline(input: &str) -> &str {
//     input
//         .strip_suffix("\r\n")
//         .or(input.strip_suffix("\n"))
//         .unwrap_or(input)
// }

// fn get_sha(object: &models::repos::Object) -> Option<String> {
//     match object {
//         models::repos::Object::Commit { sha, .. } => Some(sha.to_string()),
//         models::repos::Object::Tag { sha, .. } => Some(sha.to_string()),
//         _ => None,
//     }
// }

// fn update_file(octocrab: &Arc<octocrab::Octocrab>) {
//     octocrab::instance()
//         .repos("owner", "repo")
//         .update_file(
//             "crabs/ferris.txt",
//             "Updated ferris.txt",
//             "But me and Ferris Crab: best friends to the end.\n",
//             blob_sha,
//         )
//         .branch("master")
//         .commiter(GitUser {
//             name: "Octocat".to_string(),
//             email: "octocat@github.com".to_string(),
//         })
//         .author(GitUser {
//             name: "Ferris".to_string(),
//             email: "ferris@rust-lang.org".to_string(),
//         })
//         .send()
//         .await?;
// }

// async fn main() -> Result<(), Error> {
//     let token = "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";
//     let octacrab_builder = octocrab::Octocrab::builder().personal_token(token.to_string());

//     octocrab::initialise(octacrab_builder);

//     let source_name = "test1";
//     let source_branch_name = "main";

//     let path_of_file = "test1";

//     let destination_name = "test2";
//     let destination_branch_name = "main";

//     let owner = "frolovdev";

//     let repo1 = octocrab::instance()
//         .repos(owner, source_name)
//         .get_content()
//         .path(path_of_file)
//         .r#ref(source_branch_name)
//         .send()
//         .await?;

//     println!("{:?}", repo1);

//     let base64_content = repo1.items.first().unwrap().content.as_ref().unwrap();

//     let new_content = strip_trailing_newline(base64_content);

//     println!("olololo {:?}", new_content);
//     let content = base64::decode(new_content).unwrap();

//     println!("{:?}", content);

//     let res = String::from_utf8(content);

//     println!("{:?}", res);

//     // end of getting content, let's create a branch
//     let main_ref = octocrab::instance()
//         .repos(owner, destination_name)
//         .get_ref(&Reference::Branch(destination_branch_name.to_string()))
//         .await?;

//     // TODO: cover case when repository is empty
//     let destination_main_sha = get_sha(&main_ref.object).unwrap();

//     println!("pizdec {:?}", destination_main_sha);

//     octocrab::instance()
//         .repos(owner, destination_name)
//         .create_ref(&Reference::Branch("lol".to_string()), destination_main_sha)
//         .await?;

//     // end of creating branch let's create a commit with new data

//     let destination_ref_source_file = octocrab::instance()
//         .repos(owner, destination_name)
//         .get_content()
//         .path(path_of_file)
//         .r#ref(destination_branch_name)
//         .send()
//         .await;

//     match destination_ref_source_file {
//         Ok(F) => update_file(F),
//         Err(E) => return Ok(()),
//     };

//     // end of creating branch, let's create a pull request

//     // done

//     Ok(())
//     // let request_url = format!("https://api.github.com/repos/{owner}/{repo}/stargazers",
//     //                           owner = "rust-lang-nursery",
//     //                           repo = "rust-cookbook");
//     // println!("{}", request_url);
//     // let client = reqwest::Client::new();
//     // let response = client
//     //     .get(request_url)
//     //     .header("User-Agent", "frolovdev")
//     //     .send()
//     //     .await?;

//     // let users: Vec<User> = response.json().await?;
//     // println!("{:?}", users);
//     // Ok(())
// }

mod cli;
mod git_tree;
mod github_provider;

#[tokio::main]
async fn main() {
    let config = cli::run().unwrap();

    github_provider::call(config).await
}
