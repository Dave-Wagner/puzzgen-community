# Windows and Linux builds

Development uses native Windows Rust. The API targets Linux x86-64 with musl,
and the Leptos frontend targets WebAssembly. The frontend is built on Windows
and its generated assets work on either server OS. Node.js is not required.

## First setup on Windows x64

From the repository root in PowerShell:

```powershell
./scripts/Setup-Toolchain.ps1
```

This installs `x86_64-unknown-linux-musl` and `wasm32-unknown-unknown` into the
existing rustup toolchain. It installs Trunk 0.22.1 in ignored `.tools/trunk/`
and checks its pinned upstream SHA-256 before extraction. It does not change
the global PATH or install Docker, Zig, WSL, or an administrator service.
Internet access is required for the first download and initial crate build.
Trunk also downloads the `wasm-bindgen` CLI matching the resolved Rust crate.

For only the backend targets, pass `-SkipFrontendTools`.

## Build

```powershell
cargo build --locked --release -p puzzgen  # Windows executable
./scripts/Build-Linux.ps1                 # Linux x86-64 executable
./scripts/Build-Frontend.ps1              # Portable browser assets
```

Outputs:

| Purpose | Path |
| --- | --- |
| Windows server | `target/release/puzzgen.exe` |
| Linux server | `target/x86_64-unknown-linux-musl/release/puzzgen` |
| Frontend | `frontend/dist/` |

The build scripts also accept `-Development`. `Build-Linux.ps1` checks the ELF header
so an accidentally copied Windows executable cannot pass as a Linux build.
The workspace `default-members` selects the backend; use `-p puzzgen-web` when
checking the frontend directly.

## Why no Linux linker installation?

`.cargo/config.toml` selects the `rust-lld` linker bundled with Rust, the musl
runtime bundled with the Linux Rust target, and static C-runtime linking.
The current backend dependencies are pure Rust. This avoids the usual missing
Linux `cc`/GCC problem when building on Windows. The target is intentionally
scoped so `cargo run` continues to produce and run a Windows executable.

If a future dependency requires a C/C++ build script (for example some database,
TLS, compression, or image libraries), the bundled linker alone may be
insufficient. Prefer pure Rust dependencies, or deliberately add a C toolchain
such as [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) at that time.
Do not set `target-cpu=native` for VPS artifacts: the VPS CPU may differ.

## Linux development

Install stable Rust using rustup and Trunk, then:

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked --version 0.22.1 trunk
cargo build --locked --release -p puzzgen
cd frontend
trunk build --release --locked
```

A native Linux GNU build is supported as well as the cross-built musl binary.
The provided Windows cross-build target is **x86-64**, not ARM64; check `uname -m`
on the VPS before copying it. An ARM64 VPS should build natively or use an
explicitly configured ARM64 cross compiler.

## Verification on this machine

On 2026-10-26, Rust/Cargo 1.99.1 on Windows successfully installed both targets
and `scripts/Build-Linux.ps1` compiled the actual API in release mode using
`rust-lld`. LLVM inspection reported `elf64-x86-64`, with ELF magic
`7f 45 4c 46`, machine `EM_X86_64`, no `PT_INTERP` dynamic loader, and an empty
`NeededLibraries` list. The initial release executable was 4,077,632 bytes.
Trunk 0.22.1 was downloaded from its official GitHub release and
matched SHA-256 `1cc65012a9df60b02db948f5a3f0601e594ce952efb1cfd23147c9021c9f7d75`.
WSL and Docker are absent on this machine, so executing an ELF binary requires
the VPS or another Linux machine. Build success and runtime testing are
separate checks. Personal VPS deployment tooling is excluded from this source.

Upstream references: [Rust platform support](https://doc.rust-lang.org/rustc/platform-support.html),
[static linking](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes),
[Trunk source and documentation](https://github.com/trunk-rs/trunk).
