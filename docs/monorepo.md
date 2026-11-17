# Monorepo layout

The existing Cargo workspace already contains the Rust API and Rust web client. Keep working paths stable while adding independent clients:

| Path | Responsibility |
|---|---|
| `Cargo.toml`, `src/` | `puzzgen` API/server and generator library |
| `frontend/` | `puzzgen-web` Leptos/WASM learning site |
| `apps/android/` | Native Rust/Bevy Beacon & Bough Android game, shared models, weather and progression |
| `contracts/openapi.json` | Generated API contract shared with clients |
| `migrations/` | PostgreSQL account schema migrations embedded in the server |
| `tests/`, `examples/` | API/account tests and local benchmark tools |
| `scripts/` | Reviewed generic build and development helpers |


The API serves the web frontend and API routes from one server process. Personal deployment and release automation are excluded from this public source. The native Android game ships separately and connects to the API over HTTPS. Java integrates Android lifecycle, permissions and Firebase authentication; Rust/Bevy renders the game. See [build and sideload instructions](BUILD-AND-SIDELOAD.md) for current source-build instructions.
