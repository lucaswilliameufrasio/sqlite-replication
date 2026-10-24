#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_path="${1:-$repository_root/conformance/fixtures/ltx-v2/snapshot-4096.ltx}"
page_size="${2:-4096}"
if [[ -e "$fixture_path" ]]; then
    printf 'Refusing to overwrite existing fixture: %s\n' "$fixture_path" >&2
    exit 1
fi

temporary_directory="$(mktemp -d "${TMPDIR:-/tmp}/sqlite-replication-fixture.XXXXXX")"
trap 'rm -rf "$temporary_directory"' EXIT
mkdir -p "$(dirname "$fixture_path")"

(
    cd "$repository_root/go"
    go run ./cmd/conformance create "$temporary_directory/source.sqlite" "$page_size"
)
(
    cd "$repository_root/conformance/reference"
    go run . create "$temporary_directory/source.sqlite" "$temporary_directory/snapshot.ltx"
)
cp "$temporary_directory/snapshot.ltx" "$fixture_path"
sha256sum "$fixture_path"
