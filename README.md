# Puzzgen and Beacon & Bough

A free, open source place to learn game development, and a welcoming game to
enjoy the results. Puzzgen provides seeded puzzle data, a Rust web playground
and interactive teaching pages. Beacon & Bough is its native Rust/Bevy Android
companion: small puzzles, a guiding Beacon and a personal garden.

I began the game so my mother could enjoy puzzles without being tricked by
deceptive advertisements. Older people deserve to play without deciphering
whether a button belongs to a game or an ad. The game will remain free and
ad-free. It makes no medical or therapeutic claims.

## What is here

- **Puzzgen:** a seeded puzzle API, browser playground and interactive lessons
  for learning interface and game development.
- **Beacon & Bough:** a native Android companion with puzzle games, a guiding
  Beacon and rewards that grow a personal garden.

This is a development preview. Source builds are available for experimentation;
not every device, game or accessibility workflow has completed human review.
The original code is free to use under MIT. Third-party materials keep their
own licenses, including attribution requirements and Google SDK terms.

## Build the API and playground

Install Rust stable, native compiler tools and PowerShell 7. Windows development
uses MSVC build tools; Linux native builds need a compiler toolchain. The server
and frontend are Axum/Tokio and Leptos/WebAssembly. Node is not required.

```powershell
git clone https://github.com/Dave-Wagner/puzzgen-community.git
Set-Location puzzgen-community
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.22.1 --locked
pwsh -NoProfile -File ./scripts/Build-Frontend.ps1
cargo run --locked -p puzzgen
```

Windows x64 can alternatively use `scripts/Setup-Toolchain.ps1` to install a
checksum-verified project-local Trunk. Open http://127.0.0.1:8080 for the
playground, `/lessons/` for learning pages, `/docs` for the API reference and
`/api/v1/openapi.json` for the schema. All generators are deterministic by seed;
language puzzles draw on finite curated CC0 corpora and can repeat.

The server reads process environment variables, not `.env` automatically.
See `.env.example`. PostgreSQL accounts are optional for local stateless
experiments; use a dedicated database when enabling accounts, apply migrations
explicitly, then create the protected owner interactively:

```powershell
cargo run --locked -p puzzgen -- migrate
cargo run --locked -p puzzgen -- create-admin
```

Never reuse the maintainer's database, credentials, signing key or family key.
Public hosting is personally funded and capacity-limited. Use bundled practice
fixtures or your own backend for experiments that generate sustained traffic.
Hosted service access has quotas and no unlimited availability promise.
Production and testing Firebase projects must remain separate.

## Build and sideload Android

See [build and sideload instructions](docs/BUILD-AND-SIDELOAD.md).
The public helper builds only; installation is an explicit ADB command using
your own device serial. Independent builds use their builder's signing key and
cannot replace an official app signed with another key. Stop on signature
mismatches instead of uninstalling and losing saves.

## Tests and licenses

`cargo test --locked -p puzzgen` runs the core API tests without configuring a
live database. Database-specific tests require a dedicated test database. The
game model, weather and progression crates have their own focused tests. Build
verification does not replace human touch, accessibility or device review.

Original code is MIT; clue datasets are CC0. Asset, font, dependency and audio
licenses are distinct: see LICENSE, THIRD-PARTY-NOTICES.md and docs/licenses.
CC-BY music retains attribution. Exact MPL decoder source archives are retained
under third_party/symphonia-source. No maintainer convenience, signing, backup,
key-provisioning or private deployment tools belong in the public distribution.
