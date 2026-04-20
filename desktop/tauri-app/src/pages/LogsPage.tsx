import { useEffect, useMemo, useState } from "react";
import { Modal } from "../components/Modal";
import { useNotifier } from "../components/NotificationProvider";
import { apiClient } from "../lib/apiClient";
import type { LogDetailV2, LogRecordV2 } from "../lib/types";

type DeleteConfirmState = {
  open: boolean;
  mode: "single" | "bulk" | "all";
  ids: number[];
};

export function LogsPage() {
  const notifier = useNotifier();
  const [loading, setLoading] = useState(false);
  const [logs, setLogs] = useState<LogRecordV2[]>([]);
  const [total, setTotal] = useState(0);
  const [limit, setLimit] = useState(20);
  const [offset, setOffset] = useState(0);
  const [sortOrder, setSortOrder] = useState<"asc" | "desc">("desc");
  const [selected, setSelected] = useState<Record<number, boolean>>({});

  const [keySearch, setKeySearch] = useState("");
  const [errorSearch, setErrorSearch] = useState("");
  const [errorCodeSearch, setErrorCodeSearch] = useState("");
  const [startDate, setStartDate] = useState("");
  const [endDate, setEndDate] = useState("");
  const [pageInput, setPageInput] = useState("");

  const [detailModal, setDetailModal] = useState<{ open: boolean; loading: boolean; data: LogDetailV2 | null }>({
    open: false,
    loading: false,
    data: null
  });
  const [deleteConfirm, setDeleteConfirm] = useState<DeleteConfirmState>({
    open: false,
    mode: "single",
    ids: []
  });

  useEffect(() => {
    void loadLogs();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [offset, limit, sortOrder]);

  async function loadLogs() {
    setLoading(true);
    try {
      const resp = await apiClient.getLogs({
        limit,
        offset,
        keySearch: keySearch || undefined,
        errorSearch: errorSearch || undefined,
        errorCodeSearch: errorCodeSearch || undefined,
        startDate: startDate || undefined,
        endDate: endDate || undefined,
        sortBy: "id",
        sortOrder
      });
      setLogs(resp.logs);
      setTotal(resp.total);
      setSelected({});
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "載入日誌失敗");
    } finally {
      setLoading(false);
    }
  }

  const selectedIds = useMemo(
    () =>
      Object.entries(selected)
        .filter(([, checked]) => checked)
        .map(([id]) => Number(id)),
    [selected]
  );

  async function search() {
    setOffset(0);
    await loadLogs();
  }

  async function openDetail(id: number) {
    setDetailModal({ open: true, loading: true, data: null });
    try {
      const resp = await apiClient.getLogDetail(id);
      setDetailModal({ open: true, loading: false, data: resp.log });
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "載入詳情失敗");
      setDetailModal({ open: false, loading: false, data: null });
    }
  }

  async function copySelected() {
    if (selectedIds.length === 0) {
      notifier.push("warning", "請先勾選日誌");
      return;
    }
    const selectedLogs = logs.filter((item) => selectedIds.includes(item.id));
    const text = selectedLogs
      .map((item) =>
        JSON.stringify(
          {
            id: item.id,
            key: item.maskedKey,
            error: item.errorType,
            statusCode: item.statusCode,
            model: item.model,
            time: item.requestAt
          },
          null,
          0
        )
      )
      .join("\n");
    await navigator.clipboard.writeText(text);
    notifier.push("success", `已複製 ${selectedLogs.length} 筆日誌`);
  }

  async function executeDelete() {
    try {
      if (deleteConfirm.mode === "single" && deleteConfirm.ids[0]) {
        await apiClient.deleteLog(deleteConfirm.ids[0]);
      } else if (deleteConfirm.mode === "bulk") {
        await apiClient.deleteLogs(deleteConfirm.ids);
      } else if (deleteConfirm.mode === "all") {
        await apiClient.deleteAllLogs();
      }
      notifier.push("success", "刪除完成");
      setDeleteConfirm({ open: false, mode: "single", ids: [] });
      await loadLogs();
    } catch (error) {
      notifier.push("error", error instanceof Error ? error.message : "刪除失敗");
    }
  }

  const begin = total === 0 ? 0 : offset + 1;
  const end = Math.min(offset + limit, total);
  const currentPage = Math.floor(offset / limit) + 1;
  const totalPages = Math.max(1, Math.ceil(total / limit));

  return (
    <div className="panel-stack">
      <section className="panel">
        <div className="panel-head">
          <h3>錯誤日誌</h3>
          <div className="actions">
            <button type="button" onClick={() => void loadLogs()} disabled={loading}>
              重新整理
            </button>
            <button type="button" onClick={() => void copySelected()}>
              批次複製
            </button>
            <button
              type="button"
              onClick={() =>
                setDeleteConfirm({
                  open: true,
                  mode: "bulk",
                  ids: selectedIds
                })
              }
              disabled={selectedIds.length === 0}
            >
              批次刪除
            </button>
            <button
              type="button"
              className="danger"
              onClick={() => setDeleteConfirm({ open: true, mode: "all", ids: [] })}
            >
              清空全部
            </button>
            <button
              type="button"
              onClick={() =>
                setSelected(Object.fromEntries(logs.map((item) => [item.id, !selected[item.id]])))
              }
            >
              全選/全不選
            </button>
          </div>
        </div>

        <div className="toolbar wrap">
          <input value={keySearch} onChange={(event) => setKeySearch(event.target.value)} placeholder="key 搜尋" />
          <input
            value={errorSearch}
            onChange={(event) => setErrorSearch(event.target.value)}
            placeholder="錯誤訊息搜尋"
          />
          <input
            value={errorCodeSearch}
            onChange={(event) => setErrorCodeSearch(event.target.value)}
            placeholder="錯誤碼"
          />
          <input type="datetime-local" value={startDate} onChange={(event) => setStartDate(event.target.value)} />
          <input type="datetime-local" value={endDate} onChange={(event) => setEndDate(event.target.value)} />
          <select value={limit} onChange={(event) => setLimit(Number(event.target.value))}>
            <option value={10}>10 / 頁</option>
            <option value={20}>20 / 頁</option>
            <option value={50}>50 / 頁</option>
          </select>
          <button type="button" onClick={() => setSortOrder((prev) => (prev === "asc" ? "desc" : "asc"))}>
            ID {sortOrder === "asc" ? "↑" : "↓"}
          </button>
          <button type="button" className="primary" onClick={() => void search()}>
            搜尋
          </button>
        </div>

        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th />
                <th>ID</th>
                <th>Key</th>
                <th>錯誤</th>
                <th>錯誤碼</th>
                <th>模型</th>
                <th>時間</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {logs.map((log) => (
                <tr key={log.id}>
                  <td>
                    <input
                      type="checkbox"
                      checked={Boolean(selected[log.id])}
                      onChange={(event) =>
                        setSelected((prev) => ({
                          ...prev,
                          [log.id]: event.target.checked
                        }))
                      }
                    />
                  </td>
                  <td>{log.id}</td>
                  <td>{log.maskedKey}</td>
                  <td>{log.errorType}</td>
                  <td>{log.statusCode}</td>
                  <td>{log.model}</td>
                  <td>{new Date(log.requestAt).toLocaleString()}</td>
                  <td>
                    <div className="actions">
                      <button type="button" onClick={() => void openDetail(log.id)}>
                        詳情
                      </button>
                      <button
                        type="button"
                        className="danger"
                        onClick={() => setDeleteConfirm({ open: true, mode: "single", ids: [log.id] })}
                      >
                        刪除
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
              {!loading && logs.length === 0 && (
                <tr>
                  <td colSpan={8} className="empty">
                    沒有資料
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
        {loading && <p className="muted-text">載入中...</p>}

        <div className="pager">
          <button type="button" disabled={offset <= 0} onClick={() => setOffset((prev) => Math.max(0, prev - limit))}>
            上一頁
          </button>
          <span>
            顯示 {begin} - {end} / {total}
          </span>
          <button
            type="button"
            disabled={offset + limit >= total}
            onClick={() => setOffset((prev) => prev + limit)}
          >
            下一頁
          </button>
          <span>跳頁</span>
          <input
            className="page-jump"
            value={pageInput}
            onChange={(event) => setPageInput(event.target.value)}
            placeholder={`1-${totalPages}`}
          />
          <button
            type="button"
            onClick={() => {
              const target = Number(pageInput);
              if (Number.isNaN(target) || target < 1 || target > totalPages) {
                notifier.push("warning", "頁碼超出範圍");
                return;
              }
              setOffset((target - 1) * limit);
              setPageInput("");
            }}
          >
            前往
          </button>
          <span>
            第 {currentPage} / {totalPages} 頁
          </span>
        </div>
      </section>

      <Modal open={detailModal.open} title="日誌詳情" onClose={() => setDetailModal({ open: false, loading: false, data: null })} wide>
        {detailModal.loading && <p>載入中...</p>}
        {!detailModal.loading && detailModal.data && (
          <div className="log-detail">
            <dl>
              <dt>ID</dt>
              <dd>{detailModal.data.id}</dd>
              <dt>Key</dt>
              <dd>{detailModal.data.maskedKey}</dd>
              <dt>錯誤</dt>
              <dd>{detailModal.data.errorType}</dd>
              <dt>狀態碼</dt>
              <dd>{detailModal.data.statusCode}</dd>
              <dt>模型</dt>
              <dd>{detailModal.data.model}</dd>
              <dt>時間</dt>
              <dd>{new Date(detailModal.data.requestAt).toLocaleString()}</dd>
              <dt>Detail</dt>
              <dd>{detailModal.data.detail}</dd>
            </dl>
            <h4>Request</h4>
            <pre>{detailModal.data.requestBody}</pre>
            <h4>Response</h4>
            <pre>{detailModal.data.responseBody}</pre>
          </div>
        )}
      </Modal>

      <Modal
        open={deleteConfirm.open}
        title="刪除確認"
        onClose={() => setDeleteConfirm({ open: false, mode: "single", ids: [] })}
        footer={
          <div className="actions">
            <button type="button" onClick={() => setDeleteConfirm({ open: false, mode: "single", ids: [] })}>
              取消
            </button>
            <button type="button" className="danger" onClick={() => void executeDelete()}>
              確認刪除
            </button>
          </div>
        }
      >
        {deleteConfirm.mode === "single" && <p>確定刪除日誌 #{deleteConfirm.ids[0]}？此操作不可復原。</p>}
        {deleteConfirm.mode === "bulk" && <p>確定批次刪除 {deleteConfirm.ids.length} 筆日誌？此操作不可復原。</p>}
        {deleteConfirm.mode === "all" && <p>確定清空全部日誌？此操作不可復原。</p>}
      </Modal>
    </div>
  );
}
