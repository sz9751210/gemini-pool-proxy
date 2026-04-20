use std::collections::{HashMap, HashSet};

use axum::{
    body::Body,
    extract::{Path, Query, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{any, delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use futures_util::StreamExt;
use gateway_core::key_pool::mask_key;
use gateway_core::types::{
    KeyActionFailedItemV2, KeyActionResultV2, KeyFiltersAppliedV2, KeyListResponseV2, KeyRecordV2,
    KeyStatus, LogDetailV2, LogListResponseV2, LogRecordV2, ModelPoolScope, ModelPoolStrategy,
    PaginationV2, PoolStrategy, ProxyCheckResultV2,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    adapters::GeminiAdapter,
    auth::{api_key_token, bearer_token, is_admin, is_allowed_user, query_key_token},
    state::SharedState,
};

#[derive(Debug, Deserialize)]
struct LoginBody {
    #[serde(alias = "authToken")]
    auth_token: String,
}

#[derive(Debug, Deserialize, Default)]
struct KeysQuery {
    page: Option<usize>,
    limit: Option<usize>,
    search: Option<String>,
    status: Option<String>,
    #[serde(alias = "minFailureCount")]
    min_failure_count: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
struct KeyUsageQuery {
    period: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct KeyDetailsQuery {
    key: Option<String>,
    period: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct StatsDetailsQuery {
    period: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct AttentionKeysQuery {
    #[serde(alias = "statusCode")]
    status_code: Option<u16>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
struct PoolStatusQuery {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct PoolStrategyBody {
    strategy: String,
}

#[derive(Debug, Deserialize, Default)]
struct KeysActionBody {
    action: String,
    ids: Option<Vec<String>>,
    keys: Option<Vec<String>>,
    #[serde(alias = "keyType")]
    key_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BulkTextBody {
    items: Option<Vec<String>>,
    values: Option<Vec<String>>,
    keys: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ProxyCheckBody {
    proxy: String,
    #[serde(default)]
    use_cache: bool,
}

#[derive(Debug, Deserialize)]
struct ProxyCheckAllBody {
    proxies: Vec<String>,
    #[serde(default)]
    use_cache: bool,
    #[serde(default)]
    max_concurrent: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
struct LogsQuery {
    limit: Option<usize>,
    offset: Option<usize>,
    key_search: Option<String>,
    error_search: Option<String>,
    error_code_search: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    sort_by: Option<String>,
    sort_order: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct LogsLookupQuery {
    id: Option<u64>,
    key: Option<String>,
    status_code: Option<u16>,
    timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeleteBulkBody {
    ids: Vec<u64>,
}

#[derive(Debug, Serialize)]
struct SessionStatus {
    authenticated: bool,
}

#[derive(Debug, Serialize)]
struct OpenAIErrorEnvelope {
    error: OpenAIErrorPayload,
}

#[derive(Debug, Serialize)]
struct OpenAIErrorPayload {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
    code: String,
}

pub fn build_router(state: SharedState) -> Router {
    Router::new()
        .route("/api/v1/session/login", post(v2_login))
        .route("/api/v1/session/logout", post(v2_logout))
        .route("/api/v1/session/status", get(v2_session_status))
        .route("/api/v1/dashboard/overview", get(v2_dashboard_overview))
        .route("/api/v1/keys", get(v2_keys))
        .route("/api/v1/keys/all", get(v2_keys_all))
        .route("/api/v1/keys/actions", post(v2_keys_actions))
        .route("/api/v1/keys/usage/{key}", get(v2_key_usage))
        .route("/api/v1/stats/details", get(v2_stats_details))
        .route("/api/v1/stats/attention-keys", get(v2_stats_attention_keys))
        .route("/api/v1/stats/key-details", get(v2_stats_key_details))
        .route("/api/v1/config", get(v2_config_get).put(v2_config_put))
        .route("/api/v1/config/reset", post(v2_config_reset))
        .route("/api/v1/config/schema", get(v2_config_schema))
        .route("/api/v1/config/ui-models", get(v2_config_ui_models))
        .route("/api/v1/config/keys/add", post(v2_config_keys_add))
        .route("/api/v1/config/keys/delete", post(v2_config_keys_delete))
        .route("/api/v1/config/proxies/add", post(v2_config_proxies_add))
        .route(
            "/api/v1/config/proxies/delete",
            post(v2_config_proxies_delete),
        )
        .route("/api/v1/proxy/check", post(v2_proxy_check))
        .route("/api/v1/proxy/check-all", post(v2_proxy_check_all))
        .route("/api/v1/proxy/cache-stats", get(v2_proxy_cache_stats))
        .route("/api/v1/proxy/cache-clear", post(v2_proxy_cache_clear))
        .route("/api/v1/scheduler/start", post(v2_scheduler_start))
        .route("/api/v1/scheduler/stop", post(v2_scheduler_stop))
        .route("/api/v1/scheduler/status", get(v2_scheduler_status))
        .route("/api/v1/logs", get(v2_logs))
        .route("/api/v1/logs/lookup", get(v2_logs_lookup))
        .route(
            "/api/v1/logs/{id}",
            get(v2_logs_detail).delete(v2_logs_delete_one),
        )
        .route("/api/v1/logs/bulk", delete(v2_logs_bulk_delete))
        .route("/api/v1/logs/all", delete(v2_logs_all_delete))
        .route("/api/v1/pool/status", get(v1_pool_status))
        .route("/api/v1/pool/strategy", put(v1_pool_strategy_put))
        .route("/v1/models", get(v2_models))
        .route(
            "/v1/models/{*path}",
            post(v1beta_gemini_proxy).get(v1beta_gemini_proxy),
        )
        .route("/v1/chat/completions", post(v2_chat_completions))
        .route(
            "/v1beta/{*path}",
            post(v1beta_gemini_proxy).get(v1beta_gemini_proxy),
        )
        .route("/api/v2", any(v2_api_gone))
        .route("/api/v2/{*path}", any(v2_api_gone))
        .route("/v2", any(v2_proxy_gone))
        .route("/v2/{*path}", any(v2_proxy_gone))
        .route("/api/config", get(v2_config_get).put(v2_config_put))
        .route("/api/config/reset", post(v2_config_reset))
        .route("/api/config/ui/models", get(v2_config_ui_models))
        .route("/api/config/keys/add", post(v2_config_keys_add))
        .route("/api/config/keys/delete", post(v2_config_keys_delete))
        .route("/api/config/keys/{key}", delete(legacy_delete_key))
        .route(
            "/api/config/keys/delete-selected",
            post(v2_config_keys_delete),
        )
        .route("/api/config/proxy/check", post(v2_proxy_check))
        .route("/api/config/proxy/check-all", post(v2_proxy_check_all))
        .route("/api/config/proxy/cache-stats", get(v2_proxy_cache_stats))
        .route("/api/config/proxy/clear-cache", post(v2_proxy_cache_clear))
        .route("/api/scheduler/start", post(v2_scheduler_start))
        .route("/api/scheduler/stop", post(v2_scheduler_stop))
        .route("/api/scheduler/status", get(v2_scheduler_status))
        .route("/api/stats/details", get(v2_stats_details))
        .route("/api/stats/attention-keys", get(v2_stats_attention_keys))
        .route("/api/stats/key-details", get(v2_stats_key_details))
        .route("/api/keys", get(legacy_keys))
        .route("/api/keys/all", get(legacy_keys_all))
        .route("/api/key-usage-details/{key}", get(v2_key_usage))
        .route("/api/logs/errors", get(v2_logs).delete(v2_logs_bulk_delete))
        .route("/api/logs/errors/all", delete(v2_logs_all_delete))
        .route("/api/logs/errors/lookup", get(v2_logs_lookup))
        .route("/api/logs/errors/{id}", delete(v2_logs_delete_one))
        .route("/api/logs/errors/{id}/details", get(v2_logs_detail))
        .route("/gemini/v1beta/verify-key/{key}", post(legacy_verify_key))
        .route(
            "/gemini/v1beta/reset-fail-count/{key}",
            post(legacy_reset_key_fail_count),
        )
        .route(
            "/gemini/v1beta/verify-selected-keys",
            post(legacy_verify_selected_keys),
        )
        .route("/api/compat/v1/session/login", post(compat_login))
        .route("/api/compat/v1/session/logout", post(compat_logout))
        .route("/api/compat/v1/session/status", get(compat_session_status))
        .route(
            "/api/compat/v1/dashboard/overview",
            get(compat_dashboard_overview),
        )
        .route("/api/compat/v1/keys", get(compat_keys))
        .route("/api/compat/v1/keys/actions", post(compat_keys_actions))
        .route("/api/compat/v1/config/schema", get(compat_config_schema))
        .route("/api/compat/v1/logs", get(compat_logs))
        .route("/api/compat/v1/logs/{id}", get(compat_logs_detail))
        .route("/api/pro/session/login", post(compat_login))
        .route("/api/pro/session/logout", post(compat_logout))
        .route("/api/pro/session", get(compat_session_status))
        .route(
            "/api/pro/dashboard/overview",
            get(compat_dashboard_overview),
        )
        .route("/api/pro/keys", get(compat_keys))
        .route("/api/pro/keys/actions", post(compat_keys_actions))
        .route("/api/pro/config/schema", get(compat_config_schema))
        .route("/api/pro/logs", get(compat_logs))
        .route("/api/pro/logs/{id}", get(compat_logs_detail))
        .with_state(state)
}

fn deprecated_response(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert("Deprecation", HeaderValue::from_static("true"));
    response.headers_mut().insert(
        "Link",
        HeaderValue::from_static("</api/v1>; rel=\"successor-version\""),
    );
    response
}

fn v2_gone_payload(migration_to: &str) -> Value {
    json!({
        "code": "API_VERSION_GONE",
        "message": "v2 routes are permanently disabled. Please migrate to v1.",
        "migrationTo": migration_to
    })
}

fn openai_error_response(
    status: StatusCode,
    message: impl Into<String>,
    error_type: impl Into<String>,
    code: impl Into<String>,
) -> Response {
    let body = OpenAIErrorEnvelope {
        error: OpenAIErrorPayload {
            message: message.into(),
            error_type: error_type.into(),
            code: code.into(),
        },
    };
    (status, Json(body)).into_response()
}

fn openai_error_for_status(status: StatusCode, message: impl Into<String>) -> Response {
    match status {
        StatusCode::BAD_REQUEST => {
            openai_error_response(status, message, "invalid_request_error", "invalid_request")
        }
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            openai_error_response(status, message, "authentication_error", "invalid_api_key")
        }
        StatusCode::TOO_MANY_REQUESTS => {
            openai_error_response(status, message, "rate_limit_error", "rate_limit_exceeded")
        }
        StatusCode::SERVICE_UNAVAILABLE => {
            openai_error_response(status, message, "api_error", "service_unavailable")
        }
        StatusCode::BAD_GATEWAY | StatusCode::GATEWAY_TIMEOUT => {
            openai_error_response(status, message, "api_error", "upstream_service_error")
        }
        _ => openai_error_response(status, message, "api_error", "upstream_service_error"),
    }
}

fn parse_upstream_error_message(body: &[u8]) -> Option<String> {
    let value = serde_json::from_slice::<serde_json::Value>(body).ok()?;
    if let Some(message) = value
        .pointer("/error/message")
        .and_then(serde_json::Value::as_str)
    {
        return Some(message.to_string());
    }
    if let Some(message) = value
        .pointer("/0/error/message")
        .and_then(serde_json::Value::as_str)
    {
        return Some(message.to_string());
    }
    value
        .get("message")
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
}

fn runtime_error_type(status: StatusCode) -> &'static str {
    match status {
        StatusCode::TOO_MANY_REQUESTS => "RateLimit",
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => "Auth",
        StatusCode::BAD_REQUEST => "Request",
        StatusCode::BAD_GATEWAY | StatusCode::SERVICE_UNAVAILABLE | StatusCode::GATEWAY_TIMEOUT => {
            "Upstream"
        }
        _ => "Upstream",
    }
}

fn should_penalize_key_failure_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::UNAUTHORIZED
            | StatusCode::FORBIDDEN
            | StatusCode::TOO_MANY_REQUESTS
            | StatusCode::REQUEST_TIMEOUT
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    ) || status.is_server_error()
}

fn parse_request_pool_strategy(headers: &HeaderMap) -> Result<Option<PoolStrategy>, StatusCode> {
    let Some(raw) = headers.get("x-pool-strategy") else {
        return Ok(None);
    };
    let text = raw.to_str().map_err(|_| StatusCode::BAD_REQUEST)?;
    let strategy = text
        .trim()
        .parse::<PoolStrategy>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Some(strategy))
}

fn sanitize_upstream_headers(headers: &HeaderMap) -> HeaderMap {
    let mut sanitized = headers.clone();
    sanitized.remove("authorization");
    sanitized.remove("host");
    sanitized.remove("x-goog-api-key");
    sanitized.remove("content-length");
    sanitized
}

async fn v2_api_gone() -> impl IntoResponse {
    (StatusCode::GONE, Json(v2_gone_payload("/api/v1")))
}

async fn v2_proxy_gone() -> impl IntoResponse {
    (StatusCode::GONE, Json(v2_gone_payload("/v1")))
}

fn session_cookie_header(name: &str, value: &str, expire: bool) -> String {
    if expire {
        format!("{name}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
    } else {
        format!("{name}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400")
    }
}

fn parse_datetime(text: &str) -> Option<DateTime<Utc>> {
    if let Ok(value) = DateTime::parse_from_rfc3339(text) {
        return Some(value.with_timezone(&Utc));
    }
    if let Ok(value) = NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(value, Utc));
    }
    None
}

fn strip_auth_query_params(query: &str) -> String {
    query
        .split('&')
        .filter_map(|segment| {
            let trimmed = segment.trim();
            if trimmed.is_empty() {
                return None;
            }
            let key = trimmed.split_once('=').map(|(k, _)| k).unwrap_or(trimmed);
            if key == "key" || key == "api_key" {
                return None;
            }
            Some(trimmed.to_string())
        })
        .collect::<Vec<_>>()
        .join("&")
}

async fn admin_guard(headers: &HeaderMap, state: &SharedState) -> Result<(), StatusCode> {
    let guard = state.inner.read().await;
    if is_admin(
        headers,
        &state.session_cookie_name,
        &guard.runtime_config.auth_token,
    ) {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn user_guard(
    headers: &HeaderMap,
    query: Option<&str>,
    state: &SharedState,
) -> Result<(), StatusCode> {
    let guard = state.inner.read().await;
    if is_allowed_user(
        headers,
        query,
        &guard.runtime_config.allowed_tokens,
        &guard.runtime_config.auth_token,
    ) {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn status_matches(record: &KeyRecordV2, status: &str) -> bool {
    match status {
        "all" => true,
        "valid" => matches!(record.status, KeyStatus::Active | KeyStatus::Cooldown),
        "invalid" => matches!(record.status, KeyStatus::Invalid),
        "active" => matches!(record.status, KeyStatus::Active),
        "cooldown" => matches!(record.status, KeyStatus::Cooldown),
        _ => true,
    }
}

fn normalize_bulk_values(body: BulkTextBody) -> Vec<String> {
    let mut values = Vec::new();
    if let Some(items) = body.items {
        values.extend(items);
    }
    if let Some(items) = body.values {
        values.extend(items);
    }
    if let Some(items) = body.keys {
        values.extend(items);
    }
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

fn parse_key_action_targets(
    pool_items: &[KeyRecordV2],
    body: &KeysActionBody,
) -> (Vec<String>, Vec<KeyActionFailedItemV2>) {
    let mut ids = Vec::new();
    let mut failed_items = Vec::new();
    let provided_ids = body.ids.clone().unwrap_or_default();
    let provided_keys = body.keys.clone().unwrap_or_default();

    let mut by_id = HashMap::new();
    let mut by_raw_key = HashMap::new();
    for item in pool_items {
        by_id.insert(item.id.clone(), item.id.clone());
        by_raw_key.insert(item.key.clone(), item.id.clone());
        by_raw_key.insert(item.masked_key.clone(), item.id.clone());
    }

    for id in provided_ids {
        if let Some(mapped) = by_id.get(&id) {
            ids.push(mapped.clone());
        } else {
            failed_items.push(KeyActionFailedItemV2 {
                key: id,
                reason: "找不到指定 id".to_string(),
            });
        }
    }
    for key in provided_keys {
        if let Some(mapped) = by_raw_key.get(&key) {
            ids.push(mapped.clone());
        } else {
            failed_items.push(KeyActionFailedItemV2 {
                key,
                reason: "找不到指定 key".to_string(),
            });
        }
    }

    if ids.is_empty() && failed_items.is_empty() {
        let mode = body.key_type.clone().unwrap_or_else(|| "all".to_string());
        if mode.eq_ignore_ascii_case("all") {
            ids = pool_items.iter().map(|item| item.id.clone()).collect();
        }
    }

    ids.sort();
    ids.dedup();
    (ids, failed_items)
}

fn as_string_list(value: &Value) -> Vec<String> {
    if let Value::Array(items) = value {
        return items
            .iter()
            .filter_map(|item| item.as_str().map(ToString::to_string))
            .collect();
    }
    Vec::new()
}

fn proxy_check_result(proxy: &str) -> ProxyCheckResultV2 {
    let available = proxy.starts_with("http://")
        || proxy.starts_with("https://")
        || proxy.starts_with("socks5://");
    ProxyCheckResultV2 {
        proxy: proxy.to_string(),
        is_available: available,
        response_time: if available { Some(0.12) } else { None },
        error_message: if available {
            None
        } else {
            Some("invalid proxy format".to_string())
        },
        checked_at: Utc::now(),
    }
}

fn build_log_detail(log: &LogRecordV2) -> LogDetailV2 {
    LogDetailV2 {
        id: log.id,
        key_id: log.masked_key.clone(),
        masked_key: log.masked_key.clone(),
        error_type: log.error_type.clone(),
        status_code: log.status_code,
        model: log.model.clone(),
        request_at: log.request_at,
        detail: log.detail.clone(),
        request_body: "{\"messages\":[{\"role\":\"user\",\"content\":\"...\"}]}".to_string(),
        response_body: "{\"error\":\"...\"}".to_string(),
    }
}

async fn v2_login(
    State(state): State<SharedState>,
    Json(body): Json<LoginBody>,
) -> Result<Response, StatusCode> {
    let guard = state.inner.read().await;
    if body.auth_token != guard.runtime_config.auth_token {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let cookie = session_cookie_header(
        &state.session_cookie_name,
        &guard.runtime_config.auth_token,
        false,
    );
    let mut response = Json(json!({ "success": true })).into_response();
    response.headers_mut().insert(
        "Set-Cookie",
        HeaderValue::from_str(&cookie).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok(response)
}

async fn v2_logout(State(state): State<SharedState>) -> Result<Response, StatusCode> {
    let cookie = session_cookie_header(&state.session_cookie_name, "", true);
    let mut response = Json(json!({ "success": true })).into_response();
    response.headers_mut().insert(
        "Set-Cookie",
        HeaderValue::from_str(&cookie).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok(response)
}

async fn v2_session_status(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<SessionStatus>, StatusCode> {
    let guard = state.inner.read().await;
    let authenticated = is_admin(
        &headers,
        &state.session_cookie_name,
        &guard.runtime_config.auth_token,
    );
    Ok(Json(SessionStatus { authenticated }))
}

async fn v2_dashboard_overview(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let guard = state.inner.read().await;
    Ok(Json(
        serde_json::to_value(guard.dashboard_overview())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

async fn v2_keys(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Query(query): Query<KeysQuery>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    guard.key_pool.recover_cooldown(Utc::now());
    let summary = guard.key_pool.summary();
    let mut items = guard.key_pool.snapshot();

    let search = query.search.clone().unwrap_or_default().to_lowercase();
    let status = query
        .status
        .clone()
        .unwrap_or_else(|| "all".to_string())
        .to_lowercase();
    if !search.is_empty() {
        items.retain(|item| {
            item.id.to_lowercase().contains(&search)
                || item.key.to_lowercase().contains(&search)
                || item.masked_key.to_lowercase().contains(&search)
        });
    }
    items.retain(|item| status_matches(item, &status));
    if let Some(min_failure_count) = query.min_failure_count {
        items.retain(|item| item.failure_count >= min_failure_count);
    }

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).max(1);
    let total_items = items.len();
    let total_pages = total_items.div_ceil(limit).max(1);
    let start = (page - 1) * limit;
    let end = (start + limit).min(total_items);
    let paged = if start < total_items {
        items[start..end].to_vec()
    } else {
        Vec::new()
    };

    let payload = KeyListResponseV2 {
        items: paged.clone(),
        summary: summary.clone(),
        pagination: PaginationV2 {
            page,
            page_size: limit,
            total_items,
            total_pages,
        },
        filters_applied: KeyFiltersAppliedV2 {
            search: query.search.unwrap_or_default(),
            status: status.clone(),
            min_failure_count: query.min_failure_count,
        },
    };

    Ok(Json(json!({
        "items": payload.items,
        "summary": payload.summary,
        "pagination": payload.pagination,
        "filtersApplied": payload.filters_applied,
        "totalItems": total_items,
        "totalPages": total_pages,
        "currentPage": page,
        "pageSize": limit
    })))
}

async fn v2_keys_all(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    guard.key_pool.recover_cooldown(Utc::now());
    let items = guard.key_pool.snapshot();
    let valid_keys = items
        .iter()
        .filter(|item| matches!(item.status, KeyStatus::Active | KeyStatus::Cooldown))
        .map(|item| item.key.clone())
        .collect::<Vec<_>>();
    let invalid_keys = items
        .iter()
        .filter(|item| matches!(item.status, KeyStatus::Invalid))
        .map(|item| item.key.clone())
        .collect::<Vec<_>>();
    Ok(Json(json!({
        "items": items,
        "summary": guard.key_pool.summary(),
        "valid_keys": valid_keys,
        "invalid_keys": invalid_keys
    })))
}

async fn v2_keys_actions(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(body): Json<KeysActionBody>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    let items = guard.key_pool.snapshot();
    let (target_ids, mut failed_items) = parse_key_action_targets(&items, &body);
    if target_ids.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let target_set: HashSet<String> = target_ids.iter().cloned().collect();
    let existing_ids = items
        .iter()
        .map(|item| item.id.clone())
        .collect::<HashSet<_>>();
    for id in &target_ids {
        if !existing_ids.contains(id) {
            failed_items.push(KeyActionFailedItemV2 {
                key: id.clone(),
                reason: "找不到指定 key".to_string(),
            });
        }
    }

    let action = body.action.to_lowercase();
    match action.as_str() {
        "verify" | "reset" => {
            guard.key_pool.reset_failures(Some(&target_ids));
        }
        "delete" => {
            guard.key_pool.remove_by_ids(&target_ids);
            let keys = guard.key_pool.raw_keys();
            guard.config_map.insert(
                "API_KEYS".to_string(),
                Value::Array(keys.into_iter().map(Value::String).collect()),
            );
        }
        _ => return Err(StatusCode::BAD_REQUEST),
    }

    let success_count = target_set.len().saturating_sub(failed_items.len());
    let result = KeyActionResultV2 {
        action: action.clone(),
        success_count,
        failed_items,
        message: format!("{action} 已完成"),
    };

    Ok(Json(json!({
        "action": result.action,
        "successCount": result.success_count,
        "failedItems": result.failed_items,
        "message": result.message,
        "success": true
    })))
}

async fn v2_key_usage(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(key): Path<String>,
    Query(query): Query<KeyUsageQuery>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let period = query.period.unwrap_or_else(|| "24h".to_string());
    let guard = state.inner.read().await;
    let usage = guard.key_usage_details(&key, &period);
    Ok(Json(json!({
        "key": key,
        "period": period,
        "usage": usage
    })))
}

async fn v2_stats_details(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Query(query): Query<StatsDetailsQuery>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let period = query.period.unwrap_or_else(|| "24h".to_string());
    let guard = state.inner.read().await;
    let details = guard.stats_details(&period);
    Ok(Json(
        serde_json::to_value(details).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

async fn v2_stats_key_details(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Query(query): Query<KeyDetailsQuery>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let period = query.period.unwrap_or_else(|| "24h".to_string());
    let target_key = query.key.map(|v| v.to_lowercase());
    let guard = state.inner.read().await;
    let from = match period.as_str() {
        "1h" => Utc::now() - chrono::Duration::hours(1),
        "8h" => Utc::now() - chrono::Duration::hours(8),
        "24h" => Utc::now() - chrono::Duration::hours(24),
        _ => Utc::now() - chrono::Duration::hours(24),
    };
    let mut rows = Vec::new();
    for call in guard.calls.iter().filter(|call| call.at >= from) {
        if let Some(target) = target_key.as_ref() {
            if !call.key_id.to_lowercase().contains(target) {
                continue;
            }
        }
        let status = if (200..300).contains(&call.status_code) {
            "success"
        } else {
            "failure"
        };
        rows.push(json!({
            "timestamp": call.at,
            "key": call.key_id,
            "model": call.model,
            "status_code": call.status_code,
            "latency_ms": if status == "success" { 120 } else { 450 },
            "status": status
        }));
    }
    Ok(Json(Value::Array(rows)))
}

async fn v2_stats_attention_keys(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Query(query): Query<AttentionKeysQuery>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let guard = state.inner.read().await;
    let items = guard.attention_keys(query.status_code, query.limit.unwrap_or(20).max(1));
    Ok(Json(
        serde_json::to_value(items).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

async fn v1_pool_status(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Query(query): Query<PoolStatusQuery>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let guard = state.inner.read().await;
    let payload = guard.pool_status(query.limit.unwrap_or(10).max(1));
    Ok(Json(
        serde_json::to_value(payload).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

async fn v1_pool_strategy_put(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(body): Json<PoolStrategyBody>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let strategy = body
        .strategy
        .parse::<PoolStrategy>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut guard = state.inner.write().await;
    guard.set_pool_strategy(strategy);
    Ok(Json(json!({
        "success": true,
        "strategy": strategy.as_str(),
        "pool": guard.pool_status(10)
    })))
}

async fn v2_config_get(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let guard = state.inner.read().await;
    Ok(Json(
        serde_json::to_value(&guard.config_map).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

async fn v2_config_put(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(body): Json<HashMap<String, Value>>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;

    for (k, v) in body {
        guard.config_map.insert(k, v);
    }

    if let Some(value) = guard.config_map.get("AUTH_TOKEN").and_then(Value::as_str) {
        guard.runtime_config.auth_token = value.to_string();
    }
    if let Some(value) = guard.config_map.get("ALLOWED_TOKENS") {
        guard.runtime_config.allowed_tokens = as_string_list(value);
    }
    if let Some(value) = guard
        .config_map
        .get("MAX_FAILURES")
        .and_then(Value::as_u64)
        .map(|v| v as u32)
    {
        guard.runtime_config.max_failures = value;
        let max_failures = guard.runtime_config.max_failures;
        let cooldown_secs = guard.runtime_config.cooldown_secs;
        guard.key_pool.set_limits(max_failures, cooldown_secs);
    }
    if let Some(value) = guard
        .config_map
        .get("COOLDOWN_SECONDS")
        .and_then(Value::as_u64)
    {
        guard.runtime_config.cooldown_secs = value;
        let max_failures = guard.runtime_config.max_failures;
        let cooldown_secs = guard.runtime_config.cooldown_secs;
        guard.key_pool.set_limits(max_failures, cooldown_secs);
    }
    if let Some(value) = guard
        .config_map
        .get("POOL_STRATEGY")
        .and_then(Value::as_str)
    {
        let strategy = value
            .parse::<PoolStrategy>()
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        guard.set_pool_strategy(strategy);
    }
    if let Some(value) = guard
        .config_map
        .get("MODEL_POOL_STRATEGY")
        .and_then(Value::as_str)
    {
        let strategy = value
            .parse::<ModelPoolStrategy>()
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        guard.runtime_config.model_pool_strategy = strategy;
    }
    if let Some(value) = guard
        .config_map
        .get("MODEL_POOL_SCOPE")
        .and_then(Value::as_str)
    {
        let scope = value
            .parse::<ModelPoolScope>()
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        guard.runtime_config.model_pool_scope = scope;
    }
    if let Some(value) = guard
        .config_map
        .get("ENABLE_SCHEDULER")
        .and_then(Value::as_bool)
    {
        guard.set_scheduler_running(value);
    }
    if let Some(value) = guard.config_map.get("API_KEYS") {
        let keys = as_string_list(value);
        guard.key_pool.upsert_keys(&keys);
    }

    Ok(Json(json!({
        "success": true,
        "config": guard.config_map
    })))
}

async fn v2_config_reset(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    let api_keys = guard.key_pool.raw_keys();
    let auth_token = guard.runtime_config.auth_token.clone();
    let allowed_tokens = guard.runtime_config.allowed_tokens.clone();
    let max_failures = guard.runtime_config.max_failures;
    let cooldown_secs = guard.runtime_config.cooldown_secs;
    let compat_mode = guard.runtime_config.compat_mode;
    let pool_strategy = guard.runtime_config.pool_strategy;
    let model_pool_strategy = guard.runtime_config.model_pool_strategy;
    let model_pool_scope = guard.runtime_config.model_pool_scope;

    guard.config_map.insert(
        "API_KEYS".to_string(),
        Value::Array(api_keys.into_iter().map(Value::String).collect()),
    );
    guard
        .config_map
        .insert("AUTH_TOKEN".to_string(), Value::String(auth_token));
    guard.config_map.insert(
        "ALLOWED_TOKENS".to_string(),
        Value::Array(
            allowed_tokens
                .iter()
                .map(|item| Value::String(item.clone()))
                .collect(),
        ),
    );
    guard.config_map.insert(
        "MAX_FAILURES".to_string(),
        Value::Number(max_failures.into()),
    );
    guard.config_map.insert(
        "COOLDOWN_SECONDS".to_string(),
        Value::Number(cooldown_secs.into()),
    );
    guard
        .config_map
        .insert("COMPAT_MODE".to_string(), Value::Bool(compat_mode));
    guard.config_map.insert(
        "POOL_STRATEGY".to_string(),
        Value::String(pool_strategy.as_str().to_string()),
    );
    guard.config_map.insert(
        "MODEL_POOL_STRATEGY".to_string(),
        Value::String(model_pool_strategy.as_str().to_string()),
    );
    guard.config_map.insert(
        "MODEL_POOL_SCOPE".to_string(),
        Value::String(model_pool_scope.as_str().to_string()),
    );

    Ok(Json(json!({
        "success": true,
        "config": guard.config_map
    })))
}

async fn v2_config_schema(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let guard = state.inner.read().await;
    Ok(Json(
        serde_json::to_value(guard.config_schema())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

async fn v2_config_ui_models(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let guard = state.inner.read().await;
    Ok(Json(json!({
        "data": guard.ui_models()
    })))
}

async fn v2_config_keys_add(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(body): Json<BulkTextBody>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    let mut current = guard.key_pool.raw_keys();
    let mut merged = current.clone();
    merged.extend(normalize_bulk_values(body));
    merged.sort();
    merged.dedup();
    guard.key_pool.upsert_keys(&merged);
    guard.config_map.insert(
        "API_KEYS".to_string(),
        Value::Array(
            merged
                .iter()
                .map(|item| Value::String(item.clone()))
                .collect(),
        ),
    );
    current.sort();
    current.dedup();
    let added = merged.len().saturating_sub(current.len());
    Ok(Json(json!({
        "success": true,
        "added": added,
        "total": merged.len()
    })))
}

async fn v2_config_keys_delete(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(body): Json<BulkTextBody>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    let target_keys = normalize_bulk_values(body);
    if target_keys.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    guard.key_pool.remove_by_raw_keys(&target_keys);
    let keys = guard.key_pool.raw_keys();
    guard.config_map.insert(
        "API_KEYS".to_string(),
        Value::Array(
            keys.iter()
                .map(|item| Value::String(item.clone()))
                .collect(),
        ),
    );
    Ok(Json(json!({
        "success": true,
        "deleted": target_keys.len(),
        "total": keys.len()
    })))
}

async fn legacy_delete_key(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    guard.key_pool.remove_by_raw_keys(&[key.clone()]);
    let keys = guard.key_pool.raw_keys();
    guard.config_map.insert(
        "API_KEYS".to_string(),
        Value::Array(
            keys.iter()
                .map(|item| Value::String(item.clone()))
                .collect(),
        ),
    );
    Ok(Json(json!({
        "success": true,
        "message": "key deleted"
    })))
}

async fn v2_config_proxies_add(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(body): Json<BulkTextBody>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    let mut current = guard
        .config_map
        .get("PROXIES")
        .map(as_string_list)
        .unwrap_or_default();
    current.extend(normalize_bulk_values(body));
    current.sort();
    current.dedup();
    guard.config_map.insert(
        "PROXIES".to_string(),
        Value::Array(
            current
                .iter()
                .map(|item| Value::String(item.clone()))
                .collect(),
        ),
    );
    Ok(Json(json!({
        "success": true,
        "total": current.len()
    })))
}

async fn v2_config_proxies_delete(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(body): Json<BulkTextBody>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    let targets = normalize_bulk_values(body);
    let mut current = guard
        .config_map
        .get("PROXIES")
        .map(as_string_list)
        .unwrap_or_default();
    current.retain(|proxy| !targets.contains(proxy));
    guard.config_map.insert(
        "PROXIES".to_string(),
        Value::Array(
            current
                .iter()
                .map(|item| Value::String(item.clone()))
                .collect(),
        ),
    );
    Ok(Json(json!({
        "success": true,
        "total": current.len()
    })))
}

async fn v2_proxy_check(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(body): Json<ProxyCheckBody>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    if body.use_cache {
        if let Some(cached) = guard.proxy_cache.get(&body.proxy) {
            if cached.expires_at >= Utc::now() {
                return Ok(Json(json!({
                    "proxy": body.proxy,
                    "isAvailable": cached.is_available,
                    "responseTime": cached.response_time,
                    "errorMessage": cached.error_message,
                    "checkedAt": cached.checked_at
                })));
            }
        }
    }

    let result = proxy_check_result(&body.proxy);
    guard.upsert_proxy_cache(&result, 300);
    Ok(Json(
        serde_json::to_value(result).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

async fn v2_proxy_check_all(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(body): Json<ProxyCheckAllBody>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    let mut results = Vec::new();
    let _max_concurrent = body.max_concurrent.unwrap_or(5);
    for proxy in body.proxies {
        if body.use_cache {
            if let Some(cached) = guard.proxy_cache.get(&proxy) {
                if cached.expires_at >= Utc::now() {
                    results.push(ProxyCheckResultV2 {
                        proxy: proxy.clone(),
                        is_available: cached.is_available,
                        response_time: cached.response_time,
                        error_message: cached.error_message.clone(),
                        checked_at: cached.checked_at,
                    });
                    continue;
                }
            }
        }
        let result = proxy_check_result(&proxy);
        guard.upsert_proxy_cache(&result, 300);
        results.push(result);
    }
    Ok(Json(
        serde_json::to_value(results).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

async fn v2_proxy_cache_stats(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let guard = state.inner.read().await;
    let stats = guard.proxy_cache_stats(Utc::now());
    Ok(Json(
        serde_json::to_value(stats).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

async fn v2_proxy_cache_clear(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    guard.clear_proxy_cache();
    Ok(Json(json!({
        "success": true,
        "message": "proxy cache cleared"
    })))
}

async fn v2_scheduler_start(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    guard.set_scheduler_running(true);
    Ok(Json(json!({
        "running": true,
        "message": "scheduler started",
        "updatedAt": guard.scheduler_updated_at
    })))
}

async fn v2_scheduler_stop(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    guard.set_scheduler_running(false);
    Ok(Json(json!({
        "running": false,
        "message": "scheduler stopped",
        "updatedAt": guard.scheduler_updated_at
    })))
}

async fn v2_scheduler_status(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let guard = state.inner.read().await;
    Ok(Json(json!({
        "running": guard.scheduler_running,
        "updatedAt": guard.scheduler_updated_at
    })))
}

async fn v2_logs(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Query(query): Query<LogsQuery>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let guard = state.inner.read().await;
    let mut logs = guard.logs.clone();

    if let Some(search) = query.key_search.as_ref() {
        let text = search.to_lowercase();
        logs.retain(|item| item.masked_key.to_lowercase().contains(&text));
    }
    if let Some(search) = query.error_search.as_ref() {
        let text = search.to_lowercase();
        logs.retain(|item| {
            item.error_type.to_lowercase().contains(&text)
                || item.detail.to_lowercase().contains(&text)
        });
    }
    if let Some(search) = query.error_code_search.as_ref() {
        logs.retain(|item| item.status_code.to_string().contains(search));
    }
    if let Some(start_date) = query
        .start_date
        .as_ref()
        .and_then(|value| parse_datetime(value))
    {
        logs.retain(|item| item.request_at >= start_date);
    }
    if let Some(end_date) = query
        .end_date
        .as_ref()
        .and_then(|value| parse_datetime(value))
    {
        logs.retain(|item| item.request_at <= end_date);
    }

    let sort_field = query.sort_by.clone().unwrap_or_else(|| "id".to_string());
    let sort_order = query
        .sort_order
        .clone()
        .unwrap_or_else(|| "desc".to_string())
        .to_lowercase();
    if sort_field == "id" {
        logs.sort_by_key(|item| item.id);
    } else if sort_field == "request_at" {
        logs.sort_by_key(|item| item.request_at);
    }
    if sort_order != "asc" {
        logs.reverse();
    }

    let limit = query.limit.unwrap_or(20).max(1);
    let offset = query.offset.unwrap_or(0);
    let total = logs.len();
    let end = (offset + limit).min(total);
    let paged = if offset < total {
        logs[offset..end].to_vec()
    } else {
        Vec::new()
    };
    let payload = LogListResponseV2 {
        logs: paged,
        total,
        limit,
        offset,
    };

    Ok(Json(
        serde_json::to_value(payload).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

async fn v2_logs_lookup(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Query(query): Query<LogsLookupQuery>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let guard = state.inner.read().await;
    let mut logs = guard.logs.clone();
    if let Some(id) = query.id {
        logs.retain(|item| item.id == id);
    }
    if let Some(key) = query.key.as_ref() {
        let text = key.to_lowercase();
        logs.retain(|item| item.masked_key.to_lowercase().contains(&text));
    }
    if let Some(code) = query.status_code {
        logs.retain(|item| item.status_code == code);
    }
    if let Some(timestamp) = query
        .timestamp
        .as_ref()
        .and_then(|value| parse_datetime(value))
    {
        logs.sort_by_key(|item| (item.request_at - timestamp).num_seconds().unsigned_abs());
    } else {
        logs.sort_by_key(|item| item.id);
        logs.reverse();
    }
    let result = logs.first().cloned().map(|log| build_log_detail(&log));
    Ok(Json(json!({ "log": result })))
}

async fn v2_logs_detail(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let guard = state.inner.read().await;
    let log = guard.logs.iter().find(|item| item.id == id).cloned();
    match log {
        Some(item) => Ok(Json(json!({ "log": build_log_detail(&item) }))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn v2_logs_delete_one(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Result<StatusCode, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    let before = guard.logs.len();
    guard.logs.retain(|item| item.id != id);
    if before == guard.logs.len() {
        Err(StatusCode::NOT_FOUND)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

async fn v2_logs_bulk_delete(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(body): Json<DeleteBulkBody>,
) -> Result<StatusCode, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    guard.logs.retain(|item| !body.ids.contains(&item.id));
    Ok(StatusCode::NO_CONTENT)
}

async fn v2_logs_all_delete(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    guard.logs.clear();
    Ok(StatusCode::NO_CONTENT)
}

async fn v2_models(State(state): State<SharedState>, req: Request) -> Response {
    let headers = req.headers().clone();
    if user_guard(&headers, req.uri().query(), &state)
        .await
        .is_err()
    {
        return openai_error_for_status(
            StatusCode::UNAUTHORIZED,
            "Missing or invalid Authorization header.",
        );
    }
    let guard = state.inner.read().await;
    Json(json!({
        "object": "list",
        "data": guard.active_models
    }))
    .into_response()
}

async fn v2_chat_completions(State(state): State<SharedState>, req: Request) -> Response {
    let headers = req.headers().clone();
    let query = req.uri().query();
    if user_guard(&headers, query, &state).await.is_err() {
        return openai_error_for_status(
            StatusCode::UNAUTHORIZED,
            "Missing or invalid Authorization header.",
        );
    }

    let request_strategy = match parse_request_pool_strategy(&headers) {
        Ok(strategy) => strategy,
        Err(_) => {
            return openai_error_for_status(
                StatusCode::BAD_REQUEST,
                "Invalid X-Pool-Strategy header value.",
            )
        }
    };
    let user_token_context = bearer_token(&headers)
        .or_else(|| api_key_token(&headers))
        .or_else(|| query_key_token(query));

    let now = Utc::now();
    let mut guard = state.inner.write().await;
    let strategy = request_strategy.unwrap_or(guard.runtime_config.pool_strategy);
    let key = guard
        .key_pool
        .next_available_key_with_strategy(now, strategy);
    let Some(active_key) = key else {
        return openai_error_for_status(
            StatusCode::SERVICE_UNAVAILABLE,
            "No healthy API key is currently available.",
        );
    };
    drop(guard);

    let (parts, body) = req.into_parts();
    let mut bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return openai_error_for_status(
                StatusCode::BAD_REQUEST,
                "Invalid request body payload.",
            )
        }
    };

    let mut resolved_model = "unknown".to_string();

    // Rewrite model name in body if model pool applies
    if let Ok(mut json_body) = serde_json::from_slice::<serde_json::Value>(&bytes) {
        if let Some(model_val) = json_body
            .get("model")
            .and_then(|v| v.as_str())
            .map(String::from)
        {
            let mut guard = state.inner.write().await;
            let resolved =
                guard.resolve_model_alias_with_context(&model_val, user_token_context.as_deref());
            resolved_model = resolved.clone();
            if resolved != model_val {
                json_body["model"] = serde_json::Value::String(resolved);
                bytes = serde_json::to_vec(&json_body)
                    .unwrap_or(bytes.to_vec())
                    .into();
            } else {
                resolved_model = model_val;
            }
        }
    };

    let mut new_headers = sanitize_upstream_headers(&parts.headers);
    if GeminiAdapter::apply_openai_auth(&mut new_headers, &active_key).is_err() {
        return openai_error_for_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to apply upstream authentication.",
        );
    }

    let original_query = parts.uri.query().unwrap_or("");
    let forward_query = strip_auth_query_params(original_query);
    let url = GeminiAdapter::openai_chat_completions_url(&forward_query);

    let upstream_req = state
        .http_client
        .request(parts.method, &url)
        .headers(new_headers)
        .body(bytes)
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    let upstream_req = match upstream_req {
        Ok(req) => req,
        Err(_) => {
            return openai_error_for_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to build upstream request.",
            )
        }
    };

    let upstream_res = state.http_client.execute(upstream_req).await;
    let upstream_res = match upstream_res {
        Ok(res) => res,
        Err(err) => {
            let message = "Failed to connect to upstream provider.".to_string();
            let now = Utc::now();
            let mut guard = state.inner.write().await;
            let key_id = guard
                .key_pool
                .id_by_raw_key(&active_key)
                .unwrap_or_else(|| mask_key(&active_key));
            guard.key_pool.mark_failure(&active_key, now);
            guard.record_call(
                key_id,
                resolved_model.clone(),
                StatusCode::BAD_GATEWAY.as_u16(),
                now,
            );

            let next_log_id = guard.logs.last().map(|item| item.id + 1).unwrap_or(1);
            guard.logs.push(LogRecordV2 {
                id: next_log_id,
                masked_key: mask_key(&active_key),
                error_type: runtime_error_type(StatusCode::BAD_GATEWAY).to_string(),
                status_code: StatusCode::BAD_GATEWAY.as_u16(),
                model: resolved_model.clone(),
                request_at: now,
                detail: format!("{message} {}", err),
            });
            if guard.logs.len() > 20_000 {
                let drop_count = guard.logs.len().saturating_sub(10_000);
                guard.logs.drain(0..drop_count);
            }

            return openai_error_for_status(StatusCode::BAD_GATEWAY, message);
        }
    };

    let status = upstream_res.status();
    if !status.is_success() {
        let upstream_body = upstream_res
            .bytes()
            .await
            .unwrap_or_else(|_| bytes::Bytes::new());
        let message = parse_upstream_error_message(&upstream_body)
            .unwrap_or_else(|| format!("Upstream provider returned an error (status {}).", status));

        let now = Utc::now();
        let mut guard = state.inner.write().await;
        let key_id = guard
            .key_pool
            .id_by_raw_key(&active_key)
            .unwrap_or_else(|| mask_key(&active_key));
        if should_penalize_key_failure_status(status) {
            guard.key_pool.mark_failure(&active_key, now);
        }
        guard.record_call(key_id, resolved_model.clone(), status.as_u16(), now);

        let next_log_id = guard.logs.last().map(|item| item.id + 1).unwrap_or(1);
        guard.logs.push(LogRecordV2 {
            id: next_log_id,
            masked_key: mask_key(&active_key),
            error_type: runtime_error_type(status).to_string(),
            status_code: status.as_u16(),
            model: resolved_model.clone(),
            request_at: now,
            detail: message.clone(),
        });
        if guard.logs.len() > 20_000 {
            let drop_count = guard.logs.len().saturating_sub(10_000);
            guard.logs.drain(0..drop_count);
        }

        return openai_error_for_status(status, message);
    }

    {
        let now = Utc::now();
        let mut guard = state.inner.write().await;
        let key_id = guard
            .key_pool
            .id_by_raw_key(&active_key)
            .unwrap_or_else(|| mask_key(&active_key));
        guard.key_pool.mark_success(&active_key);
        guard.record_call(key_id, resolved_model.clone(), status.as_u16(), now);
    }

    let mut res_builder = Response::builder().status(status);
    for (k, v) in upstream_res.headers() {
        if k != "transfer-encoding" {
            res_builder = res_builder.header(k, v);
        }
    }

    let stream = upstream_res
        .bytes_stream()
        .map(|res: Result<bytes::Bytes, reqwest::Error>| {
            res.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        });

    match res_builder.body(Body::from_stream(stream)) {
        Ok(response) => response,
        Err(_) => openai_error_for_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to build response payload.",
        ),
    }
}

async fn v1beta_gemini_proxy(
    State(state): State<SharedState>,
    req: Request,
) -> Result<Response, StatusCode> {
    let headers = req.headers().clone();
    let query = req.uri().query();
    user_guard(&headers, query, &state).await?;
    let user_token_context = bearer_token(&headers)
        .or_else(|| api_key_token(&headers))
        .or_else(|| query_key_token(query));

    let request_strategy = parse_request_pool_strategy(&headers)?;

    let now = Utc::now();
    let mut guard = state.inner.write().await;
    let strategy = request_strategy.unwrap_or(guard.runtime_config.pool_strategy);
    let key = guard
        .key_pool
        .next_available_key_with_strategy(now, strategy);
    let Some(active_key) = key else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };
    drop(guard);

    // Extract the full path+query from the incoming URI
    let incoming_path = req.uri().path().trim_start_matches('/').to_string();
    // Strip leading version prefixes so we forward the sub-path correctly
    // Also resolve model alias to real model name in path
    let sub_path = if incoming_path.starts_with("v1beta/") {
        incoming_path["v1beta/".len()..].to_string()
    } else if incoming_path.starts_with("v1/models/") {
        "models/".to_string() + &incoming_path["v1/models/".len()..]
    } else {
        incoming_path
    };
    let mut resolved_model = "unknown".to_string();
    // Apply model pool resolution to path segment
    // Path looks like: models/<model-name>:generateContent
    let sub_path = {
        let mut guard = state.inner.write().await;
        if sub_path.starts_with("models/") {
            let rest = &sub_path["models/".len()..];
            // Split on ':' to separate model name from method
            let (model_part, method_part) = if let Some(colon_pos) = rest.find(':') {
                (&rest[..colon_pos], Some(&rest[colon_pos..]))
            } else {
                (rest.as_ref(), None)
            };
            let resolved =
                guard.resolve_model_alias_with_context(model_part, user_token_context.as_deref());
            resolved_model = resolved.clone();
            match method_part {
                Some(m) => format!("models/{}{}", resolved, m),
                None => format!("models/{}", resolved),
            }
        } else {
            sub_path
        }
    };
    let original_query = req.uri().query().unwrap_or("");
    let forward_query = strip_auth_query_params(original_query);
    let url = GeminiAdapter::native_proxy_url(&sub_path, &forward_query);

    let (parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut new_headers = sanitize_upstream_headers(&parts.headers);
    if GeminiAdapter::apply_native_api_key(&mut new_headers, &active_key).is_err() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let upstream_req = state
        .http_client
        .request(parts.method, &url)
        .headers(new_headers)
        .body(bytes)
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let upstream_res = match state.http_client.execute(upstream_req).await {
        Ok(res) => res,
        Err(err) => {
            let now = Utc::now();
            let mut guard = state.inner.write().await;
            let key_id = guard
                .key_pool
                .id_by_raw_key(&active_key)
                .unwrap_or_else(|| mask_key(&active_key));
            guard.key_pool.mark_failure(&active_key, now);
            guard.record_call(
                key_id,
                resolved_model.clone(),
                StatusCode::BAD_GATEWAY.as_u16(),
                now,
            );

            let next_log_id = guard.logs.last().map(|item| item.id + 1).unwrap_or(1);
            guard.logs.push(LogRecordV2 {
                id: next_log_id,
                masked_key: mask_key(&active_key),
                error_type: runtime_error_type(StatusCode::BAD_GATEWAY).to_string(),
                status_code: StatusCode::BAD_GATEWAY.as_u16(),
                model: resolved_model.clone(),
                request_at: now,
                detail: format!("Failed to connect to upstream provider. {}", err),
            });
            if guard.logs.len() > 20_000 {
                let drop_count = guard.logs.len().saturating_sub(10_000);
                guard.logs.drain(0..drop_count);
            }
            return Err(StatusCode::BAD_GATEWAY);
        }
    };

    let status = upstream_res.status();
    if !status.is_success() {
        let upstream_body = upstream_res
            .bytes()
            .await
            .unwrap_or_else(|_| bytes::Bytes::new());
        let message = parse_upstream_error_message(&upstream_body)
            .unwrap_or_else(|| format!("Upstream provider returned an error (status {}).", status));

        let now = Utc::now();
        let mut guard = state.inner.write().await;
        let key_id = guard
            .key_pool
            .id_by_raw_key(&active_key)
            .unwrap_or_else(|| mask_key(&active_key));
        if should_penalize_key_failure_status(status) {
            guard.key_pool.mark_failure(&active_key, now);
        }
        guard.record_call(key_id, resolved_model.clone(), status.as_u16(), now);

        let next_log_id = guard.logs.last().map(|item| item.id + 1).unwrap_or(1);
        guard.logs.push(LogRecordV2 {
            id: next_log_id,
            masked_key: mask_key(&active_key),
            error_type: runtime_error_type(status).to_string(),
            status_code: status.as_u16(),
            model: resolved_model.clone(),
            request_at: now,
            detail: message,
        });
        if guard.logs.len() > 20_000 {
            let drop_count = guard.logs.len().saturating_sub(10_000);
            guard.logs.drain(0..drop_count);
        }

        return Ok(Response::builder()
            .status(status)
            .body(Body::from(upstream_body))
            .unwrap_or_else(|_| Response::new(Body::empty())));
    }

    {
        let now = Utc::now();
        let mut guard = state.inner.write().await;
        let key_id = guard
            .key_pool
            .id_by_raw_key(&active_key)
            .unwrap_or_else(|| mask_key(&active_key));
        guard.key_pool.mark_success(&active_key);
        guard.record_call(key_id, resolved_model.clone(), status.as_u16(), now);
    }

    let mut res_builder = Response::builder().status(status);
    for (k, v) in upstream_res.headers() {
        if k != "transfer-encoding" {
            res_builder = res_builder.header(k, v);
        }
    }
    let stream = upstream_res
        .bytes_stream()
        .map(|res: Result<bytes::Bytes, reqwest::Error>| {
            res.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        });
    Ok(res_builder.body(Body::from_stream(stream)).unwrap())
}

async fn legacy_keys(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Query(query): Query<KeysQuery>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    guard.key_pool.recover_cooldown(Utc::now());
    let summary = guard.key_pool.summary();
    let mut items = guard.key_pool.snapshot();
    let search = query.search.clone().unwrap_or_default().to_lowercase();
    let status = query
        .status
        .clone()
        .unwrap_or_else(|| "all".to_string())
        .to_lowercase();
    if !search.is_empty() {
        items.retain(|item| {
            item.id.to_lowercase().contains(&search)
                || item.key.to_lowercase().contains(&search)
                || item.masked_key.to_lowercase().contains(&search)
        });
    }
    items.retain(|item| status_matches(item, &status));
    if let Some(min_failure_count) = query.min_failure_count {
        items.retain(|item| item.failure_count >= min_failure_count);
    }

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).max(1);
    let total_items = items.len();
    let total_pages = total_items.div_ceil(limit).max(1);
    let start = (page - 1) * limit;
    let end = (start + limit).min(total_items);
    let paged = if start < total_items {
        items[start..end].to_vec()
    } else {
        Vec::new()
    };

    let legacy_items = paged
        .into_iter()
        .map(|item| {
            let legacy_status = if matches!(item.status, KeyStatus::Invalid) {
                "invalid"
            } else {
                "valid"
            };
            json!({
                "key": item.key,
                "masked_key": item.masked_key,
                "fail_count": item.failure_count,
                "status": legacy_status
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(json!({
        "items": legacy_items,
        "total_items": total_items,
        "total_pages": total_pages,
        "current_page": page,
        "page_size": limit,
        "search": query.search.unwrap_or_default(),
        "status": status,
        "summary": {
            "valid_count": summary.active + summary.cooldown,
            "invalid_count": summary.invalid,
            "total_count": summary.total
        }
    })))
}

async fn legacy_keys_all(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    guard.key_pool.recover_cooldown(Utc::now());
    let items = guard.key_pool.snapshot();
    let valid_keys = items
        .iter()
        .filter(|item| matches!(item.status, KeyStatus::Active | KeyStatus::Cooldown))
        .map(|item| item.key.clone())
        .collect::<Vec<_>>();
    let invalid_keys = items
        .iter()
        .filter(|item| matches!(item.status, KeyStatus::Invalid))
        .map(|item| item.key.clone())
        .collect::<Vec<_>>();
    Ok(Json(json!({
        "valid_keys": valid_keys,
        "invalid_keys": invalid_keys
    })))
}

async fn legacy_verify_key(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    if !key.starts_with("AIza") {
        return Ok(Json(json!({
            "success": false,
            "status": "invalid",
            "error": "invalid key format"
        })));
    }

    if let Some(id) = guard.key_pool.id_by_raw_key(&key) {
        guard.key_pool.reset_failures(Some(&[id]));
    }
    Ok(Json(json!({
        "success": true,
        "status": "valid"
    })))
}

async fn legacy_reset_key_fail_count(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    let target_id = guard.key_pool.id_by_raw_key(&key);
    if let Some(id) = target_id {
        guard.key_pool.reset_failures(Some(&[id]));
        Ok(Json(json!({
            "success": true
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn legacy_verify_selected_keys(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(body): Json<BulkTextBody>,
) -> Result<Json<Value>, StatusCode> {
    admin_guard(&headers, &state).await?;
    let mut guard = state.inner.write().await;
    let keys = normalize_bulk_values(body);
    let mut success = 0_usize;
    let mut failed = 0_usize;
    for key in keys {
        if key.starts_with("AIza") {
            if let Some(id) = guard.key_pool.id_by_raw_key(&key) {
                guard.key_pool.reset_failures(Some(&[id]));
                success += 1;
            } else {
                failed += 1;
            }
        } else {
            failed += 1;
        }
    }
    Ok(Json(json!({
        "success": true,
        "successCount": success,
        "failedCount": failed
    })))
}

async fn compat_login(
    State(state): State<SharedState>,
    Json(body): Json<LoginBody>,
) -> Result<Response, StatusCode> {
    let response = v2_login(State(state), Json(body)).await?;
    Ok(deprecated_response(response))
}

async fn compat_logout(State(state): State<SharedState>) -> Result<Response, StatusCode> {
    let response = v2_logout(State(state)).await?;
    Ok(deprecated_response(response))
}

async fn compat_session_status(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let response = v2_session_status(State(state), headers)
        .await?
        .into_response();
    Ok(deprecated_response(response))
}

async fn compat_dashboard_overview(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let response = v2_dashboard_overview(State(state), headers)
        .await?
        .into_response();
    Ok(deprecated_response(response))
}

async fn compat_keys(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Query(query): Query<KeysQuery>,
) -> Result<Response, StatusCode> {
    let response = legacy_keys(State(state), headers, Query(query))
        .await?
        .into_response();
    Ok(deprecated_response(response))
}

async fn compat_keys_actions(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(body): Json<KeysActionBody>,
) -> Result<Response, StatusCode> {
    let response = v2_keys_actions(State(state), headers, Json(body))
        .await?
        .into_response();
    Ok(deprecated_response(response))
}

async fn compat_config_schema(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let response = v2_config_schema(State(state), headers)
        .await?
        .into_response();
    Ok(deprecated_response(response))
}

async fn compat_logs(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Query(query): Query<LogsQuery>,
) -> Result<Response, StatusCode> {
    let response = v2_logs(State(state), headers, Query(query))
        .await?
        .into_response();
    Ok(deprecated_response(response))
}

async fn compat_logs_detail(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Result<Response, StatusCode> {
    let response = v2_logs_detail(State(state), headers, Path(id))
        .await?
        .into_response();
    Ok(deprecated_response(response))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use gateway_core::types::{ModelPoolScope, ModelPoolStrategy, PoolStrategy};
    use tower::ServiceExt;

    use super::build_router;
    use crate::{config::ServerConfig, state::SharedState};

    fn test_state() -> SharedState {
        SharedState::new(ServerConfig {
            bind_host: "127.0.0.1".to_string(),
            port_start: 18080,
            port_end: 18099,
            auth_token: "sk-admin-test".to_string(),
            allowed_tokens: vec!["sk-user-test".to_string()],
            api_keys: vec!["AIzaSy_test_1".to_string(), "AIzaSy_test_2".to_string()],
            session_cookie_name: "gb_session".to_string(),
            compat_mode: false,
            max_failures: 3,
            cooldown_secs: 60,
            pool_strategy: PoolStrategy::RoundRobin,
            thinking_models: vec![],
            image_models: vec![],
            search_models: vec![],
            filtered_models: vec![],
            url_context_models: vec![],
            model_pools: std::collections::HashMap::new(),
            model_pool_strategy: ModelPoolStrategy::RoundRobin,
            model_pool_scope: ModelPoolScope::Global,
        })
    }

    fn auth_request(uri: &str) -> Request<Body> {
        Request::builder()
            .uri(uri)
            .header("Authorization", "Bearer sk-admin-test")
            .body(Body::empty())
            .expect("request")
    }

    #[tokio::test]
    async fn v1_session_route_should_exist() {
        let app = build_router(test_state());
        let response = app
            .oneshot(auth_request("/api/v1/session/status"))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn v1_models_unauthorized_returns_openai_error_payload() {
        let app = build_router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/models")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let body = serde_json::from_slice::<serde_json::Value>(&bytes).expect("json body");
        assert_eq!(body["error"]["code"], "invalid_api_key");
    }

    #[tokio::test]
    async fn v1_chat_unauthorized_returns_openai_error_payload() {
        let app = build_router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "model": "gemini-2.5-flash",
                            "messages": [{ "role": "user", "content": "hello" }]
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let body = serde_json::from_slice::<serde_json::Value>(&bytes).expect("json body");
        assert_eq!(body["error"]["type"], "authentication_error");
    }

    #[tokio::test]
    async fn v2_api_routes_should_return_gone() {
        let app = build_router(test_state());
        let response = app
            .oneshot(auth_request("/api/v2/session/status"))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::GONE);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let body = serde_json::from_slice::<serde_json::Value>(&bytes).expect("json body");
        assert!(body.get("migrationTo").is_some());
    }

    #[tokio::test]
    async fn v2_proxy_routes_should_return_gone() {
        let app = build_router(test_state());
        let response = app
            .oneshot(auth_request("/v2/models"))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::GONE);
    }

    #[tokio::test]
    async fn invalid_pool_strategy_header_should_be_bad_request() {
        let app = build_router(test_state());
        let body = serde_json::json!({
            "model": "gemini-2.5-flash",
            "messages": [{ "role": "user", "content": "hello" }]
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .header("Authorization", "Bearer sk-user-test")
                    .header("Content-Type", "application/json")
                    .header("X-Pool-Strategy", "not-a-strategy")
                    .body(Body::from(body.to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn v1_chat_authorized_bearer_should_not_fail_user_guard() {
        let app = build_router(test_state());
        let body = serde_json::json!({
            "model": "gemini-2.5-flash",
            "messages": [{ "role": "user", "content": "hello" }]
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .header("Authorization", "Bearer sk-user-test")
                    .header("Content-Type", "application/json")
                    .body(Body::from(body.to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn sanitize_upstream_headers_should_remove_sensitive_and_length_headers() {
        let request = Request::builder()
            .uri("/v1/chat/completions")
            .header("authorization", "Bearer sk-user-test")
            .header("x-goog-api-key", "AIzaSy_test")
            .header("host", "127.0.0.1:18080")
            .header("content-length", "999")
            .header("content-type", "application/json")
            .body(Body::empty())
            .expect("request");

        let sanitized = super::sanitize_upstream_headers(request.headers());
        assert!(sanitized.get("authorization").is_none());
        assert!(sanitized.get("x-goog-api-key").is_none());
        assert!(sanitized.get("host").is_none());
        assert!(sanitized.get("content-length").is_none());
        assert_eq!(
            sanitized
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or(""),
            "application/json"
        );
    }

    #[test]
    fn should_penalize_key_failure_statuses_only_for_upstream_or_auth_related_errors() {
        assert!(super::should_penalize_key_failure_status(
            StatusCode::UNAUTHORIZED
        ));
        assert!(super::should_penalize_key_failure_status(
            StatusCode::FORBIDDEN
        ));
        assert!(super::should_penalize_key_failure_status(
            StatusCode::TOO_MANY_REQUESTS
        ));
        assert!(super::should_penalize_key_failure_status(
            StatusCode::BAD_GATEWAY
        ));
        assert!(super::should_penalize_key_failure_status(
            StatusCode::SERVICE_UNAVAILABLE
        ));

        assert!(!super::should_penalize_key_failure_status(
            StatusCode::BAD_REQUEST
        ));
        assert!(!super::should_penalize_key_failure_status(
            StatusCode::NOT_FOUND
        ));
        assert!(!super::should_penalize_key_failure_status(
            StatusCode::UNPROCESSABLE_ENTITY
        ));
    }
}
