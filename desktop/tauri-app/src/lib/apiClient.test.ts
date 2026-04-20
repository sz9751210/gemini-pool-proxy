import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { apiClient, clearAuthToken, setAuthToken } from "./apiClient";

describe("apiClient", () => {
  beforeEach(() => {
    clearAuthToken();
    vi.restoreAllMocks();
  });

  afterEach(() => {
    clearAuthToken();
  });

  it("adds bearer token into request header", async () => {
    const fetchMock = vi.fn(async () => new Response(JSON.stringify({ authenticated: true }), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    setAuthToken("sk-test-token");
    await apiClient.getSessionStatus();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const init = fetchMock.mock.calls[0][1] as RequestInit;
    const headers = init.headers as Headers;
    expect(headers.get("Authorization")).toBe("Bearer sk-test-token");
  });

  it("throws and emits session-expired on unauthorized response", async () => {
    const fetchMock = vi.fn(async () => new Response(JSON.stringify({ detail: "unauthorized" }), { status: 401 }));
    vi.stubGlobal("fetch", fetchMock);
    const listener = vi.fn();
    window.addEventListener("session-expired", listener);
    await expect(apiClient.getSessionStatus()).rejects.toThrow("登入已失效");
    expect(listener).toHaveBeenCalledTimes(1);
    window.removeEventListener("session-expired", listener);
  });

  it("builds log query with filters", async () => {
    const fetchMock = vi.fn(async () => new Response(JSON.stringify({ logs: [], total: 0, limit: 20, offset: 0 }), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    await apiClient.getLogs({
      limit: 20,
      offset: 40,
      keySearch: "abc",
      errorSearch: "429",
      sortBy: "id",
      sortOrder: "asc"
    });
    const url = String(fetchMock.mock.calls[0][0]);
    expect(url).toContain("limit=20");
    expect(url).toContain("offset=40");
    expect(url).toContain("key_search=abc");
    expect(url).toContain("error_search=429");
    expect(url).toContain("sort_order=asc");
  });
});
