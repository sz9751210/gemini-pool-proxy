import { useEffect, useState, type ReactNode } from "react";
import { Navigate } from "react-router-dom";
import { apiClient } from "../lib/apiClient";

export function RequireSession({ children }: { children: ReactNode }) {
  const [loading, setLoading] = useState(true);
  const [authenticated, setAuthenticated] = useState(false);

  useEffect(() => {
    let canceled = false;
    apiClient
      .getSessionStatus()
      .then((resp) => {
        if (!canceled) {
          setAuthenticated(Boolean(resp.authenticated));
        }
      })
      .catch(() => {
        if (!canceled) {
          setAuthenticated(false);
        }
      })
      .finally(() => {
        if (!canceled) {
          setLoading(false);
        }
      });
    return () => {
      canceled = true;
    };
  }, []);

  if (loading) {
    return (
      <div className="page-loading">
        <div className="spinner" />
        <p>正在驗證登入狀態...</p>
      </div>
    );
  }

  if (!authenticated) {
    return <Navigate to="/" replace />;
  }

  return <>{children}</>;
}
