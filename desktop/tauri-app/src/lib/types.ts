export type SessionStatusResponse = {
  authenticated: boolean;
};

export type PoolStrategy = "round_robin" | "random" | "least_fail";

export type PoolSelectionEvent = {
  at: string;
  strategy: PoolStrategy;
  keyId: string;
  maskedKey: string;
  failureCount: number;
  status: KeyStatus;
};

export type PoolStatus = {
  strategy: PoolStrategy;
  totalKeys: number;
  availableKeys: number;
  cooldownKeys: number;
  invalidKeys: number;
  lastSelected?: PoolSelectionEvent | null;
  recentSelections: PoolSelectionEvent[];
};

export type KeyStatus = "active" | "cooldown" | "invalid";

export type KeyRecordV2 = {
  id: string;
  key: string;
  maskedKey: string;
  status: KeyStatus;
  failureCount: number;
  lastUsedAt?: string | null;
  cooldownUntil?: string | null;
};

export type KeysSummaryV2 = {
  total: number;
  active: number;
  cooldown: number;
  invalid: number;
};

export type KeyListResponseV2 = {
  items: KeyRecordV2[];
  summary: KeysSummaryV2;
  pagination: {
    page: number;
    pageSize: number;
    totalItems: number;
    totalPages: number;
  };
  filtersApplied: {
    search: string;
    status: string;
    minFailureCount?: number;
  };
  totalItems: number;
  totalPages: number;
  currentPage: number;
  pageSize: number;
};

export type KeyActionResultV2 = {
  action: string;
  successCount: number;
  failedItems: Array<{ key: string; reason: string }>;
  message: string;
};

export type CallsSummaryV2 = {
  total: number;
  success: number;
  failure: number;
};

export type DashboardOverviewV2 = {
  keysSummary: KeysSummaryV2;
  callsSummary: {
    oneMinute: CallsSummaryV2;
    oneHour: CallsSummaryV2;
    twentyFourHours: CallsSummaryV2;
    month: CallsSummaryV2;
  };
  health: {
    score: number;
    level: "healthy" | "warning" | "critical" | string;
    activeKeyRatio: number;
    cooldownKeyRatio: number;
    invalidKeyRatio: number;
    failureRate24h: number;
    totalCalls24h: number;
  };
  modelDistribution24h: Array<{
    model: string;
    total: number;
    success: number;
    failure: number;
    successRate: number;
  }>;
  statusDistribution24h: Array<{
    statusCode: number;
    count: number;
  }>;
  modelPools: Record<string, string[]>;
  attentionKeys: AttentionKeyItemV2[];
  recentErrors: LogRecordV2[];
};

export type StatsDetailsV2 = {
  period: string;
  series: Array<{
    at: string;
    total: number;
    success: number;
    failure: number;
  }>;
  success: number;
  failure: number;
  total: number;
};

export type AttentionKeyItemV2 = {
  key: string;
  maskedKey: string;
  statusCode: number;
  count: number;
  lastAt?: string | null;
};

export type ConfigSchemaFieldV2 = {
  key: string;
  label: string;
  fieldType: "string" | "number" | "boolean" | "array" | "object";
  group: string;
  rules: {
    required: boolean;
    min?: number;
    max?: number;
    pattern?: string;
  };
  uiHints: {
    placeholder?: string;
    help?: string;
    multiline: boolean;
    secret: boolean;
    options: string[];
  };
};

export type ConfigSchemaV2 = {
  fieldCount: number;
  sections: Array<{
    id: string;
    name: string;
    fields: ConfigSchemaFieldV2[];
  }>;
};

export type ProxyCheckResultV2 = {
  proxy: string;
  isAvailable: boolean;
  responseTime?: number | null;
  errorMessage?: string | null;
  checkedAt: string;
};

export type ProxyCacheStatsV2 = {
  totalCached: number;
  validCached: number;
  expiredCached: number;
};

export type SchedulerStatusV2 = {
  running: boolean;
  updatedAt?: string | null;
};

export type LogRecordV2 = {
  id: number;
  maskedKey: string;
  errorType: string;
  statusCode: number;
  model: string;
  requestAt: string;
  detail: string;
};

export type LogDetailV2 = {
  id: number;
  keyId: string;
  maskedKey: string;
  errorType: string;
  statusCode: number;
  model: string;
  requestAt: string;
  detail: string;
  requestBody: string;
  responseBody: string;
};

export type LogListResponseV2 = {
  logs: LogRecordV2[];
  total: number;
  limit: number;
  offset: number;
};
