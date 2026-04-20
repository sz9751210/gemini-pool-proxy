use std::{
    collections::{BTreeMap, HashMap, HashSet},
    hash::{Hash, Hasher},
};

use chrono::{DateTime, Duration as ChronoDuration, Utc};

use crate::{
    key_pool::KeyPool,
    types::{
        AttentionKeyItemV2, CallsSummaryV2, ConfigFieldRuleV2, ConfigFieldUiHintsV2,
        ConfigSchemaFieldV2, ConfigSchemaV2, ConfigSectionV2, DashboardCallsSummary,
        DashboardHealthV2, DashboardModelMetricV2, DashboardOverviewV2, DashboardStatusMetricV2,
        LogRecordV2, ModelPoolScope, ModelPoolStrategy, PoolStatus, PoolStrategy,
        ProxyCacheStatsV2, ProxyCheckResultV2, StatsDetailsV2, StatsPointV2,
    },
};

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub auth_token: String,
    pub allowed_tokens: Vec<String>,
    pub listen_addr: String,
    pub max_failures: u32,
    pub cooldown_secs: u64,
    pub compat_mode: bool,
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

#[derive(Debug, Clone)]
pub struct AppStateModel {
    pub runtime_config: RuntimeConfig,
    pub key_pool: KeyPool,
    pub model_pool_cursors: HashMap<String, usize>,
    pub model_pool_usage: HashMap<String, HashMap<String, u64>>,
    pub model_pool_key_cycle_steps: HashMap<String, usize>,
    pub config_map: HashMap<String, serde_json::Value>,
    pub logs: Vec<LogRecordV2>,
    pub calls: Vec<CallRecord>,
    pub proxy_cache: HashMap<String, ProxyCacheEntry>,
    pub scheduler_running: bool,
    pub scheduler_updated_at: Option<DateTime<Utc>>,
    pub active_models: Vec<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct CallRecord {
    pub at: DateTime<Utc>,
    pub key_id: String,
    pub model: String,
    pub status_code: u16,
}

#[derive(Debug, Clone)]
pub struct ProxyCacheEntry {
    pub checked_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_available: bool,
    pub response_time: Option<f64>,
    pub error_message: Option<String>,
}

impl AppStateModel {
    pub fn new(runtime_config: RuntimeConfig, api_keys: Vec<String>) -> Self {
        let mut config_map = HashMap::new();
        config_map.insert(
            "API_KEYS".to_string(),
            serde_json::Value::Array(
                api_keys
                    .iter()
                    .map(|k| serde_json::Value::String(k.clone()))
                    .collect(),
            ),
        );
        config_map.insert(
            "AUTH_TOKEN".to_string(),
            serde_json::Value::String(runtime_config.auth_token.clone()),
        );
        config_map.insert(
            "ALLOWED_TOKENS".to_string(),
            serde_json::Value::Array(
                runtime_config
                    .allowed_tokens
                    .iter()
                    .map(|k| serde_json::Value::String(k.clone()))
                    .collect(),
            ),
        );
        config_map.insert(
            "MAX_FAILURES".to_string(),
            serde_json::Value::Number(runtime_config.max_failures.into()),
        );
        config_map.insert(
            "COOLDOWN_SECONDS".to_string(),
            serde_json::Value::Number(runtime_config.cooldown_secs.into()),
        );
        config_map.insert(
            "COMPAT_MODE".to_string(),
            serde_json::Value::Bool(runtime_config.compat_mode),
        );
        config_map.insert(
            "POOL_STRATEGY".to_string(),
            serde_json::Value::String(runtime_config.pool_strategy.as_str().to_string()),
        );
        config_map.insert(
            "MODEL_POOL_STRATEGY".to_string(),
            serde_json::Value::String(runtime_config.model_pool_strategy.as_str().to_string()),
        );
        config_map.insert(
            "MODEL_POOL_SCOPE".to_string(),
            serde_json::Value::String(runtime_config.model_pool_scope.as_str().to_string()),
        );
        config_map.insert(
            "MODEL_NAME".to_string(),
            serde_json::Value::String("gemini-2.5-flash".to_string()),
        );
        config_map.insert(
            "THINKING_MODELS".to_string(),
            serde_json::Value::Array(
                runtime_config
                    .thinking_models
                    .iter()
                    .map(|m| serde_json::Value::String(m.clone()))
                    .collect(),
            ),
        );
        config_map.insert(
            "SEARCH_MODELS".to_string(),
            serde_json::Value::Array(
                runtime_config
                    .search_models
                    .iter()
                    .map(|m| serde_json::Value::String(m.clone()))
                    .collect(),
            ),
        );
        config_map.insert(
            "IMAGE_MODELS".to_string(),
            serde_json::Value::Array(
                runtime_config
                    .image_models
                    .iter()
                    .map(|m| serde_json::Value::String(m.clone()))
                    .collect(),
            ),
        );
        config_map.insert(
            "URL_CONTEXT_MODELS".to_string(),
            serde_json::Value::Array(
                runtime_config
                    .url_context_models
                    .iter()
                    .map(|m| serde_json::Value::String(m.clone()))
                    .collect(),
            ),
        );
        config_map.insert(
            "FILTERED_MODELS".to_string(),
            serde_json::Value::Array(
                runtime_config
                    .filtered_models
                    .iter()
                    .map(|m| serde_json::Value::String(m.clone()))
                    .collect(),
            ),
        );
        config_map.insert(
            "MODEL_POOLS".to_string(),
            serde_json::to_value(&runtime_config.model_pools)
                .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new())),
        );
        config_map.insert("ENABLE_TTS".to_string(), serde_json::Value::Bool(false));
        config_map.insert(
            "TTS_MODEL".to_string(),
            serde_json::Value::String("gemini-2.5-flash".to_string()),
        );
        config_map.insert("ENABLE_IMAGE".to_string(), serde_json::Value::Bool(true));
        config_map.insert(
            "IMAGE_MODEL".to_string(),
            serde_json::Value::String("gemini-2.0-flash-preview-image-generation".to_string()),
        );
        config_map.insert("STREAM_ENABLED".to_string(), serde_json::Value::Bool(true));
        config_map.insert(
            "ENABLE_SCHEDULER".to_string(),
            serde_json::Value::Bool(false),
        );
        config_map.insert(
            "CHECK_INTERVAL_HOURS".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(1.0).unwrap_or_else(|| 1.into()),
            ),
        );
        config_map.insert(
            "LOG_LEVEL".to_string(),
            serde_json::Value::String("INFO".to_string()),
        );
        config_map.insert(
            "LOG_RETENTION_DAYS".to_string(),
            serde_json::Value::Number(30.into()),
        );
        config_map.insert("PROXIES".to_string(), serde_json::Value::Array(Vec::new()));

        let mut base_models = vec![
            "gemini-2.5-flash".to_string(),
            "gemini-2.5-pro".to_string(),
            "gemini-2.0-flash".to_string(),
            "gemini-2.0-flash-exp".to_string(),
            "gemini-1.5-pro".to_string(),
            "gemini-1.5-flash".to_string(),
        ];

        base_models.extend(runtime_config.thinking_models.clone());
        base_models.extend(runtime_config.image_models.clone());
        base_models.extend(runtime_config.search_models.clone());
        base_models.extend(runtime_config.url_context_models.clone());

        base_models.sort();
        base_models.dedup();

        let filtered = &runtime_config.filtered_models;
        base_models.retain(|m| !filtered.contains(m));

        let mut active_models = Vec::new();
        let mut seen = HashSet::new();
        for id in base_models {
            if seen.insert(id.clone()) {
                active_models.push(serde_json::json!({
                    "id": id,
                    "object": "model",
                    "owned_by": "google"
                }));
            }
        }

        let mut aliases = runtime_config
            .model_pools
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        aliases.sort();
        for alias in aliases {
            if alias.is_empty() || !seen.insert(alias.clone()) {
                continue;
            }
            active_models.push(serde_json::json!({
                "id": alias,
                "object": "model",
                "owned_by": "proxy"
            }));
        }

        let mut state = Self {
            key_pool: KeyPool::new(
                &api_keys,
                runtime_config.max_failures,
                runtime_config.cooldown_secs,
            ),
            runtime_config,
            model_pool_cursors: HashMap::new(),
            model_pool_usage: HashMap::new(),
            model_pool_key_cycle_steps: HashMap::new(),
            config_map,
            logs: Vec::new(),
            calls: Vec::new(),
            proxy_cache: HashMap::new(),
            scheduler_running: false,
            scheduler_updated_at: None,
            active_models,
        };
        state.seed_demo_logs();
        state.seed_demo_calls();
        state
    }

    pub fn dashboard_overview(&self) -> DashboardOverviewV2 {
        let now = Utc::now();
        let calls_summary = DashboardCallsSummary {
            one_minute: self.summary_since(now - ChronoDuration::minutes(1)),
            one_hour: self.summary_since(now - ChronoDuration::hours(1)),
            twenty_four_hours: self.summary_since(now - ChronoDuration::hours(24)),
            month: self.summary_since(now - ChronoDuration::days(30)),
        };
        let key_summary = self.key_pool.summary();
        let total_keys = key_summary.total.max(1) as f64;
        let active_key_ratio = key_summary.active as f64 / total_keys;
        let cooldown_key_ratio = key_summary.cooldown as f64 / total_keys;
        let invalid_key_ratio = key_summary.invalid as f64 / total_keys;

        let total_calls_24h = calls_summary.twenty_four_hours.total;
        let failure_calls_24h = calls_summary.twenty_four_hours.failure;
        let failure_rate_24h = if total_calls_24h == 0 {
            0.0
        } else {
            failure_calls_24h as f64 / total_calls_24h as f64
        };

        let mut score = (100.0
            - invalid_key_ratio * 55.0
            - cooldown_key_ratio * 20.0
            - failure_rate_24h * 35.0)
            .clamp(0.0, 100.0)
            .round() as u8;
        if key_summary.total == 0 {
            score = 0;
        }
        let level = if score >= 85 {
            "healthy"
        } else if score >= 60 {
            "warning"
        } else {
            "critical"
        }
        .to_string();

        let mut model_map: HashMap<String, (u64, u64, u64)> = HashMap::new();
        let mut status_map: HashMap<u16, u64> = HashMap::new();
        let from_24h = now - ChronoDuration::hours(24);
        for call in self.calls.iter().filter(|call| call.at >= from_24h) {
            let entry = model_map.entry(call.model.clone()).or_insert((0, 0, 0));
            entry.0 += 1;
            if (200..300).contains(&call.status_code) {
                entry.1 += 1;
            } else {
                entry.2 += 1;
            }
            *status_map.entry(call.status_code).or_insert(0) += 1;
        }

        let mut model_distribution_24h = model_map
            .into_iter()
            .map(
                |(model, (total, success, failure))| DashboardModelMetricV2 {
                    model,
                    total,
                    success,
                    failure,
                    success_rate: if total == 0 {
                        0.0
                    } else {
                        success as f64 / total as f64
                    },
                },
            )
            .collect::<Vec<_>>();
        model_distribution_24h
            .sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.model.cmp(&b.model)));

        let mut status_distribution_24h = status_map
            .into_iter()
            .map(|(status_code, count)| DashboardStatusMetricV2 { status_code, count })
            .collect::<Vec<_>>();
        status_distribution_24h.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| a.status_code.cmp(&b.status_code))
        });

        DashboardOverviewV2 {
            keys_summary: key_summary,
            calls_summary,
            health: DashboardHealthV2 {
                score,
                level,
                active_key_ratio,
                cooldown_key_ratio,
                invalid_key_ratio,
                failure_rate_24h,
                total_calls_24h,
            },
            model_distribution_24h,
            status_distribution_24h,
            model_pools: self.runtime_config.model_pools.clone(),
            attention_keys: self.attention_keys(Some(429), 10),
            recent_errors: self.logs.iter().rev().take(10).cloned().collect(),
        }
    }

    pub fn pool_status(&self, event_limit: usize) -> PoolStatus {
        let summary = self.key_pool.summary();
        PoolStatus {
            strategy: self.runtime_config.pool_strategy,
            total_keys: summary.total,
            available_keys: summary.active,
            cooldown_keys: summary.cooldown,
            invalid_keys: summary.invalid,
            last_selected: self.key_pool.last_selection(),
            recent_selections: self.key_pool.recent_selections(event_limit),
        }
    }

    pub fn set_pool_strategy(&mut self, strategy: PoolStrategy) {
        self.runtime_config.pool_strategy = strategy;
        self.config_map.insert(
            "POOL_STRATEGY".to_string(),
            serde_json::Value::String(strategy.as_str().to_string()),
        );
    }

    pub fn resolve_model_alias(&mut self, alias: &str) -> String {
        self.resolve_model_alias_with_context(alias, None)
    }

    pub fn resolve_model_alias_with_context(
        &mut self,
        alias: &str,
        token_context: Option<&str>,
    ) -> String {
        let Some(candidates) = self.runtime_config.model_pools.get(alias).cloned() else {
            return alias.to_string();
        };
        if candidates.is_empty() {
            return alias.to_string();
        }

        let scope_key = self.model_scope_key(alias, token_context);
        let idx = match self.runtime_config.model_pool_strategy {
            ModelPoolStrategy::RoundRobin => {
                self.select_round_robin_model_index(&scope_key, candidates.len())
            }
            ModelPoolStrategy::LeastUsed => {
                self.select_least_used_model_index(&scope_key, &candidates)
            }
            ModelPoolStrategy::PerKeyCycle => {
                self.select_per_key_cycle_model_index(&scope_key, candidates.len())
            }
        };
        let selected = candidates[idx].clone();
        self.record_model_selection(&scope_key, &selected);
        selected
    }

    fn model_scope_key(&self, alias: &str, token_context: Option<&str>) -> String {
        match self.runtime_config.model_pool_scope {
            ModelPoolScope::Global => format!("global::{alias}"),
            ModelPoolScope::Token => {
                let token_hash = token_context
                    .map(hash_scope_token)
                    .unwrap_or_else(|| "anonymous".to_string());
                format!("token:{token_hash}::{alias}")
            }
        }
    }

    fn select_round_robin_model_index(&mut self, scope_key: &str, len: usize) -> usize {
        let cursor = self
            .model_pool_cursors
            .entry(scope_key.to_string())
            .or_insert(0);
        let current = *cursor % len;
        *cursor = (*cursor + 1) % len;
        current
    }

    fn select_least_used_model_index(&mut self, scope_key: &str, candidates: &[String]) -> usize {
        let usage = self
            .model_pool_usage
            .entry(scope_key.to_string())
            .or_default();
        let min_used = candidates
            .iter()
            .map(|model| usage.get(model).copied().unwrap_or(0))
            .min()
            .unwrap_or(0);
        let tie_indices = candidates
            .iter()
            .enumerate()
            .filter_map(|(idx, model)| {
                (usage.get(model).copied().unwrap_or(0) == min_used).then_some(idx)
            })
            .collect::<Vec<_>>();

        if tie_indices.len() <= 1 {
            return tie_indices.first().copied().unwrap_or(0);
        }

        let cursor = self
            .model_pool_cursors
            .entry(scope_key.to_string())
            .or_insert(0);
        let current = *cursor % tie_indices.len();
        *cursor = (*cursor + 1) % tie_indices.len();
        tie_indices[current]
    }

    fn select_per_key_cycle_model_index(&mut self, scope_key: &str, candidates_len: usize) -> usize {
        let key_cycle_size = self.key_pool.summary().active.max(1) as usize;
        let model_cursor = self
            .model_pool_cursors
            .entry(scope_key.to_string())
            .or_insert(0);
        let selected = *model_cursor % candidates_len;

        let key_step = self
            .model_pool_key_cycle_steps
            .entry(scope_key.to_string())
            .or_insert(0);
        *key_step = (*key_step + 1) % key_cycle_size;
        if *key_step == 0 {
            *model_cursor = (*model_cursor + 1) % candidates_len;
        }

        selected
    }

    fn record_model_selection(&mut self, scope_key: &str, model: &str) {
        let usage = self
            .model_pool_usage
            .entry(scope_key.to_string())
            .or_default();
        let current = usage.get(model).copied().unwrap_or(0);
        usage.insert(model.to_string(), current.saturating_add(1));
    }

    fn summary_since(&self, from: DateTime<Utc>) -> CallsSummaryV2 {
        let mut success = 0_u64;
        let mut failure = 0_u64;
        for call in self.calls.iter().filter(|call| call.at >= from) {
            if (200..300).contains(&call.status_code) {
                success += 1;
            } else {
                failure += 1;
            }
        }
        CallsSummaryV2 {
            total: success + failure,
            success,
            failure,
        }
    }

    pub fn stats_details(&self, period: &str) -> StatsDetailsV2 {
        let now = Utc::now();
        let (bucket, points): (ChronoDuration, usize) = match period {
            "1h" => (ChronoDuration::minutes(5), 12),
            "8h" => (ChronoDuration::minutes(30), 16),
            "24h" => (ChronoDuration::hours(1), 24),
            "month" => (ChronoDuration::days(1), 30),
            _ => (ChronoDuration::hours(1), 24),
        };

        let mut series = Vec::with_capacity(points);
        let mut total_success = 0_u64;
        let mut total_failure = 0_u64;

        for idx in (0..points).rev() {
            let end = now - bucket * idx as i32;
            let start = end - bucket;
            let mut success = 0_u64;
            let mut failure = 0_u64;
            for call in self
                .calls
                .iter()
                .filter(|call| call.at >= start && call.at < end)
            {
                if (200..300).contains(&call.status_code) {
                    success += 1;
                } else {
                    failure += 1;
                }
            }
            total_success += success;
            total_failure += failure;
            series.push(StatsPointV2 {
                at: end,
                total: success + failure,
                success,
                failure,
            });
        }

        StatsDetailsV2 {
            period: period.to_string(),
            series,
            success: total_success,
            failure: total_failure,
            total: total_success + total_failure,
        }
    }

    pub fn attention_keys(
        &self,
        status_code: Option<u16>,
        limit: usize,
    ) -> Vec<AttentionKeyItemV2> {
        let mut grouped: BTreeMap<(String, u16), AttentionKeyItemV2> = BTreeMap::new();
        for log in &self.logs {
            if let Some(target) = status_code {
                if log.status_code != target {
                    continue;
                }
            }
            let key = (log.masked_key.clone(), log.status_code);
            let entry = grouped.entry(key).or_insert_with(|| AttentionKeyItemV2 {
                key: log.masked_key.clone(),
                masked_key: log.masked_key.clone(),
                status_code: log.status_code,
                count: 0,
                last_at: None,
            });
            entry.count += 1;
            if entry.last_at.map(|t| log.request_at > t).unwrap_or(true) {
                entry.last_at = Some(log.request_at);
            }
        }

        let mut items = grouped.into_values().collect::<Vec<_>>();
        items.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| b.last_at.cmp(&a.last_at))
        });
        items.truncate(limit);
        items
    }

    pub fn config_schema(&self) -> ConfigSchemaV2 {
        let sections = vec![
            ConfigSectionV2 {
                id: "api".to_string(),
                name: "API".to_string(),
                fields: vec![
                    ConfigSchemaFieldV2 {
                        key: "AUTH_TOKEN".to_string(),
                        label: "管理 Token".to_string(),
                        field_type: "string".to_string(),
                        group: "security".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: true,
                            min: Some(8.0),
                            max: None,
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: Some("sk-xxxx".to_string()),
                            help: Some("用於登入管理後台".to_string()),
                            multiline: false,
                            secret: true,
                            options: Vec::new(),
                        },
                    },
                    ConfigSchemaFieldV2 {
                        key: "API_KEYS".to_string(),
                        label: "Gemini API Keys".to_string(),
                        field_type: "array".to_string(),
                        group: "security".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: true,
                            min: Some(1.0),
                            max: None,
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: Some("AIzaSy...".to_string()),
                            help: Some("一行一個 Key".to_string()),
                            multiline: true,
                            secret: true,
                            options: Vec::new(),
                        },
                    },
                    ConfigSchemaFieldV2 {
                        key: "ALLOWED_TOKENS".to_string(),
                        label: "允許的用戶 Token".to_string(),
                        field_type: "array".to_string(),
                        group: "security".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: false,
                            min: None,
                            max: None,
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: Some("sk-user-...".to_string()),
                            help: Some("可調用代理 API 的 Token 清單".to_string()),
                            multiline: true,
                            secret: false,
                            options: Vec::new(),
                        },
                    },
                ],
            },
            ConfigSectionV2 {
                id: "model".to_string(),
                name: "模型".to_string(),
                fields: vec![
                    ConfigSchemaFieldV2 {
                        key: "MODEL_NAME".to_string(),
                        label: "預設模型".to_string(),
                        field_type: "string".to_string(),
                        group: "model".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: true,
                            min: None,
                            max: None,
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: Some("gemini-2.5-flash".to_string()),
                            help: Some("主要代理模型".to_string()),
                            multiline: false,
                            secret: false,
                            options: vec![
                                "gemini-2.5-flash".to_string(),
                                "gemini-2.5-pro".to_string(),
                                "gemini-2.0-flash".to_string(),
                            ],
                        },
                    },
                    ConfigSchemaFieldV2 {
                        key: "THINKING_MODELS".to_string(),
                        label: "Thinking Models".to_string(),
                        field_type: "array".to_string(),
                        group: "model".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: false,
                            min: None,
                            max: None,
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: Some("gemini-2.5-pro".to_string()),
                            help: Some("進階推理可選模型".to_string()),
                            multiline: true,
                            secret: false,
                            options: Vec::new(),
                        },
                    },
                ],
            },
            ConfigSectionV2 {
                id: "tts".to_string(),
                name: "TTS".to_string(),
                fields: vec![
                    ConfigSchemaFieldV2 {
                        key: "ENABLE_TTS".to_string(),
                        label: "啟用 TTS".to_string(),
                        field_type: "boolean".to_string(),
                        group: "tts".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: true,
                            min: None,
                            max: None,
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: None,
                            help: Some("是否開啟文字轉語音".to_string()),
                            multiline: false,
                            secret: false,
                            options: Vec::new(),
                        },
                    },
                    ConfigSchemaFieldV2 {
                        key: "TTS_MODEL".to_string(),
                        label: "TTS 模型".to_string(),
                        field_type: "string".to_string(),
                        group: "tts".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: false,
                            min: None,
                            max: None,
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: Some("gemini-2.5-flash".to_string()),
                            help: None,
                            multiline: false,
                            secret: false,
                            options: Vec::new(),
                        },
                    },
                ],
            },
            ConfigSectionV2 {
                id: "image".to_string(),
                name: "圖像".to_string(),
                fields: vec![
                    ConfigSchemaFieldV2 {
                        key: "ENABLE_IMAGE".to_string(),
                        label: "啟用圖像功能".to_string(),
                        field_type: "boolean".to_string(),
                        group: "image".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: true,
                            min: None,
                            max: None,
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: None,
                            help: None,
                            multiline: false,
                            secret: false,
                            options: Vec::new(),
                        },
                    },
                    ConfigSchemaFieldV2 {
                        key: "IMAGE_MODEL".to_string(),
                        label: "圖像模型".to_string(),
                        field_type: "string".to_string(),
                        group: "image".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: false,
                            min: None,
                            max: None,
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: Some(
                                "gemini-2.0-flash-preview-image-generation".to_string(),
                            ),
                            help: None,
                            multiline: false,
                            secret: false,
                            options: Vec::new(),
                        },
                    },
                ],
            },
            ConfigSectionV2 {
                id: "stream".to_string(),
                name: "流式".to_string(),
                fields: vec![ConfigSchemaFieldV2 {
                    key: "STREAM_ENABLED".to_string(),
                    label: "啟用串流輸出".to_string(),
                    field_type: "boolean".to_string(),
                    group: "stream".to_string(),
                    rules: ConfigFieldRuleV2 {
                        required: true,
                        min: None,
                        max: None,
                        pattern: None,
                    },
                    ui_hints: ConfigFieldUiHintsV2 {
                        placeholder: None,
                        help: None,
                        multiline: false,
                        secret: false,
                        options: Vec::new(),
                    },
                }],
            },
            ConfigSectionV2 {
                id: "scheduler".to_string(),
                name: "排程".to_string(),
                fields: vec![
                    ConfigSchemaFieldV2 {
                        key: "ENABLE_SCHEDULER".to_string(),
                        label: "啟用排程".to_string(),
                        field_type: "boolean".to_string(),
                        group: "scheduler".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: true,
                            min: None,
                            max: None,
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: None,
                            help: Some("控制健康檢查排程".to_string()),
                            multiline: false,
                            secret: false,
                            options: Vec::new(),
                        },
                    },
                    ConfigSchemaFieldV2 {
                        key: "CHECK_INTERVAL_HOURS".to_string(),
                        label: "檢查間隔（小時）".to_string(),
                        field_type: "number".to_string(),
                        group: "scheduler".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: true,
                            min: Some(0.0),
                            max: Some(168.0),
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: Some("1".to_string()),
                            help: None,
                            multiline: false,
                            secret: false,
                            options: Vec::new(),
                        },
                    },
                ],
            },
            ConfigSectionV2 {
                id: "logs".to_string(),
                name: "日誌".to_string(),
                fields: vec![
                    ConfigSchemaFieldV2 {
                        key: "LOG_LEVEL".to_string(),
                        label: "日誌等級".to_string(),
                        field_type: "string".to_string(),
                        group: "logs".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: true,
                            min: None,
                            max: None,
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: None,
                            help: None,
                            multiline: false,
                            secret: false,
                            options: vec![
                                "DEBUG".to_string(),
                                "INFO".to_string(),
                                "WARN".to_string(),
                                "ERROR".to_string(),
                            ],
                        },
                    },
                    ConfigSchemaFieldV2 {
                        key: "LOG_RETENTION_DAYS".to_string(),
                        label: "保留天數".to_string(),
                        field_type: "number".to_string(),
                        group: "logs".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: true,
                            min: Some(1.0),
                            max: Some(180.0),
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: Some("30".to_string()),
                            help: None,
                            multiline: false,
                            secret: false,
                            options: Vec::new(),
                        },
                    },
                    ConfigSchemaFieldV2 {
                        key: "PROXIES".to_string(),
                        label: "代理清單".to_string(),
                        field_type: "array".to_string(),
                        group: "api".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: false,
                            min: None,
                            max: None,
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: Some("http://user:pass@host:port".to_string()),
                            help: Some("支援 http/https/socks5".to_string()),
                            multiline: true,
                            secret: false,
                            options: Vec::new(),
                        },
                    },
                    ConfigSchemaFieldV2 {
                        key: "MAX_FAILURES".to_string(),
                        label: "最大失敗次數".to_string(),
                        field_type: "number".to_string(),
                        group: "runtime".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: true,
                            min: Some(1.0),
                            max: Some(100.0),
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: Some("3".to_string()),
                            help: None,
                            multiline: false,
                            secret: false,
                            options: Vec::new(),
                        },
                    },
                    ConfigSchemaFieldV2 {
                        key: "COOLDOWN_SECONDS".to_string(),
                        label: "冷卻秒數".to_string(),
                        field_type: "number".to_string(),
                        group: "runtime".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: true,
                            min: Some(0.0),
                            max: Some(86400.0),
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: Some("60".to_string()),
                            help: None,
                            multiline: false,
                            secret: false,
                            options: Vec::new(),
                        },
                    },
                    ConfigSchemaFieldV2 {
                        key: "POOL_STRATEGY".to_string(),
                        label: "Key Pool 策略".to_string(),
                        field_type: "string".to_string(),
                        group: "runtime".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: true,
                            min: None,
                            max: None,
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: Some("round_robin".to_string()),
                            help: Some("可選 round_robin / random / least_fail".to_string()),
                            multiline: false,
                            secret: false,
                            options: vec![
                                "round_robin".to_string(),
                                "random".to_string(),
                                "least_fail".to_string(),
                            ],
                        },
                    },
                    ConfigSchemaFieldV2 {
                        key: "MODEL_POOL_STRATEGY".to_string(),
                        label: "Model Pool 策略".to_string(),
                        field_type: "string".to_string(),
                        group: "runtime".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: true,
                            min: None,
                            max: None,
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: Some("round_robin".to_string()),
                            help: Some(
                                "可選 round_robin / least_used / per_key_cycle".to_string(),
                            ),
                            multiline: false,
                            secret: false,
                            options: vec![
                                "round_robin".to_string(),
                                "least_used".to_string(),
                                "per_key_cycle".to_string(),
                            ],
                        },
                    },
                    ConfigSchemaFieldV2 {
                        key: "MODEL_POOL_SCOPE".to_string(),
                        label: "Model Pool 範圍".to_string(),
                        field_type: "string".to_string(),
                        group: "runtime".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: true,
                            min: None,
                            max: None,
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: Some("global".to_string()),
                            help: Some("可選 global / token".to_string()),
                            multiline: false,
                            secret: false,
                            options: vec!["global".to_string(), "token".to_string()],
                        },
                    },
                    ConfigSchemaFieldV2 {
                        key: "COMPAT_MODE".to_string(),
                        label: "相容模式".to_string(),
                        field_type: "boolean".to_string(),
                        group: "runtime".to_string(),
                        rules: ConfigFieldRuleV2 {
                            required: true,
                            min: None,
                            max: None,
                            pattern: None,
                        },
                        ui_hints: ConfigFieldUiHintsV2 {
                            placeholder: None,
                            help: Some("開啟後可使用舊版 API 路由".to_string()),
                            multiline: false,
                            secret: false,
                            options: Vec::new(),
                        },
                    },
                ],
            },
        ];

        ConfigSchemaV2 {
            field_count: sections.iter().map(|s| s.fields.len()).sum(),
            sections,
        }
    }

    pub fn key_usage_details(&self, key: &str, period: &str) -> HashMap<String, u64> {
        let now = Utc::now();
        let from = match period {
            "1h" => now - ChronoDuration::hours(1),
            "8h" => now - ChronoDuration::hours(8),
            "24h" => now - ChronoDuration::hours(24),
            "month" => now - ChronoDuration::days(30),
            _ => now - ChronoDuration::hours(24),
        };

        let mut result = HashMap::new();
        for call in self.calls.iter().filter(|call| call.at >= from) {
            let key_match = call.key_id == key
                || self
                    .key_pool
                    .raw_key_by_id(&call.key_id)
                    .map(|raw| raw == key)
                    .unwrap_or(false);
            if !key_match {
                continue;
            }
            *result.entry(call.model.clone()).or_insert(0) += 1;
        }
        result
    }

    pub fn ui_models(&self) -> Vec<serde_json::Value> {
        vec![
            serde_json::json!({
                "id": "gemini-2.5-flash",
                "label": "Gemini 2.5 Flash",
                "category": "text",
                "supportsThinking": false
            }),
            serde_json::json!({
                "id": "gemini-2.5-pro",
                "label": "Gemini 2.5 Pro",
                "category": "text",
                "supportsThinking": true
            }),
            serde_json::json!({
                "id": "gemini-2.0-flash-preview-image-generation",
                "label": "Gemini 2.0 Flash Image",
                "category": "image",
                "supportsThinking": false
            }),
        ]
    }

    pub fn set_scheduler_running(&mut self, running: bool) {
        self.scheduler_running = running;
        self.scheduler_updated_at = Some(Utc::now());
        self.config_map.insert(
            "ENABLE_SCHEDULER".to_string(),
            serde_json::Value::Bool(running),
        );
    }

    pub fn upsert_proxy_cache(&mut self, result: &ProxyCheckResultV2, ttl_seconds: i64) {
        self.proxy_cache.insert(
            result.proxy.clone(),
            ProxyCacheEntry {
                checked_at: result.checked_at,
                expires_at: result.checked_at + ChronoDuration::seconds(ttl_seconds),
                is_available: result.is_available,
                response_time: result.response_time,
                error_message: result.error_message.clone(),
            },
        );
    }

    pub fn proxy_cache_stats(&self, now: DateTime<Utc>) -> ProxyCacheStatsV2 {
        let total_cached = self.proxy_cache.len();
        let mut valid_cached = 0_usize;
        let mut expired_cached = 0_usize;
        for entry in self.proxy_cache.values() {
            if now <= entry.expires_at {
                valid_cached += 1;
            } else {
                expired_cached += 1;
            }
        }
        ProxyCacheStatsV2 {
            total_cached,
            valid_cached,
            expired_cached,
        }
    }

    pub fn clear_proxy_cache(&mut self) {
        self.proxy_cache.clear();
    }

    pub fn seed_demo_calls(&mut self) {
        if !self.calls.is_empty() {
            return;
        }

        let now = Utc::now();
        let models = ["gemini-2.5-flash", "gemini-2.5-pro"];
        let keys = self
            .key_pool
            .snapshot()
            .into_iter()
            .map(|k| k.id)
            .collect::<Vec<_>>();
        if keys.is_empty() {
            return;
        }

        for idx in 0..240 {
            let at = now - ChronoDuration::minutes((idx * 6) as i64);
            let status_code = if idx % 9 == 0 { 429 } else { 200 };
            let key_id = keys[idx % keys.len()].clone();
            let model = models[idx % models.len()].to_string();
            self.calls.push(CallRecord {
                at,
                key_id,
                model,
                status_code,
            });
        }
    }

    pub fn record_call(
        &mut self,
        key_id: String,
        model: String,
        status_code: u16,
        at: DateTime<Utc>,
    ) {
        self.calls.push(CallRecord {
            at,
            key_id,
            model,
            status_code,
        });
        if self.calls.len() > 20_000 {
            let drop_count = self.calls.len().saturating_sub(10_000);
            self.calls.drain(0..drop_count);
        }
    }

    pub fn seed_demo_logs(&mut self) {
        if !self.logs.is_empty() {
            return;
        }
        let now = Utc::now();
        self.logs.push(LogRecordV2 {
            id: 1,
            masked_key: "AIzaSy...abcd".to_string(),
            error_type: "RateLimit".to_string(),
            status_code: 429,
            model: "gemini-2.5-flash".to_string(),
            request_at: now - ChronoDuration::minutes(15),
            detail: "Too many requests".to_string(),
        });
        self.logs.push(LogRecordV2 {
            id: 2,
            masked_key: "AIzaSy...efgh".to_string(),
            error_type: "Upstream".to_string(),
            status_code: 503,
            model: "gemini-2.5-pro".to_string(),
            request_at: now - ChronoDuration::minutes(3),
            detail: "Upstream timeout".to_string(),
        });
    }
}

fn hash_scope_token(token: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    token.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::{Duration as ChronoDuration, Utc};

    use crate::types::{ModelPoolScope, ModelPoolStrategy, PoolStrategy};

    use super::{AppStateModel, RuntimeConfig};

    fn test_state() -> AppStateModel {
        AppStateModel::new(
            RuntimeConfig {
                auth_token: "sk-admin".to_string(),
                allowed_tokens: vec!["sk-user".to_string()],
                listen_addr: "127.0.0.1:18080".to_string(),
                max_failures: 3,
                cooldown_secs: 60,
                compat_mode: true,
                pool_strategy: PoolStrategy::RoundRobin,
                thinking_models: vec![],
                image_models: vec![],
                search_models: vec![],
                filtered_models: vec![],
                url_context_models: vec![],
                model_pools: std::collections::HashMap::new(),
                model_pool_strategy: ModelPoolStrategy::RoundRobin,
                model_pool_scope: ModelPoolScope::Global,
            },
            vec!["AIzaSy_key_1".to_string(), "AIzaSy_key_2".to_string()],
        )
    }

    fn test_state_with_model_pool() -> AppStateModel {
        let mut model_pools = HashMap::new();
        model_pools.insert(
            "claude-sonnet".to_string(),
            vec![
                "gemma-4-26b-a4b-it".to_string(),
                "gemma-4-31b-it".to_string(),
            ],
        );

        AppStateModel::new(
            RuntimeConfig {
                auth_token: "sk-admin".to_string(),
                allowed_tokens: vec!["sk-user".to_string()],
                listen_addr: "127.0.0.1:18080".to_string(),
                max_failures: 3,
                cooldown_secs: 60,
                compat_mode: true,
                pool_strategy: PoolStrategy::RoundRobin,
                thinking_models: vec![],
                image_models: vec![],
                search_models: vec![],
                filtered_models: vec![],
                url_context_models: vec![],
                model_pools,
                model_pool_strategy: ModelPoolStrategy::RoundRobin,
                model_pool_scope: ModelPoolScope::Global,
            },
            vec!["AIzaSy_key_1".to_string(), "AIzaSy_key_2".to_string()],
        )
    }

    #[test]
    fn stats_details_have_series() {
        let state = test_state();
        let details = state.stats_details("24h");
        assert_eq!(details.period, "24h");
        assert!(!details.series.is_empty());
        assert!(details.total >= details.success);
    }

    #[test]
    fn attention_keys_are_sorted() {
        let state = test_state();
        let items = state.attention_keys(Some(429), 10);
        if items.len() > 1 {
            assert!(items[0].count >= items[1].count);
        }
    }

    #[test]
    fn dashboard_overview_contains_health_and_distributions() {
        let state = test_state();
        let overview = state.dashboard_overview();
        assert!(overview.health.score <= 100);
        assert!(
            overview.calls_summary.twenty_four_hours.total
                >= overview.calls_summary.twenty_four_hours.success
        );
        assert!(!overview.model_distribution_24h.is_empty());
        assert!(!overview.status_distribution_24h.is_empty());
    }

    #[test]
    fn resolve_model_alias_should_rotate_in_round_robin_order() {
        let mut state = test_state_with_model_pool();

        assert_eq!(
            state.resolve_model_alias("claude-sonnet"),
            "gemma-4-26b-a4b-it"
        );
        assert_eq!(state.resolve_model_alias("claude-sonnet"), "gemma-4-31b-it");
        assert_eq!(
            state.resolve_model_alias("claude-sonnet"),
            "gemma-4-26b-a4b-it"
        );
    }

    #[test]
    fn resolve_model_alias_should_fallback_to_original_name() {
        let mut state = test_state_with_model_pool();
        assert_eq!(
            state.resolve_model_alias("gemini-2.5-flash"),
            "gemini-2.5-flash"
        );
    }

    #[test]
    fn token_scope_should_keep_independent_model_cursor() {
        let mut state = test_state_with_model_pool();
        state.runtime_config.model_pool_scope = ModelPoolScope::Token;

        assert_eq!(
            state.resolve_model_alias_with_context("claude-sonnet", Some("token-a")),
            "gemma-4-26b-a4b-it"
        );
        assert_eq!(
            state.resolve_model_alias_with_context("claude-sonnet", Some("token-a")),
            "gemma-4-31b-it"
        );
        assert_eq!(
            state.resolve_model_alias_with_context("claude-sonnet", Some("token-b")),
            "gemma-4-26b-a4b-it"
        );
    }

    #[test]
    fn least_used_should_balance_model_selection() {
        let mut state = test_state_with_model_pool();
        state.runtime_config.model_pool_strategy = ModelPoolStrategy::LeastUsed;

        let mut a: i32 = 0;
        let mut b: i32 = 0;
        for _ in 0..10 {
            let model = state.resolve_model_alias("claude-sonnet");
            if model == "gemma-4-26b-a4b-it" {
                a += 1;
            } else if model == "gemma-4-31b-it" {
                b += 1;
            }
        }
        assert!((a - b).abs() <= 1);
    }

    #[test]
    fn per_key_cycle_should_rotate_models_after_each_full_key_round() {
        let mut state = test_state_with_model_pool();
        state.runtime_config.model_pool_strategy = ModelPoolStrategy::PerKeyCycle;
        state.runtime_config.pool_strategy = PoolStrategy::RoundRobin;

        let now = Utc::now();
        let mut observed = Vec::new();
        for idx in 0..4 {
            let selected_key = state
                .key_pool
                .next_available_key_with_strategy(
                    now + ChronoDuration::seconds(idx),
                    PoolStrategy::RoundRobin,
                )
                .expect("selected key");
            let selected_model = state.resolve_model_alias("claude-sonnet");
            observed.push((selected_key, selected_model));
        }

        assert_eq!(
            observed,
            vec![
                (
                    "AIzaSy_key_1".to_string(),
                    "gemma-4-26b-a4b-it".to_string()
                ),
                ("AIzaSy_key_2".to_string(), "gemma-4-26b-a4b-it".to_string()),
                ("AIzaSy_key_1".to_string(), "gemma-4-31b-it".to_string()),
                ("AIzaSy_key_2".to_string(), "gemma-4-31b-it".to_string()),
            ]
        );
    }
}
