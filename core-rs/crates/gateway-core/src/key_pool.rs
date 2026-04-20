use chrono::{DateTime, Duration as ChronoDuration, Utc};

use crate::types::{KeyRecordV2, KeyStatus, KeysSummaryV2, PoolSelectionEvent, PoolStrategy};

#[derive(Debug, Clone)]
pub struct KeyState {
    pub id: String,
    pub raw_key: String,
    pub failure_count: u32,
    pub status: KeyStatus,
    pub last_used_at: Option<DateTime<Utc>>,
    pub cooldown_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct KeyPool {
    keys: Vec<KeyState>,
    next_idx: usize,
    max_failures: u32,
    cooldown_secs: u64,
    random_state: usize,
    last_selection: Option<PoolSelectionEvent>,
    selection_events: Vec<PoolSelectionEvent>,
}

impl KeyPool {
    pub fn new(raw_keys: &[String], max_failures: u32, cooldown_secs: u64) -> Self {
        let keys = raw_keys
            .iter()
            .enumerate()
            .map(|(idx, key)| KeyState {
                id: format!("key-{}", idx + 1),
                raw_key: key.clone(),
                failure_count: 0,
                status: KeyStatus::Active,
                last_used_at: None,
                cooldown_until: None,
            })
            .collect();

        Self {
            keys,
            next_idx: 0,
            max_failures,
            cooldown_secs,
            random_state: 1,
            last_selection: None,
            selection_events: Vec::new(),
        }
    }

    pub fn upsert_keys(&mut self, raw_keys: &[String]) {
        self.keys = raw_keys
            .iter()
            .enumerate()
            .map(|(idx, key)| KeyState {
                id: format!("key-{}", idx + 1),
                raw_key: key.clone(),
                failure_count: 0,
                status: KeyStatus::Active,
                last_used_at: None,
                cooldown_until: None,
            })
            .collect();
        self.next_idx = 0;
        self.last_selection = None;
        self.selection_events.clear();
    }

    pub fn set_limits(&mut self, max_failures: u32, cooldown_secs: u64) {
        self.max_failures = max_failures;
        self.cooldown_secs = cooldown_secs;
    }

    pub fn remove_by_ids(&mut self, ids: &[String]) {
        self.keys.retain(|k| !ids.contains(&k.id));
        if self.keys.is_empty() {
            self.next_idx = 0;
        } else {
            self.next_idx %= self.keys.len();
        }
    }

    pub fn remove_by_raw_keys(&mut self, raw_keys: &[String]) {
        self.keys.retain(|k| !raw_keys.contains(&k.raw_key));
        if self.keys.is_empty() {
            self.next_idx = 0;
        } else {
            self.next_idx %= self.keys.len();
        }
    }

    pub fn raw_key_by_id(&self, id: &str) -> Option<String> {
        self.keys
            .iter()
            .find(|k| k.id == id)
            .map(|k| k.raw_key.clone())
    }

    pub fn id_by_raw_key(&self, raw_key: &str) -> Option<String> {
        self.keys
            .iter()
            .find(|k| k.raw_key == raw_key)
            .map(|k| k.id.clone())
    }

    pub fn raw_keys(&self) -> Vec<String> {
        self.keys.iter().map(|k| k.raw_key.clone()).collect()
    }

    pub fn next_available_key(&mut self, now: DateTime<Utc>) -> Option<String> {
        self.next_available_key_with_strategy(now, PoolStrategy::RoundRobin)
    }

    pub fn next_available_key_with_strategy(
        &mut self,
        now: DateTime<Utc>,
        strategy: PoolStrategy,
    ) -> Option<String> {
        self.recover_cooldown(now);
        if self.keys.is_empty() {
            return None;
        }

        let selected_idx = match strategy {
            PoolStrategy::RoundRobin => self.select_round_robin_idx(),
            PoolStrategy::Random => self.select_random_idx(now),
            PoolStrategy::LeastFail => self.select_least_fail_idx(),
        }?;
        Some(self.record_selection(selected_idx, strategy, now))
    }

    pub fn mark_success(&mut self, raw_key: &str) {
        if let Some(key) = self.keys.iter_mut().find(|k| k.raw_key == raw_key) {
            key.failure_count = 0;
            key.status = KeyStatus::Active;
            key.cooldown_until = None;
        }
    }

    pub fn mark_failure(&mut self, raw_key: &str, now: DateTime<Utc>) {
        if let Some(key) = self.keys.iter_mut().find(|k| k.raw_key == raw_key) {
            key.failure_count = key.failure_count.saturating_add(1);
            if key.failure_count >= self.max_failures {
                key.status = KeyStatus::Cooldown;
                key.cooldown_until = Some(now + ChronoDuration::seconds(self.cooldown_secs as i64));
            } else {
                key.status = KeyStatus::Active;
                key.cooldown_until = None;
            }
        }
    }

    pub fn reset_failures(&mut self, ids: Option<&[String]>) {
        for key in &mut self.keys {
            let should_reset = ids
                .map(|selected| selected.contains(&key.id))
                .unwrap_or(true);
            if should_reset {
                key.failure_count = 0;
                key.status = KeyStatus::Active;
                key.cooldown_until = None;
            }
        }
    }

    pub fn snapshot(&self) -> Vec<KeyRecordV2> {
        self.keys
            .iter()
            .map(|k| KeyRecordV2 {
                id: k.id.clone(),
                key: k.raw_key.clone(),
                masked_key: mask_key(&k.raw_key),
                status: k.status,
                failure_count: k.failure_count,
                last_used_at: k.last_used_at,
                cooldown_until: k.cooldown_until,
            })
            .collect()
    }

    pub fn summary(&self) -> KeysSummaryV2 {
        let mut active = 0_u64;
        let mut cooldown = 0_u64;
        let mut invalid = 0_u64;

        for key in &self.keys {
            match key.status {
                KeyStatus::Active => active += 1,
                KeyStatus::Cooldown => cooldown += 1,
                KeyStatus::Invalid => invalid += 1,
            }
        }

        KeysSummaryV2 {
            total: self.keys.len() as u64,
            active,
            cooldown,
            invalid,
        }
    }

    pub fn recover_cooldown(&mut self, now: DateTime<Utc>) {
        for key in &mut self.keys {
            if key.status == KeyStatus::Cooldown {
                if let Some(until) = key.cooldown_until {
                    if now >= until {
                        key.failure_count = 0;
                        key.status = KeyStatus::Active;
                        key.cooldown_until = None;
                    }
                }
            }
        }
    }

    pub fn last_selection(&self) -> Option<PoolSelectionEvent> {
        self.last_selection.clone()
    }

    pub fn recent_selections(&self, limit: usize) -> Vec<PoolSelectionEvent> {
        let size = limit.max(1);
        self.selection_events
            .iter()
            .rev()
            .take(size)
            .cloned()
            .collect()
    }

    fn select_round_robin_idx(&mut self) -> Option<usize> {
        for _ in 0..self.keys.len() {
            let idx = self.next_idx % self.keys.len();
            self.next_idx = (self.next_idx + 1) % self.keys.len();
            if self.keys[idx].status == KeyStatus::Active {
                return Some(idx);
            }
        }
        None
    }

    fn select_random_idx(&mut self, now: DateTime<Utc>) -> Option<usize> {
        let candidates = self
            .keys
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| (item.status == KeyStatus::Active).then_some(idx))
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return None;
        }

        self.random_state = self
            .random_state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223)
            .wrapping_add(now.timestamp_subsec_nanos() as usize)
            .wrapping_add(self.keys.len());
        let selected = self.random_state % candidates.len();
        Some(candidates[selected])
    }

    fn select_least_fail_idx(&self) -> Option<usize> {
        self.keys
            .iter()
            .enumerate()
            .filter(|(_, item)| item.status == KeyStatus::Active)
            .min_by(|(_, a), (_, b)| {
                let a_last = a
                    .last_used_at
                    .map(|it| it.timestamp_millis())
                    .unwrap_or(i64::MIN);
                let b_last = b
                    .last_used_at
                    .map(|it| it.timestamp_millis())
                    .unwrap_or(i64::MIN);
                a.failure_count
                    .cmp(&b.failure_count)
                    .then_with(|| a_last.cmp(&b_last))
                    .then_with(|| a.id.cmp(&b.id))
            })
            .map(|(idx, _)| idx)
    }

    fn record_selection(
        &mut self,
        idx: usize,
        strategy: PoolStrategy,
        now: DateTime<Utc>,
    ) -> String {
        self.keys[idx].last_used_at = Some(now);
        let raw_key = self.keys[idx].raw_key.clone();
        let event = PoolSelectionEvent {
            at: now,
            strategy,
            key_id: self.keys[idx].id.clone(),
            masked_key: mask_key(&raw_key),
            failure_count: self.keys[idx].failure_count,
            status: self.keys[idx].status,
        };
        self.last_selection = Some(event.clone());
        self.selection_events.push(event);
        if self.selection_events.len() > 200 {
            let drop_count = self.selection_events.len().saturating_sub(200);
            self.selection_events.drain(0..drop_count);
        }
        raw_key
    }
}

pub fn mask_key(raw: &str) -> String {
    if raw.len() <= 10 {
        return raw.to_string();
    }
    format!("{}...{}", &raw[0..6], &raw[raw.len() - 4..])
}

#[cfg(test)]
mod tests {
    use chrono::{Duration as ChronoDuration, Utc};

    use crate::types::PoolStrategy;

    use super::{KeyPool, KeyStatus};

    #[test]
    fn rotates_available_keys() {
        let keys = vec![
            "test-key-alpha-0001".to_string(),
            "test-key-bravo-0002".to_string(),
        ];
        let mut pool = KeyPool::new(&keys, 3, 60);
        let now = Utc::now();
        let first = pool.next_available_key(now).expect("first key");
        let second = pool
            .next_available_key(now + ChronoDuration::seconds(1))
            .expect("second key");
        assert_ne!(first, second);
    }

    #[test]
    fn least_fail_uses_deterministic_tiebreak() {
        let keys = vec![
            "test-key-alpha-0001".to_string(),
            "test-key-bravo-0002".to_string(),
            "test-key-charlie-0003".to_string(),
        ];
        let mut pool = KeyPool::new(&keys, 3, 60);
        let now = Utc::now();

        let first = pool
            .next_available_key_with_strategy(now, PoolStrategy::LeastFail)
            .expect("first key");
        let second = pool
            .next_available_key_with_strategy(
                now + ChronoDuration::seconds(1),
                PoolStrategy::LeastFail,
            )
            .expect("second key");
        let third = pool
            .next_available_key_with_strategy(
                now + ChronoDuration::seconds(2),
                PoolStrategy::LeastFail,
            )
            .expect("third key");

        assert_eq!(first, "test-key-alpha-0001");
        assert_eq!(second, "test-key-bravo-0002");
        assert_eq!(third, "test-key-charlie-0003");
    }

    #[test]
    fn random_strategy_picks_active_keys_only() {
        let keys = vec![
            "test-key-alpha-0001".to_string(),
            "test-key-bravo-0002".to_string(),
        ];
        let mut pool = KeyPool::new(&keys, 3, 60);
        let now = Utc::now();

        let mut observed = Vec::new();
        for offset in 0..5 {
            let selected = pool
                .next_available_key_with_strategy(
                    now + ChronoDuration::milliseconds(offset),
                    PoolStrategy::Random,
                )
                .expect("random selected key");
            observed.push(selected);
        }

        assert!(observed.iter().all(|item| item.starts_with("test-key-")));
    }

    #[test]
    fn enters_and_recovers_from_cooldown() {
        let keys = vec!["test-key-alpha-0001".to_string()];
        let mut pool = KeyPool::new(&keys, 2, 1);
        let now = Utc::now();
        pool.mark_failure("test-key-alpha-0001", now);
        pool.mark_failure("test-key-alpha-0001", now);
        let snapshot = pool.snapshot();
        assert_eq!(snapshot[0].status, KeyStatus::Cooldown);

        pool.recover_cooldown(now + ChronoDuration::seconds(2));
        let snapshot = pool.snapshot();
        assert_eq!(snapshot[0].status, KeyStatus::Active);
        assert_eq!(snapshot[0].failure_count, 0);
    }

    #[test]
    fn failure_below_threshold_should_not_remove_key_from_rotation() {
        let keys = vec!["test-key-alpha-0001".to_string()];
        let mut pool = KeyPool::new(&keys, 3, 60);
        let now = Utc::now();

        pool.mark_failure("test-key-alpha-0001", now);

        let snapshot = pool.snapshot();
        assert_eq!(snapshot[0].failure_count, 1);
        assert_eq!(snapshot[0].status, KeyStatus::Active);
        assert_eq!(
            pool.next_available_key(now + ChronoDuration::seconds(1))
                .as_deref(),
            Some("test-key-alpha-0001")
        );
    }
}
