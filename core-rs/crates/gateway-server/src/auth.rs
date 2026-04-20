use axum::http::HeaderMap;

pub fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get("authorization")?.to_str().ok()?.trim();
    let token = raw.strip_prefix("Bearer ")?;
    Some(token.trim().to_string())
}

pub fn cookie_token(headers: &HeaderMap, cookie_name: &str) -> Option<String> {
    let raw = headers.get("cookie")?.to_str().ok()?;
    for item in raw.split(';') {
        let trimmed = item.trim();
        if let Some((k, v)) = trimmed.split_once('=') {
            if k == cookie_name {
                return Some(v.to_string());
            }
        }
    }
    None
}

pub fn is_admin(headers: &HeaderMap, cookie_name: &str, auth_token: &str) -> bool {
    if let Some(token) = cookie_token(headers, cookie_name) {
        if token == auth_token {
            return true;
        }
    }
    if let Some(token) = bearer_token(headers) {
        return token == auth_token;
    }
    false
}

pub fn api_key_token(headers: &HeaderMap) -> Option<String> {
    if let Some(k) = headers
        .get("x-goog-api-key")
        .or_else(|| headers.get("x-api-key"))
    {
        return k.to_str().ok().map(|s| s.trim().to_string());
    }
    None
}

pub fn query_key_token(query: Option<&str>) -> Option<String> {
    let query = query?;
    for pair in query.split('&') {
        let segment = pair.trim();
        if segment.is_empty() {
            continue;
        }
        let (key, value) = segment.split_once('=').unwrap_or((segment, ""));
        if key == "key" || key == "api_key" {
            let token = value.trim();
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }
    }
    None
}

pub fn is_allowed_user(
    headers: &HeaderMap,
    query: Option<&str>,
    allowed_tokens: &[String],
    auth_token: &str,
) -> bool {
    if let Some(token) = bearer_token(headers)
        .or_else(|| api_key_token(headers))
        .or_else(|| query_key_token(query))
    {
        return token == auth_token || allowed_tokens.iter().any(|it| it == &token);
    }
    false
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn is_allowed_user_accepts_bearer_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer sk-user-1"),
        );
        let allowed = vec!["sk-user-1".to_string()];
        assert!(is_allowed_user(&headers, None, &allowed, "sk-admin"));
    }

    #[test]
    fn is_allowed_user_accepts_query_key() {
        let headers = HeaderMap::new();
        let allowed = vec!["sk-user-1".to_string()];
        assert!(is_allowed_user(
            &headers,
            Some("key=sk-user-1"),
            &allowed,
            "sk-admin",
        ));
    }
}
