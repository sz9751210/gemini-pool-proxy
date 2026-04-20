<p align="center">
  <a href="https://github.com/alan/gemini-pool-proxy" target="_blank">
    <img src="https://img.shields.io/badge/Refactored%20by-Alan-orange" alt="Refactored by Alan"/>
  </a>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.77%2B-brown.svg" alt="Rust"></a>
  <a href="https://tauri.app/"><img src="https://img.shields.io/badge/Tauri-2.0-blue.svg" alt="Tauri"></a>
</p>

# Gemini Pool Proxy

Gemini Pool Proxy 是一個基於 Rust 與 Tauri 構建的高效能 API 代理與 API Key 管理工具，專注於 **桌面端管理 + 本地代理**，為開發工具（如 Claude Code）提供順暢的整合體驗。

> ⚠️ **重要聲明**: 本專案採用 [CC BY-NC 4.0](LICENSE) 協議，**嚴禁任何形式的商業倒賣服務**。

## ✨ 核心特色

- **統一 API 標準** (`/v1/chat/completions`)：以單一 OpenAI 相容協議對接所有客戶端。
- **Gemini Native 相容通道** (`/v1beta`)：保留給舊版/原生客戶端，不建議作為新專案主入口。
- **模型池 (Model Pool) Alias**：支援自定義模型別名並自動輪替後端模型清單。
- **自動化金鑰輪替與管理**：內建多種規則 (Round Robin / Random / Least Fail) 管理多個 API Key。
- **高效能 Rust 核心**：提供透明、極低延遲的串流反向代理。
- **視覺化桌面管理 (Tauri)**：擁有直觀的 GUI 後台，配置修改即時生效。

## 📍 專案結構

- `core-rs/`: 核心 Rust Workspace (包含 `gateway-server` 與狀態模型 `gateway-core`)。
- `desktop/tauri-app/`: 桌面管理應用的前端 (React + Vite + TypeScript)。
- `gemini-balance-next/`: (開發中) Go 語言重寫版本。

## 🚀 快速開始

### 1. 環境要求
請確保安裝的 Rust 與 Cargo 版本為 **1.77 或以上**。若遇 `lock file version 4 was found` 錯誤，請執行：
```bash
rustup update
```

### 2. 環境配置 (自動或手動)

**選擇一：互動式自動配置 (推薦新手或 Headless 用戶)**
```bash
./setup.sh
```
此腳本會友善地詢問並引導您填寫 Google API 密鑰，並自動幫您生成符合規則的 `.env`。

**選擇二：手動配置**
```bash
cp .env.example .env
```
編輯 `.env` 並填入對應的 `AUTH_TOKEN`, `ALLOWED_TOKENS`, 與 `API_KEYS` 陣列等。

### 3. 啟動方式選擇

**啟動桌面版 (GUI)**：
```bash
./start-desktop.sh
```
啟動成功後，即可使用 `AUTH_TOKEN` 的值登入管理介面。

**啟動無頭模式 (Headless / Server-only)**：
您也可以略過前端介面，只啟動輕量且極速的 Rust 核心轉發代理。
```bash
./start-headless.sh
```

## ⚙️ 常用 API 呼叫範例

新整合建議預設：
- Base URL: `http://127.0.0.1:18080/v1`
- 驗證標頭：`Authorization: Bearer <proxy_token>`
- 模型欄位：使用 alias（例如 `sonnet`、`fast`）

### 快速驗證（Smoke Test）
可直接用一支腳本驗證 `/v1`、模型 alias、與 `/v1beta` 相容通道：
```bash
./scripts/quick-verify-api.sh
```

可選參數（覆寫預設）：
```bash
BASE_URL=http://127.0.0.1:18080 \
PROXY_TOKEN=sk-user-123456 \
OPENAI_MODEL=gemini-2.5-flash \
ALIAS_MODEL=claude-sonnet \
NATIVE_MODEL=gemini-2.5-flash \
./scripts/quick-verify-api.sh
```
若聊天/原生測試出現 `429` 或 `503`，腳本會標記為 warning（代表上游配額或高流量），不視為本地 proxy 失敗。

### 驗證 `per_key_cycle` 輪詢
當 `MODEL_POOL_STRATEGY=per_key_cycle` 時，可用專用腳本確認：
- API key 是否每次都切到下一順位（round-robin）
- model 是否在「完整一輪 key」後才切換到下一個

```bash
./scripts/verify-per-key-cycle.sh
```

可選參數（覆寫預設）：
```bash
BASE_URL=http://127.0.0.1:18080 \
MODEL_ALIAS=claude-sonnet \
REQUEST_COUNT=8 \
./scripts/verify-per-key-cycle.sh
```

若你在 `rtk` 環境執行、且需要回圈位址存取，可加上：
```bash
USE_RTK_PROXY_CURL=1 ./scripts/verify-per-key-cycle.sh
```

一鍵啟動 headless 並驗證：
```bash
./scripts/headless-verify-per-key-cycle.sh
```

### 取得模型清單
```bash
curl -sS http://127.0.0.1:18080/v1/models \
  -H "Authorization: Bearer sk-123456"
```

### 一般對話 (OpenAI 相容模式)
```bash
curl -sS http://127.0.0.1:18080/v1/chat/completions \
  -H "Authorization: Bearer sk-123456" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "sonnet",
    "messages": [{"role": "user", "content": "hello"}]
  }'
```

### 原生 Gemini 模式（相容通道）
```bash
curl -sS http://127.0.0.1:18080/v1beta/models/gemini-2.5-flash:generateContent \
  -H "X-goog-api-key: sk-123456" \
  -H "Content-Type: application/json" \
  -d '{
    "contents": [{"parts": [{"text": "Explain AI"}]}]
  }'
```
相容說明：舊客戶端仍可使用 `x-api-key`、`x-goog-api-key`、`?key=`。

### 模型池配置 (Model Pool)
在 `.env` 中加入以下配置：
```env
MODEL_POOLS={"claude-sonnet":["gemini-2.5-pro"],"fast":["gemini-2.5-flash","gemma-3-4b-it"]}
```
配置後即可在請求中將 model 指定為 `claude-sonnet` 或 `fast`，系統會自動轉換並輪替對應的模型。

## 🤝 致謝

本專案由 **Alan** 重構與維護。
特別感謝原上游開源專案：[snailyp/gemini-balance](https://github.com/snailyp/gemini-balance)。

## 📚 文件地圖 (Map of Content)

為保持根目錄簡潔，我們將詳細的說明手冊與設定詳解歸納至 `docs/` 目錄中。請參閱以下文件：

- ⚙️ **[環境變數與參數配置指南 (Configuration)](docs/configuration-zh.md)**：包含 `.env` 內如模型池、路由黑名單等所有進階參數的詳細設定說明。
- 🧭 **[統一 API 標準（多 Provider 轉發）](docs/unified-api-standard-zh.md)**：對外 API / 驗證 / 模型 alias 的統一規範。
- 📖 **[新手快速上手教學 (Beginner Guide)](docs/beginner-guide-zh.md)**：五分鐘內啟動您的代理服務手冊。
- 🤖 **[Hermes Agent / OpenClaw 接入指南](docs/hermes-openclaw-setup-zh.md)**：示範如何以統一 `/v1` 入口完成代理設定與驗證。
- 🏗️ **[開發與系統架構說明 (Architecture)](docs/architecture-zh.md)**：開發者必讀，暸解 Tauri、Rust Gateway 與 Core 之間的通訊設計與架構。

## 📜 授權條款

本專案採用 [CC BY-NC 4.0](LICENSE) 條款授權。商業使用或二次銷售被嚴格禁止。
