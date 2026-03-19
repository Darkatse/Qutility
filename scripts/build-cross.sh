#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(CDPATH= cd -- "${SCRIPT_DIR}/.." && pwd)"
EXTRA_ARGS=()

usage() {
    cat <<'EOF'
Usage:
  ./scripts/build-cross.sh <target-alias> [cargo-args...]
  ./scripts/build-cross.sh list

Target aliases:
  macos-x64        -> x86_64-apple-darwin
  macos-x64        -> x86_64-apple-darwin
  macos-universal  -> universal2 macOS binary (macOS host only)
  linux-x64        -> x86_64-unknown-linux-gnu
  linux-arm64      -> aarch64-unknown-linux-gnu
  windows-x64      -> x86_64-pc-windows-gnu

Examples:
  ./scripts/build-cross.sh macos-x64
  ./scripts/build-cross.sh macos-universal
  ./scripts/build-cross.sh linux-x64
  ./scripts/build-cross.sh windows-x64 --features foo
  ./scripts/build-cross.sh linux-x64 --features foo

Notes:
  - This helper is intentionally opinionated.
  - Apple targets are built with native cargo.
  - This helper is intentionally opinionated.
  - Extra arguments are forwarded to cargo/cross build.
EOF
}

print_targets() {
    cat <<'EOF'
Supported targets:
  macos-arm64
  macos-x64
  macos-universal
  linux-x64
  linux-arm64
  windows-x64
EOF
}

require_command() {
    local cmd="$1"

    if ! command -v "${cmd}" >/dev/null 2>&1; then
        echo "Error: missing required command '${cmd}'." >&2
        exit 1
    fi
}

native_build() {
    local target="$1"
    local binary_path="${PROJECT_ROOT}/target/${target}/release/qutility"

    require_command cargo
    require_command rustup

    rustup target add "${target}"
    cargo build --release --target "${target}" "${EXTRA_ARGS[@]}"

    echo "Built ${target}: ${binary_path}"
}

cross_build() {
    local target="$1"
    local binary_name="qutility"
    local binary_path="${PROJECT_ROOT}/target/${target}/release/${binary_name}"

    require_command cross

    if [[ "${target}" == *windows* ]]; then
        binary_name="qutility.exe"
        binary_path="${PROJECT_ROOT}/target/${target}/release/${binary_name}"
    fi

    cross build --release --target "${target}" "${EXTRA_ARGS[@]}"
    echo "Built ${target}: ${binary_path}"
}

build_universal() {
    local arm_target="aarch64-apple-darwin"
    local x64_target="x86_64-apple-darwin"
    local output_dir="${PROJECT_ROOT}/target/universal2-apple-darwin/release"
    local output_path="${output_dir}/qutility"

    require_command cargo
    require_command rustup
    require_command lipo

    rustup target add "${arm_target}" "${x64_target}"
    cargo build --release --target "${arm_target}" "${EXTRA_ARGS[@]}"
    cargo build --release --target "${x64_target}" "${EXTRA_ARGS[@]}"

    mkdir -p "${output_dir}"
    lipo -create \
        -output "${output_path}" \
        "${PROJECT_ROOT}/target/${arm_target}/release/qutility" \
        "${PROJECT_ROOT}/target/${x64_target}/release/qutility"

    echo "Built universal macOS binary: ${output_path}"
}

TARGET_ALIAS="${1:-}"

if [[ -z "${TARGET_ALIAS}" ]]; then
    usage >&2
    exit 1
fi

if [[ "${TARGET_ALIAS}" == "list" ]]; then
    print_targets
    exit 0
fi

if [[ "${TARGET_ALIAS}" == "--help" || "${TARGET_ALIAS}" == "-h" ]]; then
    usage
    exit 0
fi

shift
EXTRA_ARGS=("$@")

cd "${PROJECT_ROOT}"

case "${TARGET_ALIAS}" in
    macos-arm64)
        native_build "aarch64-apple-darwin"
        ;;
    macos-x64)
        native_build "x86_64-apple-darwin"
        ;;
    macos-universal)
        build_universal
        ;;
    linux-x64)
        cross_build "x86_64-unknown-linux-gnu"
        ;;
    linux-arm64)
        cross_build "aarch64-unknown-linux-gnu"
        ;;
    windows-x64)
        cross_build "x86_64-pc-windows-gnu"
        ;;
    *)
        echo "Error: unsupported target alias '${TARGET_ALIAS}'." >&2
        usage >&2
        exit 1
        ;;
esac
