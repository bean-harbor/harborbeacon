# HarborNavi K3 RISC-V Builder Precheck - 2026-05-27

## Summary

- Target: build host `.197`.
- Host access: SSH succeeded through the local target registry credentials.
- Purpose: verify whether the existing HarborBeacon builder is ready to produce
  K3 RISC-V binaries for `harbornavi/mlp-vpf-p0`.
- Result: blocked. Rust is installed, but the RISC-V Rust target and GNU linker
  are not installed yet.

## Observed Builder State

```text
host=harbor-innovations-System-Product-Name
arch=x86_64
rustc=rustc 1.95.0 (59807616e 2026-04-14)
cargo=cargo 1.95.0 (f2d3ce0bd 2026-03-21)
installed_targets=x86_64-unknown-linux-gnu,x86_64-unknown-linux-musl
riscv64-linux-gnu-gcc=missing
```

The existing `~/src/HarborBeacon` path is present, but the first probe did not
find it as a normal git checkout. A separate git checkout-like path exists under
`~/src/HarborBeacon.github-20260424-060344`. The K3 build lane should use a
fresh or intentionally selected checkout when the actual smoke build starts.

## Required Before Build Smoke

```bash
rustup target add riscv64gc-unknown-linux-gnu
sudo apt-get update
sudo apt-get install -y gcc-riscv64-linux-gnu g++-riscv64-linux-gnu libc6-dev-riscv64-cross
```

Cargo target configuration:

```toml
[target.riscv64gc-unknown-linux-gnu]
linker = "riscv64-linux-gnu-gcc"
```

## Blocked Smoke Command

This command was not run because the target and linker were missing:

```bash
cargo build --target riscv64gc-unknown-linux-gnu --bin harbor-model-api
```

## Next Step

Prepare `.197` with the RISC-V Rust target and GNU cross toolchain, then rerun
the smoke from a known HarborBeacon checkout of `harbornavi/mlp-vpf-p0`.
