import { Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "./components/AppShell";
import { NotificationProvider } from "./components/NotificationProvider";
import { RequireSession } from "./components/RequireSession";
import { ConfigPage } from "./pages/ConfigPage";
import { KeysPage } from "./pages/KeysPage";
import { LoginPage } from "./pages/LoginPage";
import { LogsPage } from "./pages/LogsPage";
import { NotFoundPage } from "./pages/NotFoundPage";
import { ApiTesterPage } from "./pages/ApiTesterPage";

export function App() {
  return (
    <NotificationProvider>
      <Routes>
        <Route path="/" element={<LoginPage />} />

        <Route
          element={
            <RequireSession>
              <AppShell />
            </RequireSession>
          }
        >
          <Route path="/dashboard" element={<Navigate to="/keys" replace />} />
          <Route path="/keys" element={<KeysPage />} />
          <Route path="/config" element={<ConfigPage />} />
          <Route path="/logs" element={<LogsPage />} />
          <Route path="/tester" element={<ApiTesterPage />} />
        </Route>

        <Route path="/pro/ui" element={<Navigate to="/keys" replace />} />
        <Route path="*" element={<NotFoundPage />} />
      </Routes>
    </NotificationProvider>
  );
}
