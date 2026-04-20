use std::{collections::HashMap, env};

use gateway_core::types::{ModelPoolScope, ModelPoolStrategy, PoolStrategy};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_host: String,
    pub port_start: u16,
    pub port_end: u16,
    pub auth_token: String,
    pub allowed_tokens: Vec<String>,
    pub api_keys: Vec<String>,
    pub session_cookie_name: String,
    pub compat_mode: bool,
    pub max_failures: u32,
    pub cooldown_secs: u64,
    pub pool_strategy: PoolStrategy,
    pub thinking_models: Vec<String>,
    pub image_models: Vec<String>,
    pub search_models: Vec<String>,
    pub filtered_models: Vec<String>,
    pub url_context_models: Vec<String>,
    /// Alias → list of real Gemini model names
    pub model_pools: HashMap<String, Vec<String>>,
    pub model_pool_strategy: ModelPoolStrategy,
    pub model_pool_scope: ModelPoolScope,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        let bind_host = env::var("RUNTIME_BIND_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port_start = env::var("RUNTIME_PORT_START")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(18080);
        let port_end = env::var("RUNTIME_PORT_END")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(18099);

        let api_keys = parse_env_array("API_KEYS", "AIzaSy_demo_key_1,AIzaSy_demo_key_2");
        let allowed_tokens = parse_env_array("ALLOWED_TOKENS", "sk-user-demo");
        let auth_token = env::var("AUTH_TOKEN").unwrap_or_else(|_| {
            allowed_tokens
                .first()
                .cloned()
                .unwrap_or_else(|| "sk-admin-demo".to_string())
        });

        let thinking_models = parse_env_array("THINKING_MODELS", "gemini-2.5-flash,gemini-2.5-pro");
        let image_models = parse_env_array(
            "IMAGE_MODELS",
            "gemini-2.0-flash-exp,gemini-2.5-flash-image-preview",
        );
        let search_models = parse_env_array("SEARCH_MODELS", "gemini-2.5-flash,gemini-2.5-pro");
        let filtered_models = parse_env_array("FILTERED_MODELS", "");
        let url_context_models =
            parse_env_array("URL_CONTEXT_MODELS", "gemini-2.5-pro,gemini-2.5-flash");

        let session_cookie_name =
            env::var("SESSION_COOKIE_NAME").unwrap_or_else(|_| "gb_session".to_string());
        let compat_mode = env::var("COMPAT_MODE")
            .ok()
            .map(|s| matches!(s.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(true);
        let max_failures = env::var("MAX_FAILURES")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(3);
        let cooldown_secs = env::var("COOLDOWN_SECONDS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(60);
        let pool_strategy = env::var("POOL_STRATEGY")
            .ok()
            .and_then(|s| s.parse::<PoolStrategy>().ok())
            .unwrap_or_default();

        let model_pools = parse_model_pools();
        let model_pool_strategy = env::var("MODEL_POOL_STRATEGY")
            .ok()
            .and_then(|s| s.parse::<ModelPoolStrategy>().ok())
            .unwrap_or_default();
        let model_pool_scope = env::var("MODEL_POOL_SCOPE")
            .ok()
            .and_then(|s| s.parse::<ModelPoolScope>().ok())
            .unwrap_or_default();

        Self {
            bind_host,
            port_start,
            port_end,
            auth_token,
            allowed_tokens,
            api_keys,
            session_cookie_name,
            compat_mode,
            max_failures,
            cooldown_secs,
            pool_strategy,
            thinking_models,
            image_models,
            search_models,
            filtered_models,
            url_context_models,
            model_pools,
            model_pool_strategy,
            model_pool_scope,
        }
    }
}

fn parse_env_array(key: &str, default_value: &str) -> Vec<String> {
    let raw = env::var(key)
        .unwrap_or_else(|_| default_value.to_string())
        .trim()
        .to_string();

    parse_array_value(&raw)
}

fn parse_array_value(raw: &str) -> Vec<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Vec::new();
    }

    if raw.starts_with('[') && raw.ends_with(']') {
        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(raw) {
            return arr
                .into_iter()
                .filter_map(|v| match v {
                    serde_json::Value::String(s) => normalize_item(&s),
                    _ => None,
                })
                .collect();
        }
        let inner = &raw[1..raw.len().saturating_sub(1)];
        return inner.split(',').filter_map(normalize_item).collect();
    }

    raw.split(',').filter_map(normalize_item).collect()
}

fn normalize_item(value: &str) -> Option<String> {
    let normalized = value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

/// Parse MODEL_POOLS environment variable.
///
/// Supported formats:
///   JSON object: {"alias":["model-a","model-b"],...}
///   Semi-colon separated pairs: alias=model-a,model-b;alias2=model-c
fn parse_model_pools() -> HashMap<String, Vec<String>> {
    let raw = match env::var("MODEL_POOLS") {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => return HashMap::new(),
    };

    parse_model_pools_value(&raw)
}

fn parse_model_pools_value(raw: &str) -> HashMap<String, Vec<String>> {
    let raw = raw.trim();
    if raw.is_empty() {
        return HashMap::new();
    }

    if raw.starts_with('{') && raw.ends_with('}') {
        if let Ok(map) = serde_json::from_str::<HashMap<String, serde_json::Value>>(raw) {
            let parsed = map
                .into_iter()
                .filter_map(|(k, v)| {
                    let alias = normalize_item(&k)?;
                    let models: Vec<String> = match v {
                        serde_json::Value::Array(arr) => arr
                            .into_iter()
                            .filter_map(|x| match x {
                                serde_json::Value::String(s) => normalize_item(&s),
                                _ => None,
                            })
                            .collect(),
                        serde_json::Value::String(s) => parse_array_value(&s),
                        _ => return None,
                    };
                    if models.is_empty() {
                        None
                    } else {
                        Some((alias, models))
                    }
                })
                .collect::<HashMap<_, _>>();
            if !parsed.is_empty() {
                return parsed;
            }
        }

        let parsed = parse_loose_object_model_pools(raw);
        if !parsed.is_empty() {
            return parsed;
        }
    }

    // Fallback: alias=model-a,model-b;alias2=model-c
    let mut result = HashMap::new();
    for pair in raw.split(';') {
        if let Some((alias, models_str)) = pair.split_once('=') {
            let Some(alias) = normalize_item(alias) else {
                continue;
            };
            let models = parse_array_value(models_str);
            if !models.is_empty() {
                result.insert(alias, models);
            }
        }
    }
    result
}

fn parse_loose_object_model_pools(raw: &str) -> HashMap<String, Vec<String>> {
    let inner = raw
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .trim();
    if inner.is_empty() {
        return HashMap::new();
    }

    let mut result = HashMap::new();
    let mut depth = 0_i32;
    let mut start = 0_usize;
    for (idx, ch) in inner.char_indices() {
        match ch {
            '[' | '{' => depth += 1,
            ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                parse_loose_model_pool_segment(&inner[start..idx], &mut result);
                start = idx + 1;
            }
            _ => {}
        }
    }
    parse_loose_model_pool_segment(&inner[start..], &mut result);
    result
}

fn parse_loose_model_pool_segment(segment: &str, result: &mut HashMap<String, Vec<String>>) {
    let piece = segment.trim();
    if piece.is_empty() {
        return;
    }
    let (alias_raw, models_raw) = piece
        .split_once(':')
        .or_else(|| piece.split_once('='))
        .unwrap_or((piece, ""));
    let Some(alias) = normalize_item(alias_raw) else {
        return;
    };
    let models = parse_array_value(models_raw);
    if !models.is_empty() {
        result.insert(alias, models);
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_array_value, parse_model_pools_value};

    #[test]
    fn parse_array_value_supports_json_array() {
        let values = parse_array_value("[\"a\", \"b\"]");
        assert_eq!(values, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn parse_array_value_supports_bracket_without_quotes() {
        let values = parse_array_value("[a, b]");
        assert_eq!(values, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn parse_model_pools_value_supports_loose_object_format() {
        let pools = parse_model_pools_value("{claude-sonnet:[gemma-4-26b-a4b-it,gemma-4-31b-it]}");
        let expected = vec![
            "gemma-4-26b-a4b-it".to_string(),
            "gemma-4-31b-it".to_string(),
        ];
        assert_eq!(pools.get("claude-sonnet"), Some(&expected));
    }

    #[test]
    fn parse_model_pools_value_supports_semicolon_format() {
        let pools =
            parse_model_pools_value("fast=gemini-2.5-flash,gemma-3-4b-it;sonnet=gemini-2.5-pro");
        let expected_fast = vec!["gemini-2.5-flash".to_string(), "gemma-3-4b-it".to_string()];
        let expected_sonnet = vec!["gemini-2.5-pro".to_string()];
        assert_eq!(pools.get("fast"), Some(&expected_fast));
        assert_eq!(pools.get("sonnet"), Some(&expected_sonnet));
    }
}
