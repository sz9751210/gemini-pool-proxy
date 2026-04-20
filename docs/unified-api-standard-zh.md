# 統一 API 標準（多 Provider 轉發）

本文件定義 `gemini-pool-proxy` 對外 API 的「單一標準」，目標是讓客戶端只對接一種協議，後端可彈性擴充到多個 LLM Provider（Google/OpenAI/Anthropic/...）而不需改客戶端程式。

## 1. 標準入口（Canonical API）

- `GET /v1/models`
- `POST /v1/chat/completions`

原則：
- 對外只推薦使用 `/v1/*`（OpenAI-compatible）。
- `/v1beta/*` 保留為相容通道，不作為新整合的主入口。

## 2. 驗證標準（Auth）

唯一標準：
- `Authorization: Bearer <proxy_token>`

相容（過渡用）：
- `x-api-key` / `x-goog-api-key`
- Query `?key=` / `?api_key=`

原則：
- 新客戶端一律使用 `Authorization: Bearer`。
- 相容格式僅保留給舊客戶端，後續可逐步淘汰。

## 3. 模型命名標準（Model Alias）

客戶端傳「邏輯模型名」，後端映射到真實 provider model：

- `sonnet`（例如映射到 Anthropic 或 Gemini 的對應模型池）
- `fast`
- `vision`
- `reasoning`

原則：
- 客戶端不直接綁定供應商原生模型名。
- 供應商切換、灰度升級、故障切換都在後端完成。

## 4. 請求與回應格式

### 請求（Request）

- 以 OpenAI Chat Completions payload 為準：
  - `model`
  - `messages`
  - `temperature` / `top_p` / `max_tokens`（或對應轉換）
  - `stream`

### 回應（Response）

- 以 OpenAI Chat Completions response 為準：
  - `id`
  - `object`
  - `created`
  - `model`
  - `choices`
  - `usage`

### 串流（Streaming）

- 採 SSE 與 OpenAI 慣例事件切片格式，結尾送出 `[DONE]`。

## 5. 錯誤模型（Error Contract）

統一回傳 OpenAI 風格錯誤：

```json
{
  "error": {
    "message": "human-readable message",
    "type": "invalid_request_error",
    "code": "invalid_api_key"
  }
}
```

映射原則：
- 上游 401/403 -> `invalid_api_key` 或 `authentication_error`
- 上游 429 -> `rate_limit_exceeded`
- 上游 5xx -> `upstream_service_error`

## 6. Provider Adapter 邊界

後端需拆分為三層：
- `ingress`：只處理標準 API（/v1）
- `router`：模型 alias -> provider + real model
- `adapter`：各 provider 的請求/回應轉換與錯誤映射

原則：
- 新增 provider 僅新增 adapter 與路由配置，不改 client contract。

## 7. 設定檔建議（.env）

針對陣列/JSON 欄位，建議使用「外層單引號包 JSON」避免解析歧義：

```dotenv
ALLOWED_TOKENS='["sk-user-1","sk-user-2"]'
API_KEYS='["AIzaSy_xxx","AIzaSy_yyy"]'
MODEL_POOLS='{"sonnet":["gemma-4-26b-a4b-it"],"fast":["gemini-2.5-flash"]}'
```

## 8. 客戶端接入建議

新專案優先：
- Base URL: `http://127.0.0.1:18080/v1`
- Auth: `Authorization: Bearer <proxy_token>`
- Model: 使用 alias（如 `sonnet` / `fast`）

僅在客戶端硬性要求 Gemini-native 時，才使用：
- `http://127.0.0.1:18080/v1beta`
- 建議上游驗證頭：`X-goog-api-key: <proxy_token>`

## 9. 遷移策略（建議）

1. 新接入全部走 `/v1/chat/completions`。
2. 舊客戶端保留 `/v1beta` 與 `x-api-key` / `x-goog-api-key` / `?key=` 相容。
3. 在管理後台標示相容流量比例（header auth vs query auth）。
4. 當 query auth 低於門檻後，公告淘汰時程。
