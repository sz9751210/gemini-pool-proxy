import {
  CategoryScale,
  Chart as ChartJS,
  Filler,
  Legend,
  LineElement,
  LinearScale,
  PointElement,
  Tooltip
} from "chart.js";
import { useEffect, useMemo, useState } from "react";
import { Line } from "react-chartjs-2";
import { Modal } from "../components/Modal";
import { useNotifier } from "../components/NotificationProvider";
import { apiClient, runtimeBase } from "../lib/apiClient";
import type {
  AttentionKeyItemV2,
  DashboardOverviewV2,
  KeyRecordV2,
  PoolStatus,
  PoolStrategy,
  StatsDetailsV2
} from "../lib/types";

ChartJS.register(CategoryScale, LinearScale, PointElement, LineElement, Tooltip, Legend, Filler);

type ChartPeriod = "1h" | "8h" | "24h";

const POOL_STRATEGY_OPTIONS: Array<{ value: PoolStrategy; label: string; desc: string }> = [
  { value: "round_robin", label: "Round Robin", desc: "平均輪詢每把 key（預設）" },
  { value: "random", label: "Random", desc: "隨機分配可用 key" },
  { value: "least_fail", label: "Least Fail", desc: "優先選擇失敗次數較低 key" }
];

function isValidKeyStatus(status: KeyRecordV2["status"]) {
  return status === "active" || status === "cooldown";
}

async function copyText(text: string) {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }
  const textarea = document.createElement("textarea");
  textarea.value = text;
  document.body.appendChild(textarea);
  textarea.select();
  document.execCommand("copy");
  document.body.removeChild(textarea);
}

export function KeysPage() {
  const notifier = useNotifier();
  const [loading, setLoading] = useState(false);
  const [keys, setKeys] = useState<KeyRecordV2[]>([]);
  const [summary, setSummary] = useState({ total: 0, active: 0, cooldown: 0, invalid: 0 });
  const [page, setPage] = useState(1);
  const [limit] = useState(20);
  const [totalPages, setTotalPages] = useState(1);
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [minFailureCount, setMinFailureCount] = useState<number | "">("");
  const [selected, setSelected] = useState<Record<string, boolean>>({});
  const [poolStatus, setPoolStatus] = useState<PoolStatus | null>(null);
  const [overview, setOverview] = useState<DashboardOverviewV2 | null>(null);
  const [updatingPoolStrategy, setUpdatingPoolStrategy] = useState(false);
  const [apiUrl, setApiUrl] = useState("");

  const [stats, setStats] = useState<Partial<Record<ChartPeriod, StatsDetailsV2>>>({});
  const [chartPeriod, setChartPeriod] = useState<ChartPeriod>("24h");

  const [attentionKeys, setAttentionKeys] = useState<AttentionKeyItemV2[]>([]);
  const [attentionStatusCode, setAttentionStatusCode] = useState(429);
  const [attentionLimit, setAttentionLimit] = useState(20);
  const [selectedAttention, setSelectedAttention] = useState<Record<string, boolean>>({});

  const [expandValid, setExpandValid] = useState(true);
  const [expandInvalid, setExpandInvalid] = useState(true);

  const [busyAction, setBusyAction] = useState<"" | "verify" | "reset" | "delete">("");
  const [progressOpen, setProgressOpen] = useState(false);
  const [resultModal, setResultModal] = useState<{ open: boolean; title: string; message: string }>({
    open: false,
    title: "",
    message: ""
  });

  const [usageModal, setUsageModal] = useState<{
    open: boolean;
    key: string;
    loading: boolean;
    usage: Record<string, number>;
    period: string;
  }>({
    open: false,
    key: "",
    loading: false,
    usage: {},
    period: "24h"
  });

  const [callsModal, setCallsModal] = useState<{
    open: boolean;
    key: string;
    period: ChartPeriod;
    loading: boolean;
    rows: Array<Record<string, unknown>>;
  }>({
    open: false,
    key: "",
    period: "24h",
    loading: false,
    rows: []
  });

  useEffect(() => {
    void loadKeys();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [page, search, statusFilter, minFailureCount]);

  useEffect(() => {
    void loadAttentionKeys();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [attentionStatusCode, attentionLimit]);

  useEffect(() => {
    void loadStats();
    void loadPoolStatus();
    void loadOverview();
    void runtimeBase().then((base) => setApiUrl(`${base}/v1/chat/completions`));
  }, []);

  async function loadKeys() {
    setLoading(true);
    try {
      const data = await apiClient.getKeys({
        page,
        limit,
        search: search.trim() || undefined,
        status: statusFilter,
        minFailureCount: minFailureCount === "" ? undefined : Number(minFailureCount)
      });
      setKeys(data.items);
      setSummary(data.summary);
      setTotalPages(Math.max(1, data.totalPages || data.pagination.totalPages || 1));
      setSelected({});
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "載入 key 清單失敗");
    } finally {
      setLoading(false);
    }
  }

  async function loadAttentionKeys() {
    try {
      const data = await apiClient.getAttentionKeys(attentionStatusCode, attentionLimit);
      setAttentionKeys(data);
      setSelectedAttention({});
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "載入 Attention keys 失敗");
    }
  }

  async function loadStats() {
    try {
      const [oneHour, eightHours, day] = await Promise.all([
        apiClient.getStatsDetails("1h"),
        apiClient.getStatsDetails("8h"),
        apiClient.getStatsDetails("24h")
      ]);
      setStats({
        "1h": oneHour,
        "8h": eightHours,
        "24h": day
      });
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "載入統計圖失敗");
    }
  }

  async function loadPoolStatus() {
    try {
      const data = await apiClient.getPoolStatus(8);
      setPoolStatus(data);
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "載入 Pool 狀態失敗");
    }
  }

  async function loadOverview() {
    try {
      const data = await apiClient.getDashboardOverview();
      setOverview(data);
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "載入 Dashboard 概覽失敗");
    }
  }

  async function updatePoolStrategy(strategy: PoolStrategy) {
    if (poolStatus?.strategy === strategy) {
      return;
    }
    setUpdatingPoolStrategy(true);
    try {
      const result = await apiClient.setPoolStrategy(strategy);
      setPoolStatus(result.pool);
      notifier.push("success", `Pool 策略已切換為 ${strategy}`);
      await Promise.all([loadKeys(), loadOverview()]);
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "切換 Pool 策略失敗");
    } finally {
      setUpdatingPoolStrategy(false);
    }
  }

  const validKeys = useMemo(() => keys.filter((item) => isValidKeyStatus(item.status)), [keys]);
  const invalidKeys = useMemo(() => keys.filter((item) => item.status === "invalid"), [keys]);

  const selectedIds = useMemo(
    () =>
      Object.entries(selected)
        .filter(([, checked]) => checked)
        .map(([id]) => id),
    [selected]
  );

  const selectedAttentionIds = useMemo(
    () =>
      Object.entries(selectedAttention)
        .filter(([, checked]) => checked)
        .map(([id]) => id),
    [selectedAttention]
  );

  const chartData = useMemo(() => {
    const current = stats[chartPeriod];
    return {
      labels: current?.series.map((item) => new Date(item.at).toLocaleTimeString("zh-TW", { hour: "2-digit", minute: "2-digit" })) ?? [],
      datasets: [
        {
          label: "成功",
          data: current?.series.map((item) => item.success) ?? [],
          borderColor: "#16a34a",
          backgroundColor: "rgba(22,163,74,0.16)",
          fill: true,
          tension: 0.3
        },
        {
          label: "失敗",
          data: current?.series.map((item) => item.failure) ?? [],
          borderColor: "#ef4444",
          backgroundColor: "rgba(239,68,68,0.15)",
          fill: true,
          tension: 0.3
        }
      ]
    };
  }, [chartPeriod, stats]);

  const chartOptions = useMemo(
    () => ({
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: {
          position: "top" as const
        }
      },
      scales: {
        y: { beginAtZero: true }
      }
    }),
    []
  );

  const health = overview?.health;
  const calls24h = overview?.callsSummary.twentyFourHours;
  const modelDistribution = overview?.modelDistribution24h ?? [];
  const statusDistribution = overview?.statusDistribution24h ?? [];
  const modelPoolEntries = useMemo(() => Object.entries(overview?.modelPools ?? {}), [overview?.modelPools]);
  const healthTone =
    health?.level === "healthy" ? "ok" : health?.level === "warning" ? "warn" : "bad";

  async function executeAction(action: "verify" | "reset" | "delete", ids: string[]) {
    if (ids.length === 0) {
      notifier.push("warning", "請先勾選要操作的 key");
      return;
    }
    setBusyAction(action);
    setProgressOpen(true);
    try {
      const result = await apiClient.keyAction(action, ids);
      setResultModal({
        open: true,
        title: `批次${action}結果`,
        message: `${result.message}，成功 ${result.successCount} 筆，失敗 ${result.failedItems.length} 筆`
      });
      await Promise.all([loadKeys(), loadAttentionKeys(), loadPoolStatus(), loadOverview()]);
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "批次操作失敗");
    } finally {
      setBusyAction("");
      setProgressOpen(false);
    }
  }

  async function copySelectedKeys() {
    if (selectedIds.length === 0) {
      notifier.push("warning", "請先勾選 key");
      return;
    }
    const selectedKeysRaw = keys.filter((item) => selectedIds.includes(item.id)).map((item) => item.key);
    await copyText(selectedKeysRaw.join("\n"));
    notifier.push("success", `已複製 ${selectedKeysRaw.length} 個 key`);
  }

  async function showUsage(keyId: string) {
    setUsageModal({ open: true, key: keyId, loading: true, usage: {}, period: "24h" });
    try {
      const resp = await apiClient.getKeyUsageDetails(keyId, "24h");
      setUsageModal((prev) => ({
        ...prev,
        loading: false,
        usage: resp.usage
      }));
    } catch (error) {
      setUsageModal((prev) => ({ ...prev, loading: false }));
      notifier.push("error", error instanceof Error ? error.message : "載入 key 用量失敗");
    }
  }

  async function showCallDetails(keyId: string, period: ChartPeriod = "24h") {
    setCallsModal({
      open: true,
      key: keyId,
      period,
      loading: true,
      rows: []
    });
    try {
      const rows = await apiClient.getKeyCallDetails(keyId, period);
      setCallsModal((prev) => ({
        ...prev,
        loading: false,
        rows
      }));
    } catch (error) {
      setCallsModal((prev) => ({ ...prev, loading: false }));
      notifier.push("error", error instanceof Error ? error.message : "載入 API 呼叫詳情失敗");
    }
  }

  function renderKeyRow(item: KeyRecordV2) {
    const selectedState = Boolean(selected[item.id]);
    return (
      <tr key={item.id}>
        <td>
          <input
            type="checkbox"
            checked={selectedState}
            onChange={(event) =>
              setSelected((prev) => ({
                ...prev,
                [item.id]: event.target.checked
              }))
            }
          />
        </td>
        <td>
          <div className="key-cell">
            <strong>{item.maskedKey}</strong>
            <small>{item.id}</small>
          </div>
        </td>
        <td>{item.failureCount}</td>
        <td>
          <span className={`tag ${item.status === "invalid" ? "tag-bad" : "tag-ok"}`}>
            {item.status}
          </span>
        </td>
        <td>
          <div className="actions">
            <button type="button" onClick={() => void copyText(item.key).then(() => notifier.push("success", "已複製 key"))}>
              複製
            </button>
            <button type="button" onClick={() => void executeAction("reset", [item.id])}>
              重置
            </button>
            <button
              type="button"
              onClick={() =>
                void apiClient
                  .verifySingleKey(item.id)
                  .then(() => Promise.all([loadKeys(), loadPoolStatus(), loadOverview()]))
                  .catch((e) => notifier.push("error", e.message))
              }
            >
              驗證
            </button>
            <button type="button" onClick={() => void showUsage(item.id)}>
              用量
            </button>
            <button type="button" onClick={() => void showCallDetails(item.id, "24h")}>
              呼叫詳情
            </button>
          </div>
        </td>
      </tr>
    );
  }

  return (
    <div className="panel-stack">
      <section className="panel">
        <div className="panel-head">
          <h3>Dashboard 總覽</h3>
          <div className="actions">
            <button
              type="button"
              onClick={() => void Promise.all([loadKeys(), loadAttentionKeys(), loadStats(), loadPoolStatus(), loadOverview()])}
            >
              <i className="fas fa-rotate" /> 重新整理
            </button>
          </div>
        </div>

        <div className={`dashboard-hero dashboard-hero-${healthTone}`}>
          <div className="health-score-card">
            <div className="health-score-ring">
              <strong>{health?.score ?? 0}</strong>
              <span>/100</span>
            </div>
            <div className="health-meta">
              <h4>系統健康度</h4>
              <p>
                等級：
                <span className={`tag ${healthTone === "ok" ? "tag-ok" : healthTone === "warn" ? "tag-warn" : "tag-bad"}`}>
                  {health?.level ?? "critical"}
                </span>
              </p>
              <p>24h 失敗率：{Math.round((health?.failureRate24h ?? 0) * 100)}%</p>
            </div>
          </div>
          <div className="hero-metrics">
            <div className="hero-stat">
              <span>24h 呼叫量</span>
              <strong>{health?.totalCalls24h ?? 0}</strong>
            </div>
            <div className="hero-stat">
              <span>24h 成功/失敗</span>
              <strong>
                {calls24h?.success ?? 0} / {calls24h?.failure ?? 0}
              </strong>
            </div>
            <div className="hero-stat">
              <span>高風險 Keys</span>
              <strong>{overview?.attentionKeys.length ?? attentionKeys.length}</strong>
            </div>
            <div className="hero-stat">
              <span>Model Alias 數</span>
              <strong>{modelPoolEntries.length}</strong>
            </div>
          </div>
        </div>

        <div className="pool-overview-card">
          <div className="pool-overview-head">
            <div>
              <h4>單一調用 URL</h4>
              <code>{apiUrl || "載入中..."}</code>
            </div>
            <span className="tag tag-info">策略：{poolStatus?.strategy ?? "round_robin"}</span>
          </div>
          <div className="pool-strategy-row">
            {POOL_STRATEGY_OPTIONS.map((item) => (
              <button
                key={item.value}
                type="button"
                className={poolStatus?.strategy === item.value ? "active" : ""}
                disabled={updatingPoolStrategy}
                onClick={() => void updatePoolStrategy(item.value)}
                title={item.desc}
              >
                {item.label}
              </button>
            ))}
          </div>
          <div className="stats-grid four compact">
            <div className="stat-card">
              <span>Pool 總 Key</span>
              <strong>{poolStatus?.totalKeys ?? 0}</strong>
            </div>
            <div className="stat-card success">
              <span>可用 Key</span>
              <strong>{poolStatus?.availableKeys ?? 0}</strong>
            </div>
            <div className="stat-card warning">
              <span>冷卻中</span>
              <strong>{poolStatus?.cooldownKeys ?? 0}</strong>
            </div>
            <div className="stat-card danger">
              <span>無效 Key</span>
              <strong>{poolStatus?.invalidKeys ?? 0}</strong>
            </div>
          </div>
          {poolStatus?.lastSelected && (
            <p className="pool-last-selected">
              最近選擇：
              <strong>{poolStatus.lastSelected.maskedKey}</strong>
              <span>{new Date(poolStatus.lastSelected.at).toLocaleString()}</span>
            </p>
          )}
        </div>

        <div className="insight-grid">
          <div className="chart-card">
            <div className="chart-head">
              <h4>API 呼叫趨勢</h4>
              <div className="actions">
                <button type="button" className={chartPeriod === "1h" ? "active" : ""} onClick={() => setChartPeriod("1h")}>
                  1h
                </button>
                <button type="button" className={chartPeriod === "8h" ? "active" : ""} onClick={() => setChartPeriod("8h")}>
                  8h
                </button>
                <button type="button" className={chartPeriod === "24h" ? "active" : ""} onClick={() => setChartPeriod("24h")}>
                  24h
                </button>
              </div>
            </div>
            <div className="chart-container">
              <Line data={chartData} options={chartOptions} />
            </div>
          </div>

          <div className="insight-card">
            <h4>模型分布（24h）</h4>
            {modelDistribution.length === 0 && <p className="muted-text">目前尚無呼叫資料</p>}
            {modelDistribution.length > 0 && (
              <div className="mini-table">
                {modelDistribution.slice(0, 6).map((item) => (
                  <div key={item.model} className="mini-row">
                    <div className="mini-main">
                      <strong>{item.model}</strong>
                      <small>
                        {item.success} / {item.total} 成功
                      </small>
                    </div>
                    <span>{Math.round(item.successRate * 100)}%</span>
                  </div>
                ))}
              </div>
            )}

            <h4>狀態碼分布（24h）</h4>
            <div className="status-chip-grid">
              {statusDistribution.slice(0, 8).map((item) => (
                <span
                  key={item.statusCode}
                  className={`status-chip ${
                    item.statusCode >= 500 ? "is-bad" : item.statusCode >= 400 ? "is-warn" : "is-ok"
                  }`}
                >
                  {item.statusCode} · {item.count}
                </span>
              ))}
              {statusDistribution.length === 0 && <span className="status-chip">無資料</span>}
            </div>

            <h4>近期錯誤</h4>
            <div className="mini-table">
              {(overview?.recentErrors ?? []).slice(0, 5).map((item) => (
                <div key={item.id} className="mini-row">
                  <div className="mini-main">
                    <strong>{item.model}</strong>
                    <small>{item.maskedKey}</small>
                  </div>
                  <span>{item.statusCode}</span>
                </div>
              ))}
              {(overview?.recentErrors ?? []).length === 0 && <p className="muted-text">目前沒有錯誤紀錄</p>}
            </div>
          </div>
        </div>

        <div className="stats-grid four">
          <div className="stat-card">
            <span>總 Key</span>
            <strong>{summary.total}</strong>
          </div>
          <div className="stat-card success">
            <span>可用率</span>
            <strong>{Math.round((health?.activeKeyRatio ?? 0) * 100)}%</strong>
          </div>
          <div className="stat-card warning">
            <span>冷卻率</span>
            <strong>{Math.round((health?.cooldownKeyRatio ?? 0) * 100)}%</strong>
          </div>
          <div className="stat-card danger">
            <span>無效率</span>
            <strong>{Math.round((health?.invalidKeyRatio ?? 0) * 100)}%</strong>
          </div>
        </div>

        <div className="model-pool-card">
          <div className="panel-head">
            <h3>Model Pool 映射</h3>
          </div>
          <div className="model-pool-grid">
            {modelPoolEntries.map(([alias, models]) => (
              <div className="model-pool-item" key={alias}>
                <strong>{alias}</strong>
                <p>{models.join(" → ")}</p>
              </div>
            ))}
            {modelPoolEntries.length === 0 && <p className="muted-text">尚未設定 MODEL_POOLS</p>}
          </div>
        </div>
      </section>

      <section className="panel">
        <div className="panel-head">
          <h3>Attention Keys</h3>
          <div className="toolbar">
            <label>
              狀態碼
              <select
                value={attentionStatusCode}
                onChange={(event) => setAttentionStatusCode(Number(event.target.value))}
              >
                <option value={429}>429</option>
                <option value={401}>401</option>
                <option value={403}>403</option>
                <option value={503}>503</option>
              </select>
            </label>
            <label>
              筆數
              <select value={attentionLimit} onChange={(event) => setAttentionLimit(Number(event.target.value))}>
                <option value={10}>10</option>
                <option value={20}>20</option>
                <option value={50}>50</option>
              </select>
            </label>
            <button
              type="button"
              onClick={() =>
                setSelectedAttention(
                  Object.fromEntries(
                    attentionKeys.map((item) => [item.key, !selectedAttention[item.key]])
                  )
                )
              }
            >
              全選/全不選
            </button>
            <button
              type="button"
              onClick={() =>
                void apiClient
                  .keyActionByKeys("verify", selectedAttentionIds)
                  .then(() => Promise.all([loadKeys(), loadAttentionKeys(), loadPoolStatus(), loadOverview()]))
                  .catch((error) => notifier.push("error", error instanceof Error ? error.message : "批次驗證失敗"))
              }
            >
              批次驗證
            </button>
            <button
              type="button"
              onClick={() =>
                void apiClient
                  .keyActionByKeys("reset", selectedAttentionIds)
                  .then(() => Promise.all([loadKeys(), loadAttentionKeys(), loadPoolStatus(), loadOverview()]))
                  .catch((error) => notifier.push("error", error instanceof Error ? error.message : "批次重置失敗"))
              }
            >
              批次重置
            </button>
          </div>
        </div>
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th />
                <th>Key</th>
                <th>次數</th>
                <th>狀態碼</th>
                <th>最後時間</th>
              </tr>
            </thead>
            <tbody>
              {attentionKeys.map((item) => (
                <tr key={`${item.key}-${item.statusCode}`}>
                  <td>
                    <input
                      type="checkbox"
                      checked={Boolean(selectedAttention[item.key])}
                      onChange={(event) =>
                        setSelectedAttention((prev) => ({
                          ...prev,
                          [item.key]: event.target.checked
                        }))
                      }
                    />
                  </td>
                  <td>{item.maskedKey || item.key}</td>
                  <td>{item.count}</td>
                  <td>{item.statusCode}</td>
                  <td>{item.lastAt ? new Date(item.lastAt).toLocaleString() : "-"}</td>
                </tr>
              ))}
              {attentionKeys.length === 0 && (
                <tr>
                  <td colSpan={5} className="empty">
                    目前沒有值得注意的 key
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </section>

      <section className="panel">
        <div className="panel-head">
          <h3>Key 列表</h3>
          <div className="toolbar">
            <input
              placeholder="搜尋 key / id"
              value={search}
              onChange={(event) => {
                setPage(1);
                setSearch(event.target.value);
              }}
            />
            <select
              value={statusFilter}
              onChange={(event) => {
                setPage(1);
                setStatusFilter(event.target.value);
              }}
            >
              <option value="all">全部</option>
              <option value="valid">有效</option>
              <option value="invalid">無效</option>
              <option value="cooldown">冷卻中</option>
            </select>
            <input
              type="number"
              min={0}
              placeholder="失敗次數門檻"
              value={minFailureCount}
              onChange={(event) => {
                setPage(1);
                const value = event.target.value;
                setMinFailureCount(value === "" ? "" : Number(value));
              }}
            />
            <button
              type="button"
              onClick={() =>
                setSelected(Object.fromEntries(keys.map((item) => [item.id, !selected[item.id]])))
              }
            >
              全選/全不選
            </button>
            <button type="button" onClick={() => void executeAction("verify", selectedIds)} disabled={busyAction !== ""}>
              批次驗證
            </button>
            <button type="button" onClick={() => void executeAction("reset", selectedIds)} disabled={busyAction !== ""}>
              批次重置
            </button>
            <button type="button" onClick={() => void copySelectedKeys()}>
              批次複製
            </button>
            <button type="button" className="danger" onClick={() => void executeAction("delete", selectedIds)} disabled={busyAction !== ""}>
              批次刪除
            </button>
          </div>
        </div>

        <div className="collapsible-head">
          <button type="button" onClick={() => setExpandValid((prev) => !prev)}>
            <i className={`fas ${expandValid ? "fa-angle-down" : "fa-angle-right"}`} /> 有效 Key ({validKeys.length})
          </button>
        </div>
        {expandValid && (
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th />
                  <th>Key</th>
                  <th>失敗</th>
                  <th>狀態</th>
                  <th>操作</th>
                </tr>
              </thead>
              <tbody>{validKeys.map((item) => renderKeyRow(item))}</tbody>
            </table>
          </div>
        )}

        <div className="collapsible-head">
          <button type="button" onClick={() => setExpandInvalid((prev) => !prev)}>
            <i className={`fas ${expandInvalid ? "fa-angle-down" : "fa-angle-right"}`} /> 無效 Key ({invalidKeys.length})
          </button>
        </div>
        {expandInvalid && (
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th />
                  <th>Key</th>
                  <th>失敗</th>
                  <th>狀態</th>
                  <th>操作</th>
                </tr>
              </thead>
              <tbody>{invalidKeys.map((item) => renderKeyRow(item))}</tbody>
            </table>
          </div>
        )}
        {!loading && keys.length === 0 && <p className="empty">沒有符合條件的 key</p>}
        {loading && <p className="muted-text">載入中...</p>}

        <div className="pager">
          <button type="button" disabled={page <= 1} onClick={() => setPage((prev) => prev - 1)}>
            上一頁
          </button>
          <span>
            第 {page} / {totalPages} 頁
          </span>
          <button type="button" disabled={page >= totalPages} onClick={() => setPage((prev) => prev + 1)}>
            下一頁
          </button>
        </div>
      </section>

      <Modal open={progressOpen} title="批次操作進行中" onClose={() => undefined}>
        <div className="progress-wrap">
          <div className="spinner" />
          <p>正在執行 {busyAction || "操作"}，請稍候...</p>
        </div>
      </Modal>

      <Modal
        open={resultModal.open}
        title={resultModal.title}
        onClose={() => setResultModal({ open: false, title: "", message: "" })}
      >
        <p>{resultModal.message}</p>
      </Modal>

      <Modal
        open={usageModal.open}
        title={`Key 用量詳情：${usageModal.key}`}
        onClose={() => setUsageModal((prev) => ({ ...prev, open: false }))}
      >
        {usageModal.loading && <p>載入中...</p>}
        {!usageModal.loading && Object.keys(usageModal.usage).length === 0 && <p className="empty">沒有資料</p>}
        {!usageModal.loading && Object.keys(usageModal.usage).length > 0 && (
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>模型</th>
                  <th>次數</th>
                </tr>
              </thead>
              <tbody>
                {Object.entries(usageModal.usage).map(([model, count]) => (
                  <tr key={model}>
                    <td>{model}</td>
                    <td>{count}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Modal>

      <Modal
        open={callsModal.open}
        title={`API 呼叫詳情：${callsModal.key}`}
        wide
        onClose={() => setCallsModal((prev) => ({ ...prev, open: false }))}
        footer={
          <div className="actions">
            <button type="button" onClick={() => void showCallDetails(callsModal.key, "1h")}>
              1h
            </button>
            <button type="button" onClick={() => void showCallDetails(callsModal.key, "8h")}>
              8h
            </button>
            <button type="button" onClick={() => void showCallDetails(callsModal.key, "24h")}>
              24h
            </button>
          </div>
        }
      >
        {callsModal.loading && <p>載入中...</p>}
        {!callsModal.loading && callsModal.rows.length === 0 && <p className="empty">沒有可顯示資料</p>}
        {!callsModal.loading && callsModal.rows.length > 0 && (
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>時間</th>
                  <th>模型</th>
                  <th>狀態碼</th>
                  <th>狀態</th>
                  <th>耗時(ms)</th>
                </tr>
              </thead>
              <tbody>
                {callsModal.rows.map((row, idx) => (
                  <tr key={`${String(row.timestamp)}-${idx}`}>
                    <td>{new Date(String(row.timestamp ?? "")).toLocaleString()}</td>
                    <td>{String(row.model ?? "-")}</td>
                    <td>{String(row.status_code ?? "-")}</td>
                    <td>{String(row.status ?? "-")}</td>
                    <td>{String(row.latency_ms ?? "-")}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Modal>
    </div>
  );
}
