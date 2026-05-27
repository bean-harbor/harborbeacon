# HarborNavi K3 RISC-V Builder Precheck - 2026-05-27

## Summary

- Target: build host `.197`.
- Host access: SSH succeeded through the local target registry credentials.
- Purpose: verify whether the existing HarborBeacon builder is ready to produce
  K3 RISC-V binaries for `harbornavi/mlp-vpf-p0`.
- Result: passed after installing the RISC-V Rust target and GNU cross linker.
  The first probe was blocked because those tools were missing; the follow-up
  setup and smoke build completed on the same builder.

## Initial Builder State

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

## Setup Applied

```bash
rustup target add riscv64gc-unknown-linux-gnu
sudo apt-get update
sudo apt-get install -y gcc-riscv64-linux-gnu g++-riscv64-linux-gnu libc6-dev-riscv64-cross
```

Cargo target configuration for this smoke was provided through an environment
variable, not committed repo config:

```bash
export CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER=riscv64-linux-gnu-gcc
```

## Follow-Up Builder State

Fresh checkout path:

```text
/home/harbor-innovations/src/HarborBeacon-harbornavi-mlp-vpf-p0
```

Verified branch head:

```text
5ce91a3 Add HarborNavi MLP VPF privacy guards
```

Toolchain:

```text
rustc=rustc 1.95.0 (59807616e 2026-04-14)
cargo=cargo 1.95.0 (f2d3ce0bd 2026-03-21)
installed_targets=riscv64gc-unknown-linux-gnu,x86_64-unknown-linux-gnu,x86_64-unknown-linux-musl
riscv64-linux-gnu-gcc=riscv64-linux-gnu-gcc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0
```

## Smoke Results

Host policy tests:

```bash
cargo test --lib privacy --quiet
cargo test --lib model_center -- --test-threads=1
```

Result:

```text
privacy: 5 passed
model_center: 17 passed
```

RISC-V build command:

```bash
CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER=riscv64-linux-gnu-gcc \
  cargo build --target riscv64gc-unknown-linux-gnu --bin harbor-model-api
```

Result:

```text
target/riscv64gc-unknown-linux-gnu/debug/harbor-model-api:
ELF 64-bit LSB pie executable, UCB RISC-V, RVC, double-float ABI,
dynamically linked, interpreter /lib/ld-linux-riscv64-lp64d.so.1,
for GNU/Linux 4.15.0, with debug_info, not stripped
```

No TLS, native C dependency, Candle, `reqwest`, or linker blocker appeared in
this smoke build.

## Follow-Up Package Deployment

The same builder lane later produced a real K3 Debian package from commit
`b7fcfd5`:

```text
harboros-beacon_harbornavi-p0-20260527+riscv64_riscv64.deb
sha256=125915fb243b9e0b555f457b5712441c5f746c60d8ec76c55426b69a9af7ef83
architecture=riscv64
```

The package was installed on the K3 Bianbu board and verified as a systemd
service. Deployment evidence is recorded in:

- `docs/harbornavi-k3-deployment-2026-05-27.md`
