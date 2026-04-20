# Gemini Balance 新手使用教學手冊（Desktop + Rust Runtime）

> 適用版本：`v1 only`（`/api/v1/*`、`/v1/*` 與 `/v1beta`）  
> `v2` 路由已完全停用，舊的呼叫會直接回覆 `410 Gone`。

## 1. 你會得到什麼
- **本機桌面管理介面（Tauri）**：可隨時登入、管理 API keys、設定 Pool 策略、並即時監控代理狀態。
- **高效能 API Gateway（Rust）**：建議主入口為 `/v1/chat/completions`（OpenAI 相容）。`/v1beta` 保留作為原生 Gemini 相容通道。
- **內建 Key Pool 輪詢機制**：支援 `round_robin`（依序）、`random`（隨機）、`least_fail`（最少失敗優先）。

## 2. 環境需求
開發與執行期皆在您的本機電腦上運作，強烈建議準備以下環境：
- 作業系統： macOS（Apple Silicon M1/M2/M3 優先）
- 編譯器： `rustup`（請確保為 stable toolchain, 1.77 左右及以上）
- 前端依賴： Node.js 20+ 與 npm
- 指令工具： `lsof`（啟動腳本會利用它來自動檢查 port 是否被佔用）

快速驗證環境：
```bash
rustup --version
node -v
npm -v
lsof -v
```

## 3. 第一次啟動（5 分鐘內搞定）
1. 環境設定 (自動或手動)：

**自動設定 (推薦)**：您可以直接執行：
```bash
./setup.sh
```
按照畫面提示貼上您的 Key，程式會自動幫您打包好存檔！

**手動設定**：
```bash
cp .env.example .env
```
並手動編輯 `.env` 中的必填值 (請見 `.env.example` 內針對 `API_KEYS` 的說明)。

2. 啟動方式選擇：
您可以選擇桌面版或是無頭核心模式：

- **桌面版含管理介面 (GUI)**
```bash
./start-desktop.sh
```

- **無頭模式 (Headless Server-only)**
如果是在伺服器環境或不需要介面，可以直接啟動極速核心：
```bash
./start-headless.sh
```

3. 登入管理介面 (若使用桌面版)：
- Tauri 視窗開啟後將顯示登入頁，請輸入 `.env` 中的 `AUTH_TOKEN`。
- 成功登入後，您便能自由流覽 `/keys`、`/config` 等管理頁面。

## 4. 登入後先確認這三件事
1. **Keys 頁面**：檢查您設定的 API Key 數量與 Pool 健康狀態。  
2. **Config 頁面**：檢查 `POOL_STRATEGY` 是否如期載入（建議平常保持 `round_robin`）。  
3. **終端機測試**：用 curl 測一次 `/v1/chat/completions`，確認整體路由與代理沒問題。  

## 5. 單一 URL 調用範例（重點功能）

一旦成功啟動，您在本機的 `18080` port（預設）就會開啟一個強大的代理點。

新整合建議預設：
- Base URL：`http://127.0.0.1:18080/v1`
- 驗證方式：`Authorization: Bearer <proxy_token>`
- `model` 建議使用 alias（例如 `sonnet`、`fast`）

### 5.0 一鍵快速驗證腳本
可直接使用腳本完成整體 smoke 測試（`/v1/models`、`/v1/chat/completions`、alias 路由、`/v1beta`）：
```bash
./scripts/quick-verify-api.sh
```

可選覆寫參數：
```bash
BASE_URL=http://127.0.0.1:18080 \
PROXY_TOKEN=sk-user-123456 \
OPENAI_MODEL=gemini-2.5-flash \
ALIAS_MODEL=claude-sonnet \
NATIVE_MODEL=gemini-2.5-flash \
./scripts/quick-verify-api.sh
```
若聊天/原生測試回傳 `429` 或 `503`，腳本會標記為 warning（上游配額或高流量），不代表本地 proxy 壞掉。

### 5.1 查詢支援的模型清單
```bash
curl -sS http://127.0.0.1:18080/v1/models \
  -H "Authorization: Bearer sk-user-123456"
```

### 5.2 一般聊天補全 (OpenAI 格式相容)
```bash
curl -sS http://127.0.0.1:18080/v1/chat/completions \
  -H "Authorization: Bearer sk-user-123456" \
  -H "Content-Type: application/json" \
  -d '{
    "model":"sonnet",
    "messages":[{"role":"user","content":"請回覆：hello"}]
  }'
```

### 5.3 指定模型別名（Model Pool）
如果在您的 `.env` 已經有設定了模型池映射 (例如：`MODEL_POOLS={"claude-sonnet":["gemini-2.5-pro"]}`)，您可以在 `model` 欄位直接呼叫：
```bash
curl -sS http://127.0.0.1:18080/v1/chat/completions \
  -H "Authorization: Bearer sk-user-123456" \
  -H "Content-Type: application/json" \
  -d '{
    "model":"claude-sonnet",
    "messages":[{"role":"user","content":"Hello!"}]
  }'
```
這時 Proxy 將會聰明地往後端輪替調用真實的 `gemini-2.5-pro` 模型。

### 5.4 原生 Gemini（`/v1beta`）相容通道
```bash
curl -sS http://127.0.0.1:18080/v1beta/models/gemini-2.5-flash:generateContent \
  -H "X-goog-api-key: sk-user-123456" \
  -H "Content-Type: application/json" \
  -d '{
    "contents": [{"parts": [{"text": "Explain AI"}]}]
  }'
```
相容說明：舊客戶端仍可使用 `x-api-key`、`x-goog-api-key`、`?key=`。

## 6. 常見問題 (FAQ)

### Q1: 登入一直失敗，被回傳 401？
- 可能是輸入的字串與 `.env` 中的 `AUTH_TOKEN` 不一致。
- 可能是您修改了 `.env` 但沒有重新啟動 `./start-desktop.sh`。

### Q2: 啟動時提示 Port 被佔用？
- `start-desktop.sh` 會先檢查並嘗試釋放衝突的子程序。若強制釋放失敗，請您手動關閉佔用到 `18080` - `18099` 之間或 `1420` 埠的程序後重試。

### Q3: 呼叫時出現 `503 Service Unavailable`？
- 通常代表目前沒有「健康的 Key」可用（可能全數進入了封鎖冷卻期或設定的 key 無效）。
- 到介面上的 `/keys` 檢查連線狀態，必要時手動 Reset 或是更新有效的 API Key。

## 7. 建議的日常操作流程
1. 查看 `/keys` 監控 Pool 的健康度。  
2. 若遇到限制，可到 `/config` 頁面調整策略方案。  
3. 客戶端配置優先使用 `http://127.0.0.1:18080/v1` 並搭配 `Authorization: Bearer`。  
4. 僅在客戶端必須使用 Gemini 原生端點時，再改用 `/v1beta`。  

## 8. Hermes / OpenClaw 快速導引
若您是用本機 Agent 工具串接，請直接參考：
- [Hermes Agent 與 OpenClaw 接入指南](hermes-openclaw-setup-zh.md)
