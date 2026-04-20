import type {
  AttentionKeyItemV2,
  ConfigSchemaV2,
  DashboardOverviewV2,
  KeyActionResultV2,
  KeyListResponseV2,
  LogDetailV2,
  LogListResponseV2,
  PoolStatus,
  PoolStrategy,
  ProxyCacheStatsV2,
  ProxyCheckResultV2,
  SchedulerStatusV2,
  SessionStatusResponse,
  StatsDetailsV2
} from "./types";

const DEFAULT_BASE = "http://127.0.0.1:18080";
const AUTH_STORAGE_KEY = "gb_auth_token";

let runtimeBaseCache = "";

export class ApiError extends Error {
  status: number;
  constructor(message: string, status: number) {
    super(message);
    this.status = status;
  }
}

export async function desktopInvoke<T = unknown>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export function getAuthToken(): string {
  if (typeof window === "undefined") {
    return "";
  }
  return window.localStorage.getItem(AUTH_STORAGE_KEY) ?? "";
}

export function setAuthToken(token: string) {
  if (typeof window === "undefined") {
    return;
  }
  if (token) {
    window.localStorage.setItem(AUTH_STORAGE_KEY, token);
  } else {
    window.localStorage.removeItem(AUTH_STORAGE_KEY);
  }
}

export function clearAuthToken() {
  setAuthToken("");
}

export async function runtimeBase(force = false): Promise<string> {
  if (runtimeBaseCache && !force) {
    return runtimeBaseCache;
  }
  try {
    const base = await desktopInvoke<string>("runtime_base_url");
    if (base) {
      runtimeBaseCache = base;
      return base;
    }
  } catch {
    // ignore and use fallback
  }
  runtimeBaseCache = DEFAULT_BASE;
  return runtimeBaseCache;
}

function notifySessionExpired() {
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent("session-expired"));
  }
}

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const base = await runtimeBase();
  const token = getAuthToken();
  const headers = new Headers(init.headers);
  if (!headers.has("Content-Type") && init.body) {
    headers.set("Content-Type", "application/json");
  }
  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
  }

  const res = await fetch(`${base}${path}`, {
    credentials: "omit",
    ...init,
    headers
  });

  const rawText = await res.text();
  let payload: unknown = null;
  if (rawText) {
    try {
      payload = JSON.parse(rawText);
    } catch {
      payload = rawText;
    }
  }

  if (res.status === 401 || res.status === 403) {
    notifySessionExpired();
    clearAuthToken();
    throw new ApiError("登入已失效，請重新登入", res.status);
  }

  if (!res.ok) {
    const message =
      typeof payload === "object" && payload !== null && "detail" in payload
        ? String((payload as Record<string, unknown>).detail)
        : typeof payload === "object" && payload !== null && "message" in payload
          ? String((payload as Record<string, unknown>).message)
          : `HTTP ${res.status}`;
    throw new ApiError(message, res.status);
  }

  return payload as T;
}

function normalizeKeyListResponse(payload: Record<string, unknown>): KeyListResponseV2 {
  const paginationRaw = (payload.pagination ?? {}) as Record<string, unknown>;
  const normalized = {
    ...payload,
    items: (payload.items ?? []) as unknown[],
    summary: (payload.summary ?? {}) as Record<string, unknown>,
    pagination: {
      page: Number(paginationRaw.page ?? payload.currentPage ?? 1),
      pageSize: Number(paginationRaw.pageSize ?? payload.pageSize ?? 20),
      totalItems: Number(paginationRaw.totalItems ?? payload.totalItems ?? 0),
      totalPages: Number(paginationRaw.totalPages ?? payload.totalPages ?? 1)
    },
    filtersApplied: (payload.filtersApplied ?? payload.filters_applied ?? {
      search: "",
      status: "all"
    }) as Record<string, unknown>,
    totalItems: Number(payload.totalItems ?? paginationRaw.totalItems ?? 0),
    totalPages: Number(payload.totalPages ?? paginationRaw.totalPages ?? 1),
    currentPage: Number(payload.currentPage ?? paginationRaw.page ?? 1),
    pageSize: Number(payload.pageSize ?? paginationRaw.pageSize ?? 20)
  } as KeyListResponseV2;
  return normalized;
}

function normalizeAttention(items: unknown): AttentionKeyItemV2[] {
  if (!Array.isArray(items)) {
    return [];
  }
  return items.map((item) => {
    const row = item as Record<string, unknown>;
    return {
      key: String(row.key ?? ""),
      maskedKey: String(row.maskedKey ?? row.masked_key ?? row.key ?? ""),
      statusCode: Number(row.statusCode ?? row.status_code ?? 429),
      count: Number(row.count ?? 0),
      lastAt: (row.lastAt ?? row.last_at ?? null) as string | null
    };
  });
}

export const apiClient = {
  async getSessionStatus() {
    return request<SessionStatusResponse>("/api/v1/session/status");
  },
  async login(authToken: string) {
    const resp = await request<{ success: boolean }>("/api/v1/session/login", {
      method: "POST",
      body: JSON.stringify({ auth_token: authToken })
    });
    setAuthToken(authToken);
    return resp;
  },
  async logout() {
    const resp = await request<{ success: boolean }>("/api/v1/session/logout", {
      method: "POST"
    });
    clearAuthToken();
    return resp;
  },
  getDashboardOverview() {
    return request<DashboardOverviewV2>("/api/v1/dashboard/overview");
  },
  async getKeys(params: {
    page: number;
    limit: number;
    search?: string;
    status?: string;
    minFailureCount?: number;
  }) {
    const query = new URLSearchParams({
      page: String(params.page),
      limit: String(params.limit)
    });
    if (params.search) {
      query.set("search", params.search);
    }
    if (params.status) {
      query.set("status", params.status);
    }
    if (typeof params.minFailureCount === "number") {
      query.set("minFailureCount", String(params.minFailureCount));
    }
    const payload = await request<Record<string, unknown>>(`/api/v1/keys?${query.toString()}`);
    return normalizeKeyListResponse(payload);
  },
  getAllKeys() {
    return request<{ valid_keys: string[]; invalid_keys: string[] }>("/api/v1/keys/all");
  },
  keyAction(action: "verify" | "reset" | "delete", ids: string[]) {
    return request<KeyActionResultV2>("/api/v1/keys/actions", {
      method: "POST",
      body: JSON.stringify({ action, ids })
    });
  },
  keyActionByKeys(action: "verify" | "reset" | "delete", keys: string[]) {
    return request<KeyActionResultV2>("/api/v1/keys/actions", {
      method: "POST",
      body: JSON.stringify({ action, keys })
    });
  },
  verifySingleKey(id: string) {
    return request<KeyActionResultV2>("/api/v1/keys/actions", {
      method: "POST",
      body: JSON.stringify({ action: "verify", ids: [id] })
    });
  },
  resetSingleKeyFailCount(id: string) {
    return request<KeyActionResultV2>("/api/v1/keys/actions", {
      method: "POST",
      body: JSON.stringify({ action: "reset", ids: [id] })
    });
  },
  getKeyUsageDetails(key: string, period = "24h") {
    return request<{ key: string; period: string; usage: Record<string, number> }>(
      `/api/v1/keys/usage/${encodeURIComponent(key)}?period=${encodeURIComponent(period)}`
    );
  },
  getKeyCallDetails(key: string, period = "24h") {
    return request<Array<Record<string, unknown>>>(
      `/api/v1/stats/key-details?key=${encodeURIComponent(key)}&period=${encodeURIComponent(period)}`
    );
  },
  getStatsDetails(period: "1h" | "8h" | "24h" | "month") {
    return request<StatsDetailsV2>(`/api/v1/stats/details?period=${encodeURIComponent(period)}`);
  },
  async getAttentionKeys(statusCode: number, limit: number) {
    const raw = await request<unknown[]>(
      `/api/v1/stats/attention-keys?statusCode=${encodeURIComponent(String(statusCode))}&limit=${encodeURIComponent(String(limit))}`
    );
    return normalizeAttention(raw);
  },
  getConfig() {
    return request<Record<string, unknown>>("/api/v1/config");
  },
  saveConfig(data: Record<string, unknown>) {
    return request<{ success: boolean; config: Record<string, unknown> }>("/api/v1/config", {
      method: "PUT",
      body: JSON.stringify(data)
    });
  },
  resetConfig() {
    return request<{ success: boolean; config: Record<string, unknown> }>("/api/v1/config/reset", {
      method: "POST"
    });
  },
  getConfigSchema() {
    return request<ConfigSchemaV2>("/api/v1/config/schema");
  },
  getUIModels() {
    return request<{ data: Array<Record<string, unknown>> }>("/api/v1/config/ui-models");
  },
  addKeys(keys: string[]) {
    return request<{ success: boolean; added: number; total: number }>("/api/v1/config/keys/add", {
      method: "POST",
      body: JSON.stringify({ items: keys })
    });
  },
  deleteKeys(keys: string[]) {
    return request<{ success: boolean; deleted: number; total: number }>("/api/v1/config/keys/delete", {
      method: "POST",
      body: JSON.stringify({ items: keys })
    });
  },
  addProxies(proxies: string[]) {
    return request<{ success: boolean; total: number }>("/api/v1/config/proxies/add", {
      method: "POST",
      body: JSON.stringify({ items: proxies })
    });
  },
  deleteProxies(proxies: string[]) {
    return request<{ success: boolean; total: number }>("/api/v1/config/proxies/delete", {
      method: "POST",
      body: JSON.stringify({ items: proxies })
    });
  },
  startScheduler() {
    return request<{ running: boolean; message: string; updatedAt?: string }>("/api/v1/scheduler/start", {
      method: "POST"
    });
  },
  stopScheduler() {
    return request<{ running: boolean; message: string; updatedAt?: string }>("/api/v1/scheduler/stop", {
      method: "POST"
    });
  },
  schedulerStatus() {
    return request<SchedulerStatusV2>("/api/v1/scheduler/status");
  },
  checkProxy(proxy: string, useCache = false) {
    return request<ProxyCheckResultV2>("/api/v1/proxy/check", {
      method: "POST",
      body: JSON.stringify({ proxy, use_cache: useCache })
    });
  },
  checkAllProxies(proxies: string[], useCache = false) {
    return request<ProxyCheckResultV2[]>("/api/v1/proxy/check-all", {
      method: "POST",
      body: JSON.stringify({ proxies, use_cache: useCache, max_concurrent: 5 })
    });
  },
  getProxyCacheStats() {
    return request<ProxyCacheStatsV2>("/api/v1/proxy/cache-stats");
  },
  clearProxyCache() {
    return request<{ success: boolean; message: string }>("/api/v1/proxy/cache-clear", {
      method: "POST"
    });
  },
  getPoolStatus(limit = 10) {
    return request<PoolStatus>(`/api/v1/pool/status?limit=${encodeURIComponent(String(limit))}`);
  },
  setPoolStrategy(strategy: PoolStrategy) {
    return request<{ success: boolean; strategy: PoolStrategy; pool: PoolStatus }>("/api/v1/pool/strategy", {
      method: "PUT",
      body: JSON.stringify({ strategy })
    });
  },
  getLogs(params: {
    limit: number;
    offset: number;
    keySearch?: string;
    errorSearch?: string;
    errorCodeSearch?: string;
    startDate?: string;
    endDate?: string;
    sortBy?: string;
    sortOrder?: "asc" | "desc";
  }) {
    const query = new URLSearchParams({
      limit: String(params.limit),
      offset: String(params.offset)
    });
    if (params.keySearch) {
      query.set("key_search", params.keySearch);
    }
    if (params.errorSearch) {
      query.set("error_search", params.errorSearch);
    }
    if (params.errorCodeSearch) {
      query.set("error_code_search", params.errorCodeSearch);
    }
    if (params.startDate) {
      query.set("start_date", params.startDate);
    }
    if (params.endDate) {
      query.set("end_date", params.endDate);
    }
    if (params.sortBy) {
      query.set("sort_by", params.sortBy);
    }
    if (params.sortOrder) {
      query.set("sort_order", params.sortOrder);
    }
    return request<LogListResponseV2>(`/api/v1/logs?${query.toString()}`);
  },
  lookupLog(key: string, statusCode?: number, timestamp?: string) {
    const query = new URLSearchParams();
    query.set("key", key);
    if (typeof statusCode === "number") {
      query.set("status_code", String(statusCode));
    }
    if (timestamp) {
      query.set("timestamp", timestamp);
    }
    return request<{ log: LogDetailV2 | null }>(`/api/v1/logs/lookup?${query.toString()}`);
  },
  getLogDetail(id: number) {
    return request<{ log: LogDetailV2 }>(`/api/v1/logs/${id}`);
  },
  deleteLog(id: number) {
    return request<void>(`/api/v1/logs/${id}`, { method: "DELETE" });
  },
  deleteLogs(ids: number[]) {
    return request<void>("/api/v1/logs/bulk", {
      method: "DELETE",
      body: JSON.stringify({ ids })
    });
  },
  deleteAllLogs() {
    return request<void>("/api/v1/logs/all", { method: "DELETE" });
  },
  gatewayStatus() {
    return desktopInvoke<string>("gateway_status");
  },
  startGateway() {
    return desktopInvoke<string>("start_gateway");
  },
  stopGateway() {
    return desktopInvoke<string>("stop_gateway");
  },
  importEnv(path: string) {
    return desktopInvoke<{ imported_count: number; secure_path: string }>("import_legacy_env", { path });
  },
  getAuthTokenHint() {
    return desktopInvoke<string | null>("auth_token_hint");
  }
};
