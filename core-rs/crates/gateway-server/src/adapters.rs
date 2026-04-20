use axum::http::{header, HeaderMap, HeaderValue};

pub struct GeminiAdapter;

impl GeminiAdapter {
    const OPENAI_CHAT_COMPLETIONS_URL: &'static str =
        "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions";
    const NATIVE_BASE_URL: &'static str = "https://generativelanguage.googleapis.com/v1beta";

    pub fn openai_chat_completions_url(forward_query: &str) -> String {
        if forward_query.is_empty() {
            Self::OPENAI_CHAT_COMPLETIONS_URL.to_string()
        } else {
            format!("{}?{}", Self::OPENAI_CHAT_COMPLETIONS_URL, forward_query)
        }
    }

    pub fn apply_openai_auth(headers: &mut HeaderMap, api_key: &str) -> Result<(), ()> {
        let value = HeaderValue::from_str(&format!("Bearer {}", api_key)).map_err(|_| ())?;
        headers.insert(header::AUTHORIZATION, value);
        Ok(())
    }

    pub fn apply_native_api_key(headers: &mut HeaderMap, api_key: &str) -> Result<(), ()> {
        let value = HeaderValue::from_str(api_key).map_err(|_| ())?;
        headers.insert(header::HeaderName::from_static("x-goog-api-key"), value);
        Ok(())
    }

    pub fn native_proxy_url(sub_path: &str, forward_query: &str) -> String {
        if forward_query.is_empty() {
            format!("{}/{}", Self::NATIVE_BASE_URL, sub_path)
        } else {
            format!("{}/{}?{}", Self::NATIVE_BASE_URL, sub_path, forward_query)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GeminiAdapter;
    use axum::http::HeaderMap;

    #[test]
    fn openai_chat_url_should_not_include_key_query_by_default() {
        let url = GeminiAdapter::openai_chat_completions_url("");
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
        );
    }

    #[test]
    fn openai_chat_url_should_keep_forward_query() {
        let url = GeminiAdapter::openai_chat_completions_url("stream=true");
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions?stream=true"
        );
    }

    #[test]
    fn apply_openai_auth_should_set_bearer_header() {
        let mut headers = HeaderMap::new();
        GeminiAdapter::apply_openai_auth(&mut headers, "sk-user-test").expect("auth header");
        let value = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(value, "Bearer sk-user-test");
    }

    #[test]
    fn apply_native_api_key_should_set_x_goog_api_key_header() {
        let mut headers = HeaderMap::new();
        GeminiAdapter::apply_native_api_key(&mut headers, "AIza-test").expect("api key header");
        let value = headers
            .get("x-goog-api-key")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(value, "AIza-test");
    }

    #[test]
    fn native_proxy_url_should_not_append_key_query() {
        let url = GeminiAdapter::native_proxy_url("models/gemini:generateContent", "");
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini:generateContent"
        );
    }
}
