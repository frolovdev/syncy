use base64;
use octocrab::models::repos::{Content, ContentLinks};
use reqwest::Url;

pub fn get_content_json(path: &str, name: &str, content: Option<&str>, r#type: &str) -> Content {
    Content {
        path: path.to_string(),
        name: name.to_string(),
        sha: "".to_string(),
        content: Some(base64::encode(content.unwrap_or("").to_string())),
        size: 45,
        url: "".to_string(),
        html_url: "".to_string(),
        git_url: "".to_string(),
        download_url: Some("".to_string()),
        r#type: r#type.to_string(),
        links: ContentLinks {
            git: Url::parse("https://example.net").unwrap(),
            html: Url::parse("https://example.net").unwrap(),
            _self: Url::parse("https://example.net").unwrap(),
        },
        license: None,
    }
}
