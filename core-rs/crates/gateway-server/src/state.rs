use std::sync::Arc;

use gateway_core::app_state::{AppStateModel, RuntimeConfig};
use tokio::sync::RwLock;

use crate::config::ServerConfig;

#[derive(Clone)]
pub struct SharedState {
    pub inner: Arc<RwLock<AppStateModel>>,
    pub session_cookie_name: String,
    pub http_client: reqwest::Client,
}

impl SharedState {
    pub fn new(config: ServerConfig) -> Self {
        let model = AppStateModel::new(
            RuntimeConfig {
                auth_token: config.auth_token.clone(),
                allowed_tokens: config.allowed_tokens.clone(),
                listen_addr: format!("{}:{}", config.bind_host, config.port_start),
                max_failures: config.max_failures,
                cooldown_secs: config.cooldown_secs,
                compat_mode: config.compat_mode,
                pool_strategy: config.pool_strategy,
                thinking_models: config.thinking_models,
                image_models: config.image_models,
                search_models: config.search_models,
                filtered_models: config.filtered_models,
                url_context_models: config.url_context_models,
                model_pools: config.model_pools,
                model_pool_strategy: config.model_pool_strategy,
                model_pool_scope: config.model_pool_scope,
            },
            config.api_keys,
        );
        let mut model = model;
        model.seed_demo_logs();

        Self {
            inner: Arc::new(RwLock::new(model)),
            session_cookie_name: config.session_cookie_name,
            http_client: reqwest::Client::builder()
                .pool_idle_timeout(std::time::Duration::from_secs(90))
                .build()
                .unwrap_or_default(),
        }
    }
}
