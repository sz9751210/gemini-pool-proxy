use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyStatus {
    Active,
    Cooldown,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyRecordV2 {
    pub id: String,
    pub key: String,
    pub masked_key: String,
    pub status: KeyStatus,
    pub failure_count: u32,
    pub last_used_at: Option<DateTime<Utc>>,
    pub cooldown_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeysSummaryV2 {
    pub total: u64,
    pub active: u64,
    pub cooldown: u64,
    pub invalid: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolStrategy {
    RoundRobin,
    Random,
    LeastFail,
}

impl PoolStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RoundRobin => "round_robin",
            Self::Random => "random",
            Self::LeastFail => "least_fail",
        }
    }
}

impl Default for PoolStrategy {
    fn default() -> Self {
        Self::RoundRobin
    }
}

impl FromStr for PoolStrategy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "round_robin" | "roundrobin" | "rr" => Ok(Self::RoundRobin),
            "random" => Ok(Self::Random),
            "least_fail" | "leastfail" => Ok(Self::LeastFail),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelPoolStrategy {
    RoundRobin,
    LeastUsed,
    PerKeyCycle,
}

impl ModelPoolStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RoundRobin => "round_robin",
            Self::LeastUsed => "least_used",
            Self::PerKeyCycle => "per_key_cycle",
        }
    }
}

impl Default for ModelPoolStrategy {
    fn default() -> Self {
        Self::RoundRobin
    }
}

impl FromStr for ModelPoolStrategy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "round_robin" | "roundrobin" | "rr" => Ok(Self::RoundRobin),
            "least_used" | "leastused" | "lu" => Ok(Self::LeastUsed),
            "per_key_cycle" | "perkeycycle" | "pkc" => Ok(Self::PerKeyCycle),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelPoolScope {
    Global,
    Token,
}

impl ModelPoolScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Token => "token",
        }
    }
}

impl Default for ModelPoolScope {
    fn default() -> Self {
        Self::Global
    }
}

impl FromStr for ModelPoolScope {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "global" => Ok(Self::Global),
            "token" | "per_token" | "per-token" => Ok(Self::Token),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolSelectionEvent {
    pub at: DateTime<Utc>,
    pub strategy: PoolStrategy,
    pub key_id: String,
    pub masked_key: String,
    pub failure_count: u32,
    pub status: KeyStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolStatus {
    pub strategy: PoolStrategy,
    pub total_keys: u64,
    pub available_keys: u64,
    pub cooldown_keys: u64,
    pub invalid_keys: u64,
    pub last_selected: Option<PoolSelectionEvent>,
    pub recent_selections: Vec<PoolSelectionEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationV2 {
    pub page: usize,
    pub page_size: usize,
    pub total_items: usize,
    pub total_pages: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KeyFiltersAppliedV2 {
    pub search: String,
    pub status: String,
    pub min_failure_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyListResponseV2 {
    pub items: Vec<KeyRecordV2>,
    pub summary: KeysSummaryV2,
    pub pagination: PaginationV2,
    pub filters_applied: KeyFiltersAppliedV2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyActionFailedItemV2 {
    pub key: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyActionResultV2 {
    pub action: String,
    pub success_count: usize,
    pub failed_items: Vec<KeyActionFailedItemV2>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallsSummaryV2 {
    pub total: u64,
    pub success: u64,
    pub failure: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsPointV2 {
    pub at: DateTime<Utc>,
    pub total: u64,
    pub success: u64,
    pub failure: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsDetailsV2 {
    pub period: String,
    pub series: Vec<StatsPointV2>,
    pub success: u64,
    pub failure: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttentionKeyItemV2 {
    pub key: String,
    pub masked_key: String,
    pub status_code: u16,
    pub count: u64,
    pub last_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogRecordV2 {
    pub id: u64,
    pub masked_key: String,
    pub error_type: String,
    pub status_code: u16,
    pub model: String,
    pub request_at: DateTime<Utc>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogDetailV2 {
    pub id: u64,
    pub key_id: String,
    pub masked_key: String,
    pub error_type: String,
    pub status_code: u16,
    pub model: String,
    pub request_at: DateTime<Utc>,
    pub detail: String,
    pub request_body: String,
    pub response_body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogListResponseV2 {
    pub logs: Vec<LogRecordV2>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardOverviewV2 {
    pub keys_summary: KeysSummaryV2,
    pub calls_summary: DashboardCallsSummary,
    pub health: DashboardHealthV2,
    pub model_distribution_24h: Vec<DashboardModelMetricV2>,
    pub status_distribution_24h: Vec<DashboardStatusMetricV2>,
    pub model_pools: std::collections::HashMap<String, Vec<String>>,
    pub attention_keys: Vec<AttentionKeyItemV2>,
    pub recent_errors: Vec<LogRecordV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardCallsSummary {
    pub one_minute: CallsSummaryV2,
    pub one_hour: CallsSummaryV2,
    pub twenty_four_hours: CallsSummaryV2,
    pub month: CallsSummaryV2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardHealthV2 {
    pub score: u8,
    pub level: String,
    pub active_key_ratio: f64,
    pub cooldown_key_ratio: f64,
    pub invalid_key_ratio: f64,
    pub failure_rate_24h: f64,
    pub total_calls_24h: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardModelMetricV2 {
    pub model: String,
    pub total: u64,
    pub success: u64,
    pub failure: u64,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStatusMetricV2 {
    pub status_code: u16,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFieldRuleV2 {
    pub required: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFieldUiHintsV2 {
    pub placeholder: Option<String>,
    pub help: Option<String>,
    pub multiline: bool,
    pub secret: bool,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSchemaFieldV2 {
    pub key: String,
    pub label: String,
    pub field_type: String,
    pub group: String,
    pub rules: ConfigFieldRuleV2,
    pub ui_hints: ConfigFieldUiHintsV2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSectionV2 {
    pub id: String,
    pub name: String,
    pub fields: Vec<ConfigSchemaFieldV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSchemaV2 {
    pub field_count: usize,
    pub sections: Vec<ConfigSectionV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyCheckResultV2 {
    pub proxy: String,
    pub is_available: bool,
    pub response_time: Option<f64>,
    pub error_message: Option<String>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyCacheStatsV2 {
    pub total_cached: usize,
    pub valid_cached: usize,
    pub expired_cached: usize,
}
