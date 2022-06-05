use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

pub fn get_content_mock(
    owner: &str,
    repo: &str,
    pathname: Option<&str>,
    response: serde_json::Value,
) -> Mock {
    Mock::given(method("GET"))
        .and(path(format!(
            "/repos/{owner}/{repo}/contents/{path}",
            owner = owner,
            repo = repo,
            path = pathname.unwrap_or("")
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
}
