# 環境變數與配置指南 (Configuration Guide)

為了使 `gemini-pool-proxy` 正常並具高效率的運作，需要依賴 `.env` 來調整啟動參數以及代理伺服器的行為。
本專區整理了在 `.env.example` 中所有的支援參數，幫助您根據伺服器量級與代理需求進行專業配置。

## 基礎安全與驗證設定

這些設定會直接影響您的代理伺服器如何認證授權使用者，以及如何去 Google 取資料。

新客戶端建議統一標準：
- Base URL：`http://127.0.0.1:18080/v1`
- 驗證方式：`Authorization: Bearer <proxy_token>`
- `x-api-key` / `x-goog-api-key` / `?key=` 僅保留相容用途。

| 參數 | 型別 | 預設 / 範例 | 說明與用途 |
| :--- | :--- | :--- | :--- |
| `AUTH_TOKEN` | String | `sk-admin-123456` | **管理員登入權杖**。用於登入您的 Tauri Desktop GUI 或存取受保護的 `/api/v1/*` 管理介面。 |
| `ALLOWED_TOKENS` | Array | `["sk-user-123456"]` | **使用者驗證金鑰**。標準傳法為 `Authorization: Bearer <token>`。相容輸入（`x-api-key`、`x-goog-api-key`、`?key=`、`?api_key=`）仍可使用。 |
| `API_KEYS` | Array | `["AIzaSy..."]` | **Google Gemini 金鑰**。您需要在此填入真正對 Google Gemini 官方發出的真實 API Keys。如果您提供多組，系統會自動做負載平衡 (Load Balance)。 |

## .env 格式建議（重要）

針對 JSON/陣列欄位，建議使用「外層單引號包 JSON」，可避免解析歧義：

```dotenv
ALLOWED_TOKENS='["sk-user-1","sk-user-2"]'
API_KEYS='["AIzaSy_xxx","AIzaSy_yyy"]'
MODEL_POOLS='{"sonnet":["gemma-4-26b-a4b-it"],"fast":["gemini-2.5-flash"]}'
```

## 模型池映射 (Model Pools)

因為某些工具強制綁定特定的模型字串（如 `claude-3-5-sonnet`），又或者您希望將繁亂的模型統一命題。透過此設定，您可以在代理層直接進行智能轉換。

| 參數 | 型別 | 預設 / 範例 | 說明與用途 |
| :--- | :--- | :--- | :--- |
| `MODEL_POOLS` | JSON | `{"fast":["gemini-2.5-flash"]}` | 定義模型別名。客戶端若請求 `fast` 模型，Proxy 會自動轉換為 `gemini-2.5-flash`，並在使用多個模型時自動輪詢。 |
| `THINKING_MODELS` | Array | `["gemini-2.5-flash"]` | 註冊支援「思考 (Thinking) 歷程」的模型。 |
| `SEARCH_MODELS` | Array | `["gemini-2.5-pro"]` | 註冊支援「Google Search 網頁搜尋」工具的模型。 |
| `IMAGE_MODELS` | Array | `["gemini-2.0-flash-exp"]` | 註冊專門用於圖像理解、生成或上傳的多模態模型。 |
| `URL_CONTEXT_MODELS`| Array | `["gemini-2.5-pro"]` | 註冊能直接解析客戶端傳來 URL 上下文的模型。 |
| `FILTERED_MODELS` | Array | `["chat-bison-001"]` | **黑名單過濾**。客戶端一旦請求陣列中的模型，Proxy 會立刻阻止該請求。 |

## 連線行為與池策略

決定您的 Proxy 遭遇 Rate Limit 或是網路斷線時要作出的行為。

| 參數 | 型別 | 預設 / 範例 | 說明與用途 |
| :--- | :--- | :--- | :--- |
| `POOL_STRATEGY` | Enum | `round_robin` | 輪詢策略，可選 `round_robin` (照順序)、`random` (隨機跳轉)、`least_fail` (最少失敗優先)。推薦使用預設。 |
| `MAX_FAILURES` | Int | `3` | 允許單一金鑰在短時間內連續連線失敗的最大次數。超過此數，代理伺服器將會把該金鑰打入冷卻名單 (Cooldown)。 |
| `COOLDOWN_SECONDS`| Int | `60` | 當有金鑰因為打到 Rate Limit (429) 或連續超時被送入冷卻名單，這項參數決定了幾秒後他能被從冷凍庫放出來再次使用。 |
| `COMPAT_MODE` | Bool | `true` | 是否開啟 OpenAI 全自動相容模式，將輸入的 OpenAI `messages` 掛載上原生 Gemini 格式 (建議開啟)。 |

## 本機伺服器與綁紮

確保您本機網路的安全防護設定。

| 參數 | 型別 | 預設 / 範例 | 說明與用途 |
| :--- | :--- | :--- | :--- |
| `RUNTIME_BIND_HOST`| String | `127.0.0.1` | **非常重要**：預設為 localhost。這能確保無意間在公用網路時，別人無法透過您的 IP 呼叫您的代理伺服器。 |
| `RUNTIME_PORT_START`| Int | `18080` | Proxy Server 嘗試啟用的起始連接埠。 |
| `RUNTIME_PORT_END` | Int | `18099` | Proxy Server 若發現 Port 被佔用，自動往上跳找尋新 Port 的最大限制。 |
| `SESSION_COOKIE_NAME`| String| `gb_session` | Tauri / 基礎瀏覽網頁介面登入時存放 JWT Cookie 的自訂名稱。 |
