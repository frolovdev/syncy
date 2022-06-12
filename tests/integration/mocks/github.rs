use std::borrow::Borrow;

use wiremock::matchers::{any, method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

pub fn get_content_mock(
    owner: &str,
    repo: &str,
    pathname: Option<&str>,
    response: serde_json::Value,
    r#ref: Option<&str>,
) -> Mock {
    if let Some(val) = r#ref {
        Mock::given(method("GET"))
            .and(path(format!(
                "/repos/{owner}/{repo}/contents/{path}",
                owner = owner,
                repo = repo,
                path = pathname.unwrap_or("")
            )))
            .and(query_param("ref", val))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
    } else {
        Mock::given(method("GET"))
            .and(path(format!(
                "/repos/{owner}/{repo}/contents/{path}",
                owner = owner,
                repo = repo,
                path = pathname.unwrap_or("")
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
    }
}
