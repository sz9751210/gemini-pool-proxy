import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { AppShell } from "./AppShell";
import { NotificationProvider } from "./NotificationProvider";

describe("AppShell", () => {
  it("renders desktop nav items", () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify({ success: true }), { status: 200 }))
    );
    render(
      <NotificationProvider>
        <MemoryRouter initialEntries={["/keys"]}>
          <Routes>
            <Route element={<AppShell />}>
              <Route path="/keys" element={<div>keys page</div>} />
            </Route>
          </Routes>
        </MemoryRouter>
      </NotificationProvider>
    );

    expect(screen.getByRole("link", { name: /金鑰管理/i })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /配置中心/i })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /錯誤日誌/i })).toBeInTheDocument();
  });
});
