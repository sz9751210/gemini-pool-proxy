import { useEffect, useMemo, useState } from "react";
import { Modal } from "../components/Modal";
import { useNotifier } from "../components/NotificationProvider";
import { apiClient } from "../lib/apiClient";
import type {
  ConfigSchemaFieldV2,
  ConfigSchemaV2,
  PoolStatus,
  PoolStrategy,
  ProxyCacheStatsV2,
  ProxyCheckResultV2,
  SchedulerStatusV2
} from "../lib/types";

type BulkModalState = {
  open: boolean;
  target: "API_KEYS" | "PROXIES";
  mode: "add" | "delete";
  text: string;
};

function normalizeLines(text: string) {
  return text
    .split(/[\n,]+/g)
    .map((value) => value.trim())
    .filter((value) => value.length > 0);
}

function asStringArray(value: unknown): string[] {
  if (Array.isArray(value)) {
    return value.map((item) => String(item));
  }
  if (typeof value === "string") {
    return normalizeLines(value);
  }
  return [];
}

const POOL_STRATEGY_OPTIONS: Array<{ value: PoolStrategy; label: string }> = [
  { value: "round_robin", label: "Round Robin" },
  { value: "random", label: "Random" },
  { value: "least_fail", label: "Least Fail" }
];

export function ConfigPage() {
  const notifier = useNotifier();
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [schema, setSchema] = useState<ConfigSchemaV2 | null>(null);
  const [activeTab, setActiveTab] = useState("api");
  const [config, setConfig] = useState<Record<string, unknown>>({});
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [uiModels, setUiModels] = useState<Array<Record<string, unknown>>>([]);
  const [modelHelperTarget, setModelHelperTarget] = useState<"MODEL_NAME" | "THINKING_MODELS" | "">("");
  const [bulkModal, setBulkModal] = useState<BulkModalState>({
    open: false,
    target: "API_KEYS",
    mode: "add",
    text: ""
  });
  const [schedulerStatus, setSchedulerStatus] = useState<SchedulerStatusV2 | null>(null);
  const [poolStatus, setPoolStatus] = useState<PoolStatus | null>(null);
  const [poolUpdating, setPoolUpdating] = useState(false);

  const [singleProxy, setSingleProxy] = useState("");
  const [singleProxyResult, setSingleProxyResult] = useState<ProxyCheckResultV2 | null>(null);
  const [batchProxyResult, setBatchProxyResult] = useState<ProxyCheckResultV2[]>([]);
  const [proxyCacheStats, setProxyCacheStats] = useState<ProxyCacheStatsV2 | null>(null);
  const [proxyChecking, setProxyChecking] = useState(false);

  useEffect(() => {
    void loadAll();
  }, []);

  const sections = schema?.sections ?? [];
  const activeSection = useMemo(
    () => sections.find((section) => section.id === activeTab) ?? sections[0],
    [activeTab, sections]
  );

  async function loadAll() {
    setLoading(true);
    try {
      const [configResp, schemaResp, uiModelResp, schedulerResp, proxyStatsResp, poolResp] = await Promise.all([
        apiClient.getConfig(),
        apiClient.getConfigSchema(),
        apiClient.getUIModels(),
        apiClient.schedulerStatus(),
        apiClient.getProxyCacheStats(),
        apiClient.getPoolStatus(8)
      ]);
      setConfig(configResp);
      setSchema(schemaResp);
      setUiModels(uiModelResp.data ?? []);
      setSchedulerStatus(schedulerResp);
      setProxyCacheStats(proxyStatsResp);
      setPoolStatus(poolResp);
      setActiveTab((prev) => prev || schemaResp.sections[0]?.id || "api");

      const nextDrafts: Record<string, string> = {};
      for (const [key, value] of Object.entries(configResp)) {
        if (Array.isArray(value) || (typeof value === "object" && value !== null)) {
          nextDrafts[key] = JSON.stringify(value, null, 2);
        }
      }
      setDrafts(nextDrafts);
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "載入配置頁資料失敗");
    } finally {
      setLoading(false);
    }
  }

  function updatePrimitive(key: string, value: string, type: ConfigSchemaFieldV2["fieldType"]) {
    setConfig((prev) => {
      if (type === "number") {
        return { ...prev, [key]: Number(value) };
      }
      return { ...prev, [key]: value };
    });
  }

  function updateDraft(key: string, value: string) {
    setDrafts((prev) => ({
      ...prev,
      [key]: value
    }));
  }

  function updateArrayField(key: string, items: string[]) {
    setConfig((prev) => ({
      ...prev,
      [key]: items
    }));
    setDrafts((prev) => ({
      ...prev,
      [key]: JSON.stringify(items, null, 2)
    }));
  }

  async function saveConfig() {
    setSaving(true);
    try {
      const payload: Record<string, unknown> = { ...config };
      for (const [key, text] of Object.entries(drafts)) {
        if (!text.trim()) {
          continue;
        }
        try {
          payload[key] = JSON.parse(text);
        } catch (error) {
          throw new Error(`${key} JSON 格式錯誤`);
        }
      }
      await apiClient.saveConfig(payload);
      notifier.push("success", "配置已儲存");
      await loadAll();
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "儲存失敗");
    } finally {
      setSaving(false);
    }
  }

  async function resetConfig() {
    try {
      await apiClient.resetConfig();
      notifier.push("success", "配置已重置");
      await loadAll();
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "重置配置失敗");
    }
  }

  async function runScheduler(action: "start" | "stop") {
    try {
      if (action === "start") {
        await apiClient.startScheduler();
      } else {
        await apiClient.stopScheduler();
      }
      notifier.push("success", `排程已${action === "start" ? "啟動" : "停止"}`);
      const status = await apiClient.schedulerStatus();
      setSchedulerStatus(status);
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "排程操作失敗");
    }
  }

  async function updatePoolStrategy(strategy: PoolStrategy) {
    setPoolUpdating(true);
    try {
      const result = await apiClient.setPoolStrategy(strategy);
      setPoolStatus(result.pool);
      setConfig((prev) => ({ ...prev, POOL_STRATEGY: strategy }));
      notifier.push("success", `Pool 策略已更新為 ${strategy}`);
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "更新 Pool 策略失敗");
    } finally {
      setPoolUpdating(false);
    }
  }

  async function submitBulkAction() {
    const lines = normalizeLines(bulkModal.text);
    if (lines.length === 0) {
      notifier.push("warning", "請輸入至少一筆資料");
      return;
    }
    try {
      if (bulkModal.target === "API_KEYS") {
        if (bulkModal.mode === "add") {
          await apiClient.addKeys(lines);
        } else {
          await apiClient.deleteKeys(lines);
        }
      } else if (bulkModal.target === "PROXIES") {
        if (bulkModal.mode === "add") {
          await apiClient.addProxies(lines);
        } else {
          await apiClient.deleteProxies(lines);
        }
      }
      notifier.push("success", "批次操作已完成");
      setBulkModal((prev) => ({ ...prev, open: false, text: "" }));
      await loadAll();
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "批次操作失敗");
    }
  }

  function applyModelSuggestion(modelId: string) {
    if (modelHelperTarget === "MODEL_NAME") {
      setConfig((prev) => ({ ...prev, MODEL_NAME: modelId }));
      notifier.push("success", "已套用到 MODEL_NAME");
      setModelHelperTarget("");
      return;
    }
    if (modelHelperTarget === "THINKING_MODELS") {
      const next = asStringArray(config.THINKING_MODELS);
      if (!next.includes(modelId)) {
        next.push(modelId);
      }
      updateArrayField("THINKING_MODELS", next);
      notifier.push("success", "已加入 THINKING_MODELS");
      setModelHelperTarget("");
    }
  }

  async function checkSingleProxy() {
    if (!singleProxy.trim()) {
      notifier.push("warning", "請輸入代理 URL");
      return;
    }
    setProxyChecking(true);
    try {
      const result = await apiClient.checkProxy(singleProxy.trim(), false);
      setSingleProxyResult(result);
      notifier.push("success", "單筆代理檢測完成");
      setProxyCacheStats(await apiClient.getProxyCacheStats());
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "代理檢測失敗");
    } finally {
      setProxyChecking(false);
    }
  }

  async function checkAllProxies() {
    const proxies = asStringArray(config.PROXIES);
    if (proxies.length === 0) {
      notifier.push("warning", "目前沒有可檢測的 PROXIES");
      return;
    }
    setProxyChecking(true);
    try {
      const result = await apiClient.checkAllProxies(proxies, false);
      setBatchProxyResult(result);
      notifier.push("success", `完成 ${result.length} 筆代理檢測`);
      setProxyCacheStats(await apiClient.getProxyCacheStats());
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "批次代理檢測失敗");
    } finally {
      setProxyChecking(false);
    }
  }

  async function clearProxyCache() {
    try {
      await apiClient.clearProxyCache();
      notifier.push("success", "Proxy 快取已清除");
      setProxyCacheStats(await apiClient.getProxyCacheStats());
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "清除快取失敗");
    }
  }

  function renderField(field: ConfigSchemaFieldV2) {
    const currentValue = config[field.key];
    const type = field.fieldType;
    const options = field.uiHints.options ?? [];
    return (
      <div className="field-card" key={field.key}>
        <div className="field-meta">
          <strong>{field.label || field.key}</strong>
          <small>{field.key}</small>
          {field.uiHints.help && <p>{field.uiHints.help}</p>}
        </div>

        {type === "boolean" && (
          <label className="switch-line">
            <input
              type="checkbox"
              checked={Boolean(currentValue)}
              onChange={(event) =>
                setConfig((prev) => ({ ...prev, [field.key]: event.target.checked }))
              }
            />
            <span>{Boolean(currentValue) ? "已啟用" : "未啟用"}</span>
          </label>
        )}

        {(type === "string" || type === "number") && options.length > 0 && (
          <select
            value={String(currentValue ?? "")}
            onChange={(event) => updatePrimitive(field.key, event.target.value, type)}
          >
            {options.map((item) => (
              <option key={item} value={item}>
                {item}
              </option>
            ))}
          </select>
        )}

        {(type === "string" || type === "number") && options.length === 0 && (
          <input
            type={type === "number" ? "number" : field.uiHints.secret ? "password" : "text"}
            value={String(currentValue ?? "")}
            placeholder={field.uiHints.placeholder ?? ""}
            onChange={(event) => updatePrimitive(field.key, event.target.value, type)}
          />
        )}

        {(type === "array" || type === "object") && (
          <textarea
            rows={8}
            value={drafts[field.key] ?? JSON.stringify(currentValue ?? (type === "array" ? [] : {}), null, 2)}
            onChange={(event) => updateDraft(field.key, event.target.value)}
          />
        )}

        {field.key === "API_KEYS" && (
          <div className="actions">
            <button
              type="button"
              onClick={() => setBulkModal({ open: true, target: "API_KEYS", mode: "add", text: "" })}
            >
              批量新增
            </button>
            <button
              type="button"
              onClick={() => setBulkModal({ open: true, target: "API_KEYS", mode: "delete", text: "" })}
            >
              批量刪除
            </button>
          </div>
        )}

        {field.key === "PROXIES" && (
          <div className="actions">
            <button
              type="button"
              onClick={() => setBulkModal({ open: true, target: "PROXIES", mode: "add", text: "" })}
            >
              批量新增
            </button>
            <button
              type="button"
              onClick={() => setBulkModal({ open: true, target: "PROXIES", mode: "delete", text: "" })}
            >
              批量刪除
            </button>
            <button type="button" onClick={() => void checkAllProxies()} disabled={proxyChecking}>
              批次檢測
            </button>
          </div>
        )}

        {field.key === "MODEL_NAME" && (
          <button type="button" onClick={() => setModelHelperTarget("MODEL_NAME")}>
            模型助手
          </button>
        )}

        {field.key === "THINKING_MODELS" && (
          <button type="button" onClick={() => setModelHelperTarget("THINKING_MODELS")}>
            Thinking 模型助手
          </button>
        )}
      </div>
    );
  }

  return (
    <div className="panel-stack">
      <section className="panel">
        <div className="panel-head">
          <h3>配置管理</h3>
          <div className="actions">
            <button type="button" onClick={() => void runScheduler("start")}>
              啟動排程
            </button>
            <button type="button" onClick={() => void runScheduler("stop")}>
              停止排程
            </button>
            <button type="button" onClick={() => void loadAll()} disabled={loading}>
              重新載入
            </button>
            <button type="button" className="danger" onClick={() => void resetConfig()}>
              重置
            </button>
            <button type="button" className="primary" onClick={() => void saveConfig()} disabled={saving}>
              {saving ? "儲存中..." : "儲存配置"}
            </button>
          </div>
        </div>
        <div className="meta-grid">
          <div>
            <span>Scheduler</span>
            <strong>{schedulerStatus?.running ? "Running" : "Stopped"}</strong>
          </div>
          <div>
            <span>Schema fields</span>
            <strong>{schema?.fieldCount ?? 0}</strong>
          </div>
          <div>
            <span>Proxy cache</span>
            <strong>{proxyCacheStats?.totalCached ?? 0}</strong>
          </div>
        </div>

        <div className="pool-config-card">
          <div className="panel-head">
            <h4>Key Pool 策略</h4>
          </div>
          <div className="actions">
            {POOL_STRATEGY_OPTIONS.map((item) => (
              <button
                key={item.value}
                type="button"
                className={poolStatus?.strategy === item.value ? "active" : ""}
                disabled={poolUpdating}
                onClick={() => void updatePoolStrategy(item.value)}
              >
                {item.label}
              </button>
            ))}
          </div>
          <p className="muted-text">
            目前策略：<strong>{poolStatus?.strategy ?? String(config.POOL_STRATEGY ?? "round_robin")}</strong>
          </p>
        </div>
      </section>

      <section className="panel split">
        <aside className="tab-list">
          {sections.map((section) => (
            <button
              key={section.id}
              type="button"
              className={activeSection?.id === section.id ? "active" : ""}
              onClick={() => setActiveTab(section.id)}
            >
              {section.name}
            </button>
          ))}
        </aside>

        <div className="tab-content">
          {!activeSection && <p className="empty">沒有可用欄位</p>}
          {activeSection?.fields.map((field) => renderField(field))}
        </div>
      </section>

      <section className="panel">
        <div className="panel-head">
          <h3>Proxy 檢測</h3>
          <div className="actions">
            <button type="button" onClick={() => void checkAllProxies()} disabled={proxyChecking}>
              全部檢測
            </button>
            <button type="button" onClick={() => void clearProxyCache()}>
              清除快取
            </button>
          </div>
        </div>

        <div className="toolbar">
          <input
            value={singleProxy}
            onChange={(event) => setSingleProxy(event.target.value)}
            placeholder="http://user:pass@host:port"
          />
          <button type="button" onClick={() => void checkSingleProxy()} disabled={proxyChecking}>
            檢測單筆
          </button>
        </div>

        {singleProxyResult && (
          <div className="proxy-result-card">
            <strong>{singleProxyResult.proxy}</strong>
            <span className={`tag ${singleProxyResult.isAvailable ? "tag-ok" : "tag-bad"}`}>
              {singleProxyResult.isAvailable ? "可用" : "不可用"}
            </span>
            <small>
              延遲: {singleProxyResult.responseTime ?? "-"} / {singleProxyResult.errorMessage ?? "OK"}
            </small>
          </div>
        )}

        {proxyCacheStats && (
          <div className="stats-grid three">
            <div className="stat-card">
              <span>快取總數</span>
              <strong>{proxyCacheStats.totalCached}</strong>
            </div>
            <div className="stat-card success">
              <span>有效快取</span>
              <strong>{proxyCacheStats.validCached}</strong>
            </div>
            <div className="stat-card warning">
              <span>過期快取</span>
              <strong>{proxyCacheStats.expiredCached}</strong>
            </div>
          </div>
        )}

        {batchProxyResult.length > 0 && (
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Proxy</th>
                  <th>可用</th>
                  <th>延遲(s)</th>
                  <th>訊息</th>
                </tr>
              </thead>
              <tbody>
                {batchProxyResult.map((item) => (
                  <tr key={item.proxy}>
                    <td>{item.proxy}</td>
                    <td>
                      <span className={`tag ${item.isAvailable ? "tag-ok" : "tag-bad"}`}>
                        {item.isAvailable ? "可用" : "不可用"}
                      </span>
                    </td>
                    <td>{item.responseTime ?? "-"}</td>
                    <td>{item.errorMessage ?? "-"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      <Modal
        open={bulkModal.open}
        title={`${bulkModal.target} 批量${bulkModal.mode === "add" ? "新增" : "刪除"}`}
        onClose={() => setBulkModal((prev) => ({ ...prev, open: false }))}
        footer={
          <div className="actions">
            <button type="button" onClick={() => setBulkModal((prev) => ({ ...prev, open: false }))}>
              取消
            </button>
            <button type="button" className="primary" onClick={() => void submitBulkAction()}>
              確認
            </button>
          </div>
        }
      >
        <p>每行一筆資料，支援換行或逗號分隔。</p>
        <textarea
          rows={10}
          value={bulkModal.text}
          onChange={(event) => setBulkModal((prev) => ({ ...prev, text: event.target.value }))}
        />
      </Modal>

      <Modal
        open={modelHelperTarget !== ""}
        title="模型助手"
        onClose={() => setModelHelperTarget("")}
        wide
      >
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>ID</th>
                <th>名稱</th>
                <th>分類</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {uiModels.map((item) => {
                const id = String(item.id ?? item.model ?? "");
                return (
                  <tr key={id}>
                    <td>{id}</td>
                    <td>{String(item.label ?? "-")}</td>
                    <td>{String(item.category ?? "-")}</td>
                    <td>
                      <button type="button" onClick={() => applyModelSuggestion(id)}>
                        使用
                      </button>
                    </td>
                  </tr>
                );
              })}
              {uiModels.length === 0 && (
                <tr>
                  <td colSpan={4} className="empty">
                    沒有可用模型資料
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </Modal>
    </div>
  );
}
