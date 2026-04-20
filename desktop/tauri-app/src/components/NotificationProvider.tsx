import { createContext, useContext, useMemo, useState, type ReactNode } from "react";

type Notice = {
  id: number;
  type: "info" | "success" | "error" | "warning";
  message: string;
};

type NotificationContextValue = {
  push: (type: Notice["type"], message: string) => void;
};

const NotificationContext = createContext<NotificationContextValue | null>(null);

export function NotificationProvider({ children }: { children: ReactNode }) {
  const [notices, setNotices] = useState<Notice[]>([]);

  const value = useMemo<NotificationContextValue>(
    () => ({
      push(type, message) {
        const id = Date.now() + Math.floor(Math.random() * 1000);
        setNotices((prev) => [...prev, { id, type, message }]);
        window.setTimeout(() => {
          setNotices((prev) => prev.filter((item) => item.id !== id));
        }, 4200);
      }
    }),
    []
  );

  return (
    <NotificationContext.Provider value={value}>
      {children}
      <div className="notice-stack">
        {notices.map((notice) => (
          <button
            type="button"
            className={`notice notice-${notice.type}`}
            key={notice.id}
            onClick={() => setNotices((prev) => prev.filter((item) => item.id !== notice.id))}
          >
            {notice.message}
          </button>
        ))}
      </div>
    </NotificationContext.Provider>
  );
}

export function useNotifier() {
  const context = useContext(NotificationContext);
  if (!context) {
    throw new Error("useNotifier must be used inside NotificationProvider");
  }
  return context;
}
