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
| `deploy/` | Docker Compose, host nginx and VPS installation/update tools |
| `scripts/` | Windows/Linux builds, checks and versioned release packaging |
| `.github/workflows/` | GitHub draft development release builds |

The API and web app ship together in one runtime container. The native Android game ships separately and connects to the API over HTTPS. Java integrates Android lifecycle, permissions and Firebase authentication; Rust/Bevy renders the game. See BUILD-AND-SIDELOAD.md for current source-build instructions.
