import { FormEvent, useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { apiClient, getAuthToken, setAuthToken } from "../lib/apiClient";
import { useNotifier } from "../components/NotificationProvider";

export function LoginPage() {
  const navigate = useNavigate();
  const notifier = useNotifier();
  const [token, setToken] = useState(getAuthToken());
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    apiClient
      .getAuthTokenHint()
      .then((hint) => {
        if (hint && !token.trim()) {
          setToken(hint);
        }
      })
      .catch(() => undefined);
    // run once on initial login page render
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    apiClient
      .getSessionStatus()
      .then((status) => {
        if (status.authenticated) {
          navigate("/keys", { replace: true });
        }
      })
      .catch(() => undefined);
  }, [navigate]);

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    if (!token.trim()) {
      notifier.push("warning", "請輸入 AUTH_TOKEN");
      return;
    }
    setLoading(true);
    try {
      await apiClient.login(token.trim());
      setAuthToken(token.trim());
      notifier.push("success", "登入成功");
      navigate("/keys", { replace: true });
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "登入失敗");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="login-page">
      <div className="login-card">
        <div className="login-icon">
          <i className="fas fa-shield-halved" />
        </div>
        <h1>Gemini Balance Desktop</h1>
        <p>請輸入管理 Token 以登入控制台</p>

        <form onSubmit={onSubmit}>
          <label htmlFor="auth-token">AUTH_TOKEN</label>
          <div className="login-input-wrap">
            <i className="fas fa-key" />
            <input
              id="auth-token"
              type="password"
              value={token}
              onChange={(event) => setToken(event.target.value)}
              placeholder="sk-xxxx"
              autoFocus
            />
          </div>
          <button type="submit" className="primary wide" disabled={loading}>
            {loading ? "登入中..." : "登入"}
          </button>
        </form>
      </div>
    </div>
  );
}
