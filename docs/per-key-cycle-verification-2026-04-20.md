# Per-Key-Cycle Verification Report (2026-04-20)

## Scope
- Verify `MODEL_POOL_STRATEGY=per_key_cycle` behavior with multiple API keys and multiple models.
- Confirm sequence requirement:
  - `api-key-1 + model-1`
  - `api-key-2 + model-1`
  - `...`
  - after one full key round, switch to next model.

## Environment
- Service mode: `./start-headless.sh`
- Runtime config:
  - `MODEL_POOL_STRATEGY=per_key_cycle`
  - key pool strategy set to `round_robin` during verification
  - alias: `claude-sonnet`
  - alias models: `["gemma-4-26b-a4b-it","gemma-4-31b-it"]`
  - active keys: `4`

## Method
1. Validate auth behavior:
   - `/v1/models` without token => `401`
   - `/v1/models` with user token => `200`
2. Reset key failures and clear logs.
3. Send 8 sequential `POST /v1/chat/completions` requests with `model=claude-sonnet`.
4. Read:
   - `/api/v1/pool/status?limit=8` (key selection sequence)
   - `/api/v1/logs?limit=8&sort_by=id&sort_order=asc` (resolved model sequence)
5. Assert expected sequence formula:
   - `expected_key(i) = key_order[i % active_key_count]`
   - `expected_model(i) = alias_models[(i / active_key_count) % model_count]`

## Result
- Verification status: **PASS**
- Observed sequence:

| # | keyId | model | status |
| --- | --- | --- | --- |
| 1 | key-1 | gemma-4-26b-a4b-it | 502 |
| 2 | key-2 | gemma-4-26b-a4b-it | 502 |
| 3 | key-3 | gemma-4-26b-a4b-it | 502 |
| 4 | key-4 | gemma-4-26b-a4b-it | 502 |
| 5 | key-1 | gemma-4-31b-it | 502 |
| 6 | key-2 | gemma-4-31b-it | 502 |
| 7 | key-3 | gemma-4-31b-it | 502 |
| 8 | key-4 | gemma-4-31b-it | 502 |

## Notes
- `502` is expected in this run because upstream was intentionally made unreachable to force fast local failure and isolate routing logic.
- Even with upstream failure, key/model routing decisions are still recorded and validated from local pool/log state.
- This confirms the required behavior for multi-key + multi-model `per_key_cycle` rotation.
