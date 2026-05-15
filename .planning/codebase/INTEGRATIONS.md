# External Integrations

**Analysis Date:** 2026-05-15

## APIs & External Services

### Chia Full-Node Peer (Light-Wallet Protocol)

**Service:** Chia full-node peer endpoints (peer discovery, coin subscriptions, puzzle state queries)
- **Crate:** `crates/chia-sdk-client`
- **Wire Format:** Binary WebSocket protocol (Chia protocol messages, not JSON)
- **TLS:** Required; two options:
  - `native-tls` feature (0.2.14) — Platform native TLS (OpenSSL/Secure Transport/SChannel)
  - `rustls` feature (0.23.32) — Pure-Rust TLS with aws-lc-rs bindgen
- **Connection:** `Peer::connect()` in `crates/chia-sdk-client/src/peer.rs`
  - Accepts `SocketAddr` + `Connector` (from TLS setup)
  - Establishes `wss://ip:port/ws` WebSocket with TLS handshake
  - Returns `(Peer, mpsc::Receiver<Message>)` for bidirectional messaging
- **Key Functions:**
  - `Peer::request_*()` - Request-response patterns (coin state, puzzle solutions, peers)
  - `Peer::subscribe_*()` - Subscription patterns (coin updates, puzzle updates)
  - `Peer::send_transaction()` - Broadcast spend bundle
  - `Peer::send_raw_message()` - Arbitrary protocol message
- **Protocol Messages:** `chia-protocol 0.36.1`
  - `RequestCoinState` / `RespondCoinState`
  - `RequestPuzzleSolution` / `RespondPuzzleSolution`
  - `RequestTransaction` / `RespondTransaction`
  - `SendTransaction` / `TransactionAck`
  - `RequestPeers` / `RespondPeers`
  - `RegisterForCoinUpdates` / `RespondToCoinUpdates`
  - `RegisterForPhUpdates` / `RespondToPhUpdates`
- **Error Handling:** `ClientError` in `crates/chia-sdk-client/src/error.rs`
- **Rate Limiting:** Built-in per-peer rate limiter (`RateLimiter` in `crates/chia-sdk-client/src/rate_limiter.rs`)
  - Configurable via `PeerOptions.rate_limit_factor` (default 0.6)
  - Applies V2 rate limits from `rate_limits.rs`

### Chia Coinset REST API

**Service:** Full-node coinset REST endpoint (paginated coin records, cursor support added in commit `7f5f0bb0`)
- **Crate:** `crates/chia-sdk-coinset`
- **Wire Format:** JSON over HTTP (reqwest)
- **TLS:** Via reqwest features:
  - `native-tls` feature — Platform native TLS
  - `rustls` feature — Pure-Rust TLS
- **Connection:** `CoinsetClient::new()` in `crates/chia-sdk-coinset/src/coinset_client.rs`
  - Accepts base URL string
  - Static endpoints: `CoinsetClient::mainnet()`, `CoinsetClient::testnet11()`
  - Internal: reqwest `Client` without credentials
- **Key Functions:**
  - Implements `ChiaRpcClient` trait (defined in `crates/chia-sdk-coinset/src/chia_rpc_client.rs`)
  - `make_post_request<R, B>()` - Generic JSON RPC-style requests
  - Endpoints (per `bindings/rpc.json`):
    - `get_coin_records_by_...` - Coin queries (by parent, by puzzle hash, by hint, etc.)
    - Support for cursor-based pagination (coin record sets)
- **Response Format:** `CoinRecord`, `CoinRecordWithStatus` (from `chia-protocol 0.36.1`)
  - Cursor for pagination if truncated results
- **Error Handling:** Delegates to reqwest; returns `reqwest::Error`
- **Authentication:** None; public REST API

### Chia Daemon Websocket Client

**Service:** Chia daemon RPC (peer, crawler, harvester, farmer control plane) — Added commit `ec1a3517`
- **Crate:** `crates/chia-sdk-daemon`
- **Wire Format:** Binary WebSocket protocol with JSON-RPC requests/responses (on top of Chia daemon protocol)
- **TLS:** Required; inherits from `chia-sdk-client`:
  - `native-tls` feature — Via `chia-sdk-client/native-tls`
  - `rustls` feature — Via `chia-sdk-client/rustls`
- **Connection:** `DaemonClient::connect()` in `crates/chia-sdk-daemon/src/client.rs`
  - Accepts URL (e.g. `wss://localhost:55400`), `Connector`, timeout duration
  - Uses `tokio_tungstenite::connect_async_tls_with_config()` with TLS connector
  - Returns `DaemonClient` (handles reconnection, subscriptions internally)
  - Origin header: `chia-wallet-sdk-{nanos}` (nanosecond timestamp)
- **Key Functions:**
  - `rpc()` - Generic JSON-RPC call with request ID tracking
  - `subscribe_to_*()` - Event subscriptions (peer added/removed, block events, etc.)
  - `unsubscribe_from_*()` - Unsubscribe from events
  - `disconnect()` - Graceful disconnect
  - `reconnect()` - Manual reconnection trigger
- **Message Format:** `WebsocketRequest` / `WebsocketResponse` (JSON with request ID)
  - Fields: `{"origin": "chia-wallet-sdk-...", "command": "subscribe", "data": {...}, "id": "123"}`
- **Event Broadcasting:** Uses `tokio::sync::broadcast` for event distribution
  - `subscribe_events()` returns `broadcast::Receiver<DaemonEvent>`
  - Supports multiple concurrent listeners
- **Connection State:** Tracked via `watch::Sender<bool>` (connected flag) and broadcast channels (disconnect, reconnect)
- **Error Handling:** `DaemonError` in `crates/chia-sdk-daemon/src/error.rs`
  - Maps from WebSocket errors, JSON parse errors, timeout errors
- **Also Implements:** `ChiaRpcClient` trait from `chia-sdk-coinset`
  - Allows daemon to be used as RPC backend for SDK operations requiring full-node data

## Data Storage

**Databases:**
- No embedded database; SDK is a library, not a server
- Consumers persist state via their own storage (wallets use local SQLite, etc.)

**File Storage:**
- No direct file integration; examples write to stdout or files via consumer code

**Caching:**
- No external caching service used
- In-memory request tracking via `Arc<RequestMap>` in peer client for request-response correlation
- Rate limiter state is in-process

## Authentication & Identity

**Peer Authentication:**
- TLS mutual authentication via certificates (required by full-node peers)
- Certificate hashing into peer ID (extracted from TLS session in `peer.rs`)
- Configured via `create_native_tls_connector()` or `create_rustls_connector()` in `crates/chia-sdk-client/src/tls.rs`
  - Loads client certificate, client key, root CA from files
  - Verifies server certificate against CA chain

**Daemon Authentication:**
- TLS mutual authentication with daemon (same cert/key infrastructure)
- No username/password; identity is via TLS certificate

**No OAuth / API Key services** used

## Monitoring & Observability

**Error Tracking:**
- No integration with external error tracking (Sentry, etc.)
- Errors are returned to consumer as typed enums (`ClientError`, `DaemonError`, etc.)

**Logs:**
- Uses `tracing` crate (0.1.41) for structured logging
- Emits debug/warn/error/info spans and events
- Consumer controls subscriber setup (format, filtering, output)
- Examples: `tracing::debug!()`, `tracing::warn!()` in peer client, daemon client

**Diagnostics:**
- No external APM service
- Peer rate limiter can be queried for current limit state

## CI/CD & Deployment

**Hosting:**
- Published to crates.io (Rust packages)
- Published to npm (napi/wasm packages)
- Published to PyPI (Python wheels)
- Consumed as vendored dependency by wallet/dApp projects

**CI Pipeline:**
- GitHub Actions workflows: `rust.yml`, `napi.yml`, `wasm.yml`, `pyo3.yml`
- Builds on macOS, Windows, Linux (x86_64 and aarch64)
- Cross-platform binaries for all three binding targets
- Cargo-based Rust build + feature matrix testing
- maturin for Python wheels with platform-specific sccache

## Environment Configuration

**Required env vars:**
- None mandatory; all configuration is API-driven
- Optional: `RUST_LOG` for tracing subscriber (if consumer sets up logging)

**TLS Paths (Runtime):**
- Consumer provides file paths to cert/key files when calling `create_native_tls_connector()` or `create_rustls_connector()`
- Passed as `Path` arguments; SDK loads and validates at connection time
- Typical paths (consumer-specific): `~/.chia/mainnet/ssl/full_node/*` (Chia node installation)

**Secrets location:**
- No embedded secrets; consumers manage TLS keys
- Recommended: store certificates/keys in OS keystore or secure file storage

## Webhooks & Callbacks

**Incoming:**
- Peer: message inbound stream (`mpsc::Receiver<Message>`) — consumer polls for messages from peer
- Daemon: event broadcast channel (`broadcast::Receiver<DaemonEvent>`) — consumer subscribes to daemon events

**Outgoing:**
- None; SDK makes outbound requests (on-demand RPC) rather than callbacks

## Bindings Surface

**Exposed via bindings descriptors** (`bindings/*.json`):
- `peer.json` — Peer connection, messaging, subscriptions
- `rpc.json` — Coinset and daemon RPC (wrapped as `ChiaRpcClient`)
- Other facades: clvm.rs, simulator.rs, puzzles.rs, etc. for general SDK features

**Not exposed (future work):**
- Silent-payments tweak-service client (no CHIP-0058 light-wallet protocol yet)

## Transport Layer Summary

| Service | Crate | Protocol | Wire Format | TLS | Connection API |
|---------|-------|----------|-------------|-----|-----------------|
| Peer (light-wallet) | chia-sdk-client | Custom Chia P2P | Binary WebSocket | Mutual cert | `Peer::connect()` |
| Coinset (coin records) | chia-sdk-coinset | JSON-RPC (REST) | JSON over HTTP | native-tls/rustls | `CoinsetClient::new()` |
| Daemon (RPC) | chia-sdk-daemon | JSON-RPC (WS) | Binary WS + JSON | Mutual cert | `DaemonClient::connect()` |

---

*Integration audit: 2026-05-15*
