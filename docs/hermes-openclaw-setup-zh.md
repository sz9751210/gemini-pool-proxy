# Hermes Agent 與 OpenClaw 接入指南（統一 `/v1` 入口）

本文件提供一套可直接落地的接入流程，讓 **Hermes Agent** 與 **OpenClaw** 都走同一個 Proxy 入口：

- 統一入口：`http://127.0.0.1:18080/v1`
- 統一驗證：`Authorization: Bearer <proxy_token>`
- 模型建議：使用 alias（例如 `claude-sonnet`）

## 0. 先確認 Proxy 可用

啟動：

```bash
./start-headless.sh
```

快速驗證（建議）：

```bash
./scripts/quick-verify-api.sh
```

若腳本顯示 chat/native `429` 或 `503`，通常是上游配額或高流量，非本地 proxy 壞掉。

## 1. Hermes Agent 設定

### 1.1 推薦設定方式（走 gemini provider，但入口固定 `/v1`）

在 `~/.hermes/.env` 新增：

```dotenv
GEMINI_API_KEY=sk-user-123456
GEMINI_BASE_URL=http://127.0.0.1:18080/v1
```

設定 Hermes 預設模型與 provider：

```bash
hermes config set model.provider gemini
hermes config set model.base_url http://127.0.0.1:18080/v1
hermes config set model.default claude-sonnet
```

### 1.2 清理舊憑證（建議）

若過去曾用過 `gemini` 官方端點，可能會殘留 credential pool 設定。可先檢查：

```bash
hermes auth list
```

若看到舊的 `gemini` 憑證，先移除再重開 Hermes：

```bash
hermes auth remove gemini <index|id|label>
```

### 1.3 驗證 Hermes 是否接到本服務

```bash
GEMINI_API_KEY=sk-user-123456 \
GEMINI_BASE_URL=http://127.0.0.1:18080/v1 \
hermes chat -q "Reply with token HERMES_DOC_OK only" --provider gemini -m gemini-2.5-flash -Q
```

看到 `HERMES_DOC_OK` 即代表接通成功。

## 2. OpenClaw 設定

此處以常見的 `~/.picoclaw/config.json` 結構為例（OpenClaw/PicoClaw 相容配置）。

### 2.1 推薦：用 openai provider 對接統一 `/v1`

在 `providers.openai` 設定：

```json
{
  "providers": {
    "openai": {
      "api_key": "sk-user-123456",
      "api_base": "http://127.0.0.1:18080/v1"
    }
  },
  "agents": {
    "defaults": {
      "model": "claude-sonnet"
    }
  }
}
```

### 2.2 可選：用 `jq` 直接改檔

```bash
cp ~/.picoclaw/config.json ~/.picoclaw/config.json.bak

jq '
  .providers.openai.api_key = "sk-user-123456" |
  .providers.openai.api_base = "http://127.0.0.1:18080/v1" |
  .agents.defaults.model = "claude-sonnet"
' ~/.picoclaw/config.json > /tmp/picoclaw.config.json && mv /tmp/picoclaw.config.json ~/.picoclaw/config.json
```

修改後請重啟 OpenClaw。

## 3. 手動 API 驗證（可獨立於客戶端）

### 3.1 `/v1`（統一入口）

```bash
curl -sS http://127.0.0.1:18080/v1/chat/completions \
  -H "Authorization: Bearer sk-user-123456" \
  -H "Content-Type: application/json" \
  -d '{
    "model":"claude-sonnet",
    "messages":[{"role":"user","content":"Reply OK"}]
  }'
```

### 3.2 `/v1beta`（原生 Gemini 相容通道）

```bash
curl -sS http://127.0.0.1:18080/v1beta/models/gemini-2.5-flash:generateContent \
  -H "X-goog-api-key: sk-user-123456" \
  -H "Content-Type: application/json" \
  -d '{
    "contents":[{"parts":[{"text":"ping"}]}]
  }'
```

## 4. 常見問題

1. `Missing or invalid Authorization header`
- 請確認 base URL 是 `http://127.0.0.1:18080/v1`（不要少 `:18080`）。
- 請確認 token 在 `.env` 的 `ALLOWED_TOKENS` 內。

2. `HTTP 400 Upstream provider returned an error`
- 通常是 payload 或模型被上游拒絕，先用本文件的 curl 最小範例交叉驗證。

3. `429` / `503`
- 上游配額或高流量，建議稍後重試，或改用其他模型 alias。
