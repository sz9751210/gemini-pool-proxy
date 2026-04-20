import { useEffect, useState } from "react";
import { apiClient, runtimeBase } from "../lib/apiClient";
import { useNotifier } from "./NotificationProvider";

export function DesktopControls() {
  const notifier = useNotifier();
  const [status, setStatus] = useState("unknown");
  const [baseUrl, setBaseUrl] = useState("");
  const [importPath, setImportPath] = useState("../../.env");
  const [busy, setBusy] = useState<"" | "start" | "stop" | "import">("");

  async function refresh() {
    try {
      const [nextStatus, nextBase] = await Promise.all([apiClient.gatewayStatus(), runtimeBase()]);
      setStatus(nextStatus);
      setBaseUrl(nextBase);
    } catch {
      setStatus("unknown");
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  async function startGateway() {
    setBusy("start");
    try {
      await apiClient.startGateway();
      notifier.push("success", "服務已啟動");
      await refresh();
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "啟動失敗");
    } finally {
      setBusy("");
    }
  }

  async function stopGateway() {
    setBusy("stop");
    try {
      await apiClient.stopGateway();
      notifier.push("success", "服務已停止");
      await refresh();
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "停止失敗");
    } finally {
      setBusy("");
    }
  }

  async function importEnv() {
    setBusy("import");
    try {
      const result = await apiClient.importEnv(importPath.trim());
      notifier.push(
        "success",
        `已匯入 ${result.imported_count} 個設定，保存於 ${result.secure_path}`
      );
      await refresh();
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "匯入 .env 失敗");
    } finally {
      setBusy("");
    }
  }

  const running = status === "running";

  return (
    <div className="desktop-controls">
      <span className={`health-dot ${running ? "is-running" : "is-stopped"}`} title={status} />
      <span className="muted-text">服務：{status}</span>
      <button type="button" onClick={() => void startGateway()} disabled={busy !== ""}>
        {busy === "start" ? "啟動中..." : "Start"}
      </button>
      <button type="button" onClick={() => void stopGateway()} disabled={busy !== ""}>
        {busy === "stop" ? "停止中..." : "Stop"}
      </button>
      <button type="button" onClick={() => void refresh()} disabled={busy !== ""}>
        Refresh
      </button>
      <div className="desktop-import">
        <input
          value={importPath}
          onChange={(event) => setImportPath(event.target.value)}
          placeholder="請輸入 .env 路徑"
        />
        <button type="button" onClick={() => void importEnv()} disabled={busy !== ""}>
          {busy === "import" ? "匯入中..." : "Import .env"}
        </button>
      </div>
      <span className="muted-text">Runtime: {baseUrl}</span>
    </div>
  );
}
