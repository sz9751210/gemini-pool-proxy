# Configuration Guide

For `gemini-pool-proxy` to function effectively, it reads numerous startup parameters and proxy behaviors from the `.env` file at its root. 
This document highlights all configurable variables from `.env.example`, aiding you in optimizing the load balancing routing and security features.

## Authentication & Security

These parameters determine how your proxy server authenticates users, and how the proxy authenticates with Google APIs.

Recommended standard for all new clients:
- Base URL: `http://127.0.0.1:18080/v1`
- Auth: `Authorization: Bearer <proxy_token>`
- Keep `x-api-key` / `x-goog-api-key` / `?key=` only for compatibility use.

| Parameter | Type | Default / Example | Description |
| :--- | :--- | :--- | :--- |
| `AUTH_TOKEN` | String | `sk-admin-123456` | **Admin Session Token**. Used exclusively to log in to the Tauri Desktop dashboard or invoke protected `/api/v1/*` admin interfaces. |
| `ALLOWED_TOKENS` | Array | `["sk-user-123456"]` | **Proxy API Keys**. Standard usage is `Authorization: Bearer <token>`. Compatibility inputs (`x-api-key`, `x-goog-api-key`, `?key=`, `?api_key=`) are also accepted. |
| `API_KEYS` | Array | `["AIzaSy..."]` | **Real Google Gemini API Keys**. Paste your actual AI Studio generated Keys here. The proxy server performs load-balancing over this array of keys toward the official Google endpoints. |

## .env Format Recommendation (Important)

For JSON/array values, use single quotes outside JSON to avoid parser ambiguity:

```dotenv
ALLOWED_TOKENS='["sk-user-1","sk-user-2"]'
API_KEYS='["AIzaSy_xxx","AIzaSy_yyy"]'
MODEL_POOLS='{"sonnet":["gemma-4-26b-a4b-it"],"fast":["gemini-2.5-flash"]}'
```

## Model Pools & Routing

Since many tools strictly restrict or hardcode model strings (like `claude-3-5-sonnet`), the Model Pool allows your proxy to perform intelligent translation and redirection seamlessly.

| Parameter | Type | Default / Example | Description |
| :--- | :--- | :--- | :--- |
| `MODEL_POOLS` | JSON | `{"fast":["gemini-2.5-flash"]}` | Defines model aliases. If a client requests `fast`, the proxy translates it automatically to `gemini-2.5-flash` while rotating through configured models. |
| `THINKING_MODELS` | Array | `["gemini-2.5-flash"]` | Registers models that support "Thinking" capability workflows. |
| `SEARCH_MODELS` | Array | `["gemini-2.5-pro"]` | Registers models capable of using the "Google Search" tool. |
| `IMAGE_MODELS` | Array | `["gemini-2.0-flash-exp"]` | Registers models dedicated to multi-modal image generation/understanding. |
| `URL_CONTEXT_MODELS`| Array | `["gemini-2.5-pro"]` | Registers models that can directly fetch and understand URL contexts. |
| `FILTERED_MODELS` | Array | `["chat-bison-001"]` | **Blacklist Filter**. If a client requests a model from this array, the proxy halts and refuses the connection instantly. |

## Pooling & Network Strategy

Dictates how your Proxy deals with Rate limits and unstable connections.

| Parameter | Type | Default / Example | Description |
| :--- | :--- | :--- | :--- |
| `POOL_STRATEGY` | Enum | `round_robin` | Defines the API Key rotation strategy. Options: `round_robin`, `random`, `least_fail`. |
| `MAX_FAILURES` | Int | `3` | The maximum number of continuous API connection failures or Rate Limits allowed per API Key before it is sent to "Cooldown". |
| `COOLDOWN_SECONDS`| Int | `60` | The duration an API Key spends in the "Cooldown / Invalid" state before the proxy attempts to utilize it again. |
| `COMPAT_MODE` | Bool | `true` | Whether to engage the OpenAI Auto-Compatibility layer. Transforms payload natively to Gemini structure. |

## Local Server Binding

Settings ensuring your local network security boundary.

| Parameter | Type | Default / Example | Description |
| :--- | :--- | :--- | :--- |
| `RUNTIME_BIND_HOST`| String | `127.0.0.1` | **CRITICAL**: Defaults to localhost. Ensures nobody else on your WiFi or public network can hit your proxy API. |
| `RUNTIME_PORT_START`| Int | `18080` | Beginning of the port range the proxy will attempt to listen on. |
| `RUNTIME_PORT_END` | Int | `18099` | Maximum acceptable port bump logic if `PORT_START` is occupied. |
| `SESSION_COOKIE_NAME`| String| `gb_session` | Name of the secure Cookie generated for web/Tauri management sessions. |
