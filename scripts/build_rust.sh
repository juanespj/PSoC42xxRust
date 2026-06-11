#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../rust"
cargo build -p rust_firmware --target thumbv6m-none-eabi --release "$@"
echo "Output: rust/rust_firmware/build/thumbv6m-none-eabi/release/librust_firmware.a"
