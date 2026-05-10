set shell := ["bash", "-c"]

# Build the HarborBeacon agent binary
build:
    cargo build --release

# Run all Rust tests
test:
    cargo test

# Run the harborbeacon-agent
run-agent +args:
    ./target/release/harborbeacon-agent {{args}}

# Run validate-contract-schemas
validate-schemas +args:
    ./target/release/validate-contract-schemas {{args}}

# Run run-e2e-suite
run-e2e +args:
    ./target/release/run-e2e-suite {{args}}

# Run run-drift-matrix
run-drift +args:
    ./target/release/run-drift-matrix {{args}}

# Run evaluate-release-gate
evaluate-gate +args:
    ./target/release/evaluate-release-gate {{args}}

# Format the codebase
format:
    cargo fmt

# Lint the codebase
lint:
    cargo clippy
