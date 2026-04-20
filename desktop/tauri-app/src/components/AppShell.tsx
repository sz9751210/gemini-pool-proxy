import { useEffect } from "react";
import { Link, Outlet, useLocation, useNavigate } from "react-router-dom";
import { apiClient, clearAuthToken } from "../lib/apiClient";
import { DesktopControls } from "./DesktopControls";
import { useNotifier } from "./NotificationProvider";

const navItems = [
  { to: "/keys", label: "金鑰管理", icon: "fa-key" },
  { to: "/config", label: "配置中心", icon: "fa-sliders" },
  { to: "/logs", label: "錯誤日誌", icon: "fa-triangle-exclamation" },
  { to: "/tester", label: "API 測試", icon: "fa-vial" }
];

export function AppShell() {
  const location = useLocation();
  const navigate = useNavigate();
  const notifier = useNotifier();

  useEffect(() => {
    function onSessionExpired() {
      clearAuthToken();
      notifier.push("error", "登入已失效，請重新登入");
      navigate("/", { replace: true });
    }
    window.addEventListener("session-expired", onSessionExpired);
    return () => {
      window.removeEventListener("session-expired", onSessionExpired);
    };
  }, [navigate, notifier]);

  async function logout() {
    try {
      await apiClient.logout();
      notifier.push("success", "已登出");
      navigate("/", { replace: true });
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "登出失敗");
    }
  }

  const title = navItems.find((item) => location.pathname.startsWith(item.to))?.label ?? "Gemini Balance Desktop";

  return (
    <div className="app-layout">
      <aside className="sidebar">
        <div className="brand-block">
          <div className="brand-logo">
            <i className="fas fa-shield-halved" />
          </div>
          <div>
            <h1>Gemini Balance</h1>
            <p>Desktop Control Center</p>
          </div>
        </div>

        <nav className="sidebar-nav">
          {navItems.map((item) => {
            const active = location.pathname.startsWith(item.to);
            return (
              <Link key={item.to} to={item.to} className={active ? "active" : ""}>
                <i className={`fas ${item.icon}`} />
                <span>{item.label}</span>
              </Link>
            );
          })}
        </nav>

        <footer className="sidebar-footer">
          <p>Gemini Balance Desktop</p>
          <small>Legacy style replicated for desktop workflow</small>
        </footer>
      </aside>

      <main className="main-pane">
        <header className="topbar">
          <div className="topbar-title">
            <h2>{title}</h2>
          </div>
          <div className="topbar-tools">
            <DesktopControls />
            <button type="button" className="danger" onClick={() => void logout()}>
              <i className="fas fa-right-from-bracket" />
              <span>登出</span>
            </button>
          </div>
        </header>

        <section className="page-content">
          <Outlet />
        </section>

        <footer className="main-footer">
          <span>Gemini Balance Desktop</span>
          <span>Version 0.1.0</span>
        </footer>
      </main>
    </div>
  );
}
