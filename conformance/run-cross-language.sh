#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
temporary_directory="$(mktemp -d "${TMPDIR:-/tmp}/sqlite-replication-conformance.XXXXXX")"
trap 'rm -rf "$temporary_directory"' EXIT

go_source="$temporary_directory/go-source.sqlite"
go_repository="$temporary_directory/go-repository"
rust_from_go="$temporary_directory/rust-from-go.sqlite"
go_delta="$temporary_directory/go-delta.ltx"

rust_source="$temporary_directory/rust-source.sqlite"
rust_repository="$temporary_directory/rust-repository"
go_from_rust="$temporary_directory/go-from-rust.sqlite"
rust_delta="$temporary_directory/rust-delta.ltx"
reference_ltx="$temporary_directory/reference.ltx"
go_from_reference="$temporary_directory/go-from-reference.sqlite"
rust_from_reference="$temporary_directory/rust-from-reference.sqlite"
go_capture_source="$temporary_directory/go-capture-source.sqlite"
go_capture_repository="$temporary_directory/go-capture-repository"
go_reference_chain_restore="$temporary_directory/go-reference-chain-restore.sqlite"
rust_capture_source="$temporary_directory/rust-capture-source.sqlite"
rust_capture_repository="$temporary_directory/rust-capture-repository"
rust_reference_chain_restore="$temporary_directory/rust-reference-chain-restore.sqlite"
fixture="$repository_root/conformance/fixtures/ltx-v2/snapshot-4096.ltx"
fixture_512="$repository_root/conformance/fixtures/ltx-v2/snapshot-512.ltx"
fixture_1024="$repository_root/conformance/fixtures/ltx-v2/snapshot-1024.ltx"
fixture_8192="$repository_root/conformance/fixtures/ltx-v2/snapshot-8192.ltx"
fixture_65536="$repository_root/conformance/fixtures/ltx-v2/snapshot-65536.ltx"

apply_reference_chain() {
    local repository_path="$1"
    local destination_path="$2"
    local -a ltx_files=()
    local ltx_file
    if [[ ! -d "$repository_path/ltx/0" ]]; then
        printf 'LTX chain directory is missing: %s\n' "$repository_path/ltx/0" >&2
        return 1
    fi
    while IFS= read -r ltx_file; do
        ltx_files+=("$ltx_file")
    done < <(find "$repository_path/ltx/0" -type f -name '*.ltx' -print | LC_ALL=C sort)
    if [[ "${#ltx_files[@]}" -lt 3 ]]; then
        printf 'Expected a base snapshot and at least two deltas in %s; found %d files.\n' \
            "$repository_path/ltx/0" "${#ltx_files[@]}" >&2
        return 1
    fi
    (
        cd "$repository_root/conformance/reference"
        go run github.com/superfly/ltx/cmd/ltx apply -db "$destination_path" "${ltx_files[@]}"
    )
}

(
    cd "$repository_root/go"
    go run ./cmd/conformance create "$go_source"
    go run ./cmd/conformance snapshot "$go_source" "$go_repository"
    go run ./cmd/conformance create-delta "$go_delta"
)
(
    cd "$repository_root/conformance/reference"
    go run . verify "$go_repository/ltx/0/0000000000000001-0000000000000001.ltx" "$temporary_directory/reference-verified-go.sqlite"
    go run . verify-ltx "$go_delta"
    go run . create "$go_source" "$reference_ltx"
    for page_size in 512 1024 4096 8192 65536; do
        go run . verify \
            "$repository_root/conformance/fixtures/ltx-v2/snapshot-${page_size}.ltx" \
            "$temporary_directory/reference-fixture-${page_size}.sqlite"
    done
)
(
    cd "$repository_root/go"
    go run ./cmd/conformance verify "$temporary_directory/reference-verified-go.sqlite"
    for page_size in 512 1024 4096 8192 65536; do
        go run ./cmd/conformance verify "$temporary_directory/reference-fixture-${page_size}.sqlite"
    done
    go run ./cmd/conformance decode-ltx "$reference_ltx" "$go_from_reference"
    go run ./cmd/conformance verify "$go_from_reference"
    go run ./cmd/conformance decode-ltx "$fixture" "$temporary_directory/go-from-fixture.sqlite"
    go run ./cmd/conformance verify "$temporary_directory/go-from-fixture.sqlite"
    go run ./cmd/conformance decode-ltx "$fixture_512" "$temporary_directory/go-from-fixture-512.sqlite"
    go run ./cmd/conformance verify "$temporary_directory/go-from-fixture-512.sqlite"
    go run ./cmd/conformance decode-ltx "$fixture_1024" "$temporary_directory/go-from-fixture-1024.sqlite"
    go run ./cmd/conformance verify "$temporary_directory/go-from-fixture-1024.sqlite"
    go run ./cmd/conformance decode-ltx "$fixture_8192" "$temporary_directory/go-from-fixture-8192.sqlite"
    go run ./cmd/conformance verify "$temporary_directory/go-from-fixture-8192.sqlite"
    go run ./cmd/conformance decode-ltx "$fixture_65536" "$temporary_directory/go-from-fixture-65536.sqlite"
    go run ./cmd/conformance verify "$temporary_directory/go-from-fixture-65536.sqlite"
)
(
    cd "$repository_root/rust"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- restore "$go_repository" "$rust_from_go"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- verify "$rust_from_go"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- decode-ltx "$reference_ltx" "$rust_from_reference"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- verify "$rust_from_reference"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- decode-ltx "$fixture" "$temporary_directory/rust-from-fixture.sqlite"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- verify "$temporary_directory/rust-from-fixture.sqlite"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- decode-ltx "$fixture_512" "$temporary_directory/rust-from-fixture-512.sqlite"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- verify "$temporary_directory/rust-from-fixture-512.sqlite"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- decode-ltx "$fixture_1024" "$temporary_directory/rust-from-fixture-1024.sqlite"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- verify "$temporary_directory/rust-from-fixture-1024.sqlite"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- decode-ltx "$fixture_8192" "$temporary_directory/rust-from-fixture-8192.sqlite"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- verify "$temporary_directory/rust-from-fixture-8192.sqlite"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- decode-ltx "$fixture_65536" "$temporary_directory/rust-from-fixture-65536.sqlite"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- verify "$temporary_directory/rust-from-fixture-65536.sqlite"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- create "$rust_source"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- snapshot "$rust_source" "$rust_repository"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- create-delta "$rust_delta"
)
(
    cd "$repository_root/conformance/reference"
    go run . verify "$rust_repository/ltx/0/0000000000000001-0000000000000001.ltx" "$temporary_directory/reference-verified-rust.sqlite"
    go run . verify-ltx "$rust_delta"
)
(
    cd "$repository_root/go"
    go run ./cmd/conformance verify "$temporary_directory/reference-verified-rust.sqlite"
    go run ./cmd/conformance restore "$rust_repository" "$go_from_rust"
    go run ./cmd/conformance verify "$go_from_rust"
)

(
    cd "$repository_root/go"
    go run ./cmd/conformance create-capture-chain "$go_capture_source" "$go_capture_repository"
    go run ./cmd/conformance verify-capture-chain "$go_capture_source"
)
(
    apply_reference_chain "$go_capture_repository" "$go_reference_chain_restore"
)
(
    cd "$repository_root/go"
    go run ./cmd/conformance compare-capture-chain "$go_capture_source" "$go_reference_chain_restore"
)

(
    cd "$repository_root/rust"
    cargo run --quiet -p sqlite-replication-core --example capture-conformance -- \
        "$rust_capture_source" "$rust_capture_repository"
)
(
    apply_reference_chain "$rust_capture_repository" "$rust_reference_chain_restore"
)
(
    cd "$repository_root/go"
    go run ./cmd/conformance compare-capture-chain "$rust_capture_source" "$rust_reference_chain_restore"
)
(
    cd "$repository_root/rust"
    cargo run --quiet -p sqlite-replication-core --bin replication-conformance -- \
        verify "$rust_reference_chain_restore"
)

(
    cd "$repository_root/go"
    go build -o "$temporary_directory/go-conformance" ./cmd/conformance
)
(
    cd "$repository_root/rust"
    # Large snapshot cases exercise the optimized library, not Rust's opt-level=0
    # checksum/decompression loops. Unit and integration suites still run debug.
    cargo build --quiet --release -p sqlite-replication-core --bin replication-conformance
    # Copy through Cargo rather than assuming a target directory: CARGO_TARGET_DIR
    # and workspace configuration can redirect the executable.
    target_directory="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
    cp "$target_directory/release/replication-conformance" "$temporary_directory/rust-conformance"
)
(
    cd "$repository_root/conformance/reference"
    go build -o "$temporary_directory/reference-conformance" .
)
python3 -B -m unittest discover -s "$repository_root/conformance" -p 'test_*.py'
python3 -B "$repository_root/conformance/snapshot_matrix.py" \
    --go "$temporary_directory/go-conformance" \
    --rust "$temporary_directory/rust-conformance" \
    --reference "$temporary_directory/reference-conformance" \
    --temporary-root "$temporary_directory"

printf '%s\n' 'Go ↔ Rust and superfly/ltx v0.4.0 LTX v2 conformance passed, including independent restore of both captured LTX chains and the snapshot workload matrix.'
