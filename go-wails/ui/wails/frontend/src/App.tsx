import { useEffect, useState } from "react";
import {
  GetConfig,
  GetDashboardOverview,
  GetHealth,
  GetLogs,
  GetPoolStatus,
  SaveConfig,
  StartService,
  StopService,
} from "../wailsjs/go/wails/App";

type Health = {
  running: boolean;
  listenAddr: string;
  lastError: string;
  lastChangeAt: string;
};

export default function App() {
  const [health, setHealth] = useState<Health | null>(null);
  const [authToken, setAuthToken] = useState("");
  const [allowedTokens, setAllowedTokens] = useState("");
  const [dashboard, setDashboard] = useState<any>(null);
  const [logs, setLogs] = useState<any[]>([]);
  const [pool, setPool] = useState<any>(null);

  useEffect(() => {
    GetConfig().then((cfg: any) => {
      setAuthToken(cfg.AuthToken || "");
      setAllowedTokens((cfg.AllowedTokens || []).join(","));
    });
    GetHealth().then((h: Health) => setHealth(h));
    GetDashboardOverview().then(setDashboard);
    GetLogs(20, 0).then((items: any) => setLogs(Array.isArray(items) ? items : []));
    GetPoolStatus().then(setPool);
  }, []);

  async function onSave() {
    await SaveConfig({
      AuthToken: authToken,
      AllowedTokens: allowedTokens.split(",").map((v) => v.trim()).filter(Boolean),
    });
    setHealth(await GetHealth());
  }

  return (
    <main style={{ padding: 24, fontFamily: "IBM Plex Sans, sans-serif" }}>
      <h1>Gemini Pool Proxy - Phase 1</h1>
      <p>Status: {health?.running ? "Running" : "Stopped"} ({health?.listenAddr})</p>
      <label>Admin Token</label>
      <input value={authToken} onChange={(e) => setAuthToken(e.target.value)} />
      <label>Allowed Tokens (CSV)</label>
      <input value={allowedTokens} onChange={(e) => setAllowedTokens(e.target.value)} />
      <div style={{ marginTop: 12, display: "flex", gap: 8 }}>
        <button onClick={onSave}>Save Config</button>
        <button onClick={() => StartService().then(() => GetHealth().then(setHealth))}>Start</button>
        <button onClick={() => StopService().then(() => GetHealth().then(setHealth))}>Stop</button>
        <button onClick={() => GetDashboardOverview().then(setDashboard)}>Refresh Dashboard</button>
        <button onClick={() => GetLogs(20, 0).then((items: any) => setLogs(Array.isArray(items) ? items : []))}>Refresh Logs</button>
        <button onClick={() => GetPoolStatus().then(setPool)}>Refresh Pool</button>
      </div>
      <section style={{ marginTop: 24 }}>
        <h2>Dashboard</h2>
        <pre>{JSON.stringify(dashboard, null, 2)}</pre>
      </section>
      <section style={{ marginTop: 24 }}>
        <h2>Pool Status</h2>
        <pre>{JSON.stringify(pool, null, 2)}</pre>
      </section>
      <section style={{ marginTop: 24 }}>
        <h2>Recent Logs</h2>
        <pre>{JSON.stringify(logs, null, 2)}</pre>
      </section>
    </main>
  );
}
