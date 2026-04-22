# Go + Wails 重寫遷移設計（不動原始 Rust/Tauri 代碼）

日期：2026-04-22  
狀態：Design Approved（待實作規劃）

## 1. 背景與目標

現有系統由 `core-rs`（Rust proxy/admin API）與 `desktop/tauri-app`（Tauri GUI）構成，已支援 headless 與桌面模式。  
本設計目標是在**不修改原始代碼**前提下，於新資料夾完成 Go 重寫，使用 Wails 作為新 GUI，並保留伺服器可部署的無頭模式。

目標重點：
- 以 Go 重寫核心服務，Wails 作為桌面操作介面。
- 強調輕量化部署，headless 模式不可依賴 GUI runtime。
- 優先確保對外 API 與 `.env` 相容（GUI 可重設計）。
- 以分階段方式完成功能完全轉移，降低一次性重寫風險。

## 2. 已確認範圍

使用者已確認以下決策：
- 相容層級採 `B`：對外 API 與 `.env` 相容優先，GUI 可重新設計。
- 功能分兩階段交付：
  - 第一階段：`headless proxy + 管理 API + 基本 Wails GUI`
  - 第二階段：補齊 `key pool / model pool / logs / dashboard`
- 第一階段管理能力至少包含：
  - `登入`
  - `設定編輯`
  - `服務啟停`
  - `健康狀態`
  - `keys 管理`
- 執行型態採架構 `A`：單一 Go 核心，兩個入口（`cmd/server` 與 `cmd/wails`）。

## 3. 方案比較與採納方案

### 方案 A（採納）
單一 Go 核心，兩個入口：
- `cmd/server`：headless binary，面向伺服器部署
- `cmd/wails`：GUI binary，面向桌面管理

優點：
- 部署語意清晰，headless 不受 Wails runtime 牽制
- 核心邏輯只維護一份，降低行為漂移
- 第二階段增量功能可在共享核心上直接擴展

### 方案 B（未採納）
單一 Wails binary，使用 `--headless/--gui` 切換模式。

不採納理由：
- 伺服器部署仍綁 GUI 依賴，違背輕量化目標
- 啟動語意與運維邊界不夠清楚

### 方案 C（未採納）
GUI 與 server 完全拆成兩個獨立專案。

不採納理由：
- 版本同步與整合測試成本高
- 設定與契約演進容易分岔

## 4. 目錄與架構設計

新實作放置於新目錄（建議：`go-wails/`），與現有 `core-rs/`、`desktop/tauri-app/` 並存：

```text
go-wails/
  cmd/
    server/      # headless 入口
    wails/       # GUI 入口
  internal/
    admin/       # 管理 API handlers/use-cases
    auth/        # admin session 與 proxy token 驗證
    config/      # .env 解析、驗證、寫回
    proxy/       # /v1、/v1beta 轉發與適配
    runtime/     # 狀態管理、啟停、健康狀態
    store/       # 持久化與狀態封裝（Phase 1 先檔案/記憶體）
    keypool/     # Phase 2
    modelpool/   # Phase 2
  ui/
    wails/       # 新 GUI 前端
```

關鍵邊界：
- 代理流量與管理流量邏輯分離。
- 兩類流量共用同一個 runtime state 與 config service。
- GUI 只透過 service/binding，不直接操作底層狀態結構。

## 5. API 相容策略

## 5.1 第一階段必保留主路徑
- `GET /v1/models`
- `POST /v1/chat/completions`
- `GET|POST /v1beta/*`
- `POST /api/v1/session/login`
- `POST /api/v1/session/logout`
- `GET /api/v1/session/status`
- `GET|PUT /api/v1/config`
- `GET /api/v1/keys`
- `POST /api/v1/keys/actions`
- `GET /api/v1/health`（或等效健康狀態路徑，供 GUI/headless 檢查）

## 5.2 認證/相容輸入
- 主要方式：`Authorization: Bearer <proxy_token>`
- 相容保留：`x-api-key`、`x-goog-api-key`、`?key=`

## 5.3 Legacy 路徑策略
- 第一階段不追求完整複製所有歷史兼容路徑。
- 以主契約可運作為優先；其他 legacy/compat 路徑列入第二階段評估清單。

## 6. `.env` 相容策略

保留既有核心鍵名與語意：
- `AUTH_TOKEN`
- `ALLOWED_TOKENS`
- `API_KEYS`
- `RUNTIME_BIND_HOST`
- `RUNTIME_PORT_START`
- `RUNTIME_PORT_END`
- `POOL_STRATEGY`
- `MODEL_POOLS`
- 以及第二階段會使用的 model 相關欄位（如 `MODEL_POOL_STRATEGY`）

解析規則：
- 同時接受 JSON array/object 與既有寬鬆格式。
- 寫回設定時統一輸出為穩定格式。
- 未識別欄位需保留，避免覆寫使用者手工配置。

## 7. 分階段設計

## 7.1 第一階段（可運行、可部署、可管理）

交付內容：
- headless server 可直接部署運行
- `/v1` 與 `/v1beta` 核心代理能力
- 管理 API：登入、設定編輯、服務啟停、健康狀態、keys 管理
- 基本 Wails GUI：至少覆蓋上述管理能力

不含內容：
- 完整 key pool 策略行為
- 完整 model pool 策略行為
- logs 與 dashboard 統計分析面板
- `proxy check / scheduler / cache stats` 全量管理功能

## 7.2 第二階段（策略與觀測性補齊）

交付內容：
- `key pool`：rotation、failure/cooldown、selection events
- `model pool`：alias 映射與策略（包含 `per_key_cycle` 等）
- logs：紀錄、查詢、細節
- dashboard：聚合統計與健康視圖

## 8. 資料流設計

### 8.1 Headless 啟動流
`cmd/server` 啟動 -> 載入 `.env` -> 建立 runtime/config service -> 掛載 proxy/admin 路由 -> 服務監控與健康輸出

### 8.2 GUI 啟動流
`cmd/wails` 啟動 -> 初始化同一套 runtime/config service -> frontend 透過 bindings 呼叫管理 use-cases -> 更新狀態與回饋

### 8.3 代理請求流（Phase 1）
Client -> token 驗證 -> 模型/設定檢查 -> upstream Gemini 轉發 -> 標準化回應或錯誤映射

### 8.4 管理請求流（Phase 1）
GUI/admin client -> session 驗證 -> config validation -> `.env` 寫回 -> runtime 更新/受控重載 -> 回傳最新狀態

### 8.5 策略擴展流（Phase 2）
Proxy handler 在進入 upstream 前：
1. modelpool 決定實際模型
2. keypool 選擇可用 key
3. 請求結果回寫 success/failure、cooldown、selection events

## 9. 錯誤處理與狀態一致性

## 9.1 錯誤分類
- `config errors`：配置缺失/格式錯誤/寫回失敗
- `runtime errors`：啟停、port 綁定、內部狀態轉換錯誤
- `upstream errors`：Gemini 上游逾時、限流、5xx、回應格式異常

要求：
- 對外不暴露內部錯誤堆疊。
- 對管理端提供可診斷訊息（錯誤碼 + 簡潔描述）。

## 9.2 狀態同步原則
- `.env` 為第一階段權威持久化來源。
- runtime 為執行鏡像，更新流程固定為：`驗證 -> 寫檔 -> 更新記憶體狀態`。
- 需重建 listener/client 的設定變更採「受控重載」，避免半更新狀態。

## 10. 測試與驗證策略

## 10.1 第一階段測試層次
- Unit tests：
  - `.env` parser（JSON 與寬鬆格式）
  - auth/token 解析（Bearer/header/query）
  - config validation
  - upstream 錯誤映射
- Integration tests：
  - `/v1/models`
  - `/v1/chat/completions`
  - `/v1beta/*`
  - `/api/v1/session/*`
  - `/api/v1/config`
  - `/api/v1/keys`、`/api/v1/keys/actions`
- Smoke tests：
  - headless 啟動與 port 綁定
  - 基本代理請求可通
  - GUI 可完成登入、設定變更、keys 管理、啟停與健康檢查

## 10.2 轉移驗證
- 既有 `scripts/quick-verify-api.sh` 可演進為「舊 Rust 與新 Go」雙目標對照驗證腳本。
- 判定標準：相同輸入下，關鍵路徑回應碼與核心回應結構一致。

## 11. 風險與對策

- 風險：第一階段 API 相容不足導致既有客戶端失效  
  對策：先鎖定主路徑契約，建立 integration baseline 測試。

- 風險：設定熱更新造成 runtime 與檔案不一致  
  對策：實施單一路徑更新流程與受控重載策略。

- 風險：第二階段策略功能直接耦合 handler，導致可維護性下降  
  對策：先建立 keypool/modelpool 獨立模組，再由 handler 綁接。

## 12. 驗收標準

第一階段驗收：
- 新目錄實作可獨立 build/run，不改動既有 Rust/Tauri 代碼。
- headless 模式可在伺服器環境以單一 Go binary 運行。
- Wails GUI 可完成第一階段管理能力操作。
- 對外主 API 與 `.env` 相容需求達標。

第二階段驗收：
- key pool 與 model pool 功能行為符合既有策略期待。
- logs 與 dashboard 能提供可用監控與追蹤資料。
- 回歸測試可證明「功能完全轉移」。

## 13. 非目標（本設計不做）

- 不在本次重寫中修改或刪除現有 `core-rs` / `desktop/tauri-app`。
- 不追求第一階段即覆蓋全部 legacy/compat API。
- 不在設計階段引入額外基礎設施（如 DB、分散式佈署）作為必要前提。
