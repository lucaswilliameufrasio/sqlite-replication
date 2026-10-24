#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scratch_root="/tmp/opencode"
image="debian@sha256:62bd423aeee912639ca8ee681c2b762cb3e25137e0a92b20be455898a9f034ad"

if [[ ! -d "$scratch_root" ]]; then
    printf 'Required temporary directory does not exist: %s\n' "$scratch_root" >&2
    exit 1
fi

docker_contexts="$(docker context ls --format '{{.Name}}')"
if ! grep -Fxq 'rootless' <<< "$docker_contexts"; then
    printf '%s\n' 'The bounded ENOSPC drill requires a verified rootless Docker context.' >&2
    exit 1
fi
security_options="$(docker --context rootless info --format '{{json .SecurityOptions}}')"
if [[ "$security_options" != *name=rootless* ]]; then
    printf '%s\n' 'The rootless Docker context is not connected to a verified rootless daemon.' >&2
    exit 1
fi
docker_command=(docker --context rootless)

if ! "${docker_command[@]}" image inspect "$image" >/dev/null 2>&1; then
    printf 'Pinned local drill image is not available; no image will be pulled: %s\n' "$image" >&2
    exit 1
fi

temporary_directory="$(mktemp -d "$scratch_root/sqlite-replication-cp01-enospc.XXXXXX")"
cleanup() {
    result=$?
    trap - EXIT
    case "$temporary_directory" in
        "$scratch_root"/sqlite-replication-cp01-enospc.*)
            rm -rf -- "$temporary_directory"
            ;;
        *)
            printf 'Refusing to remove unexpected temporary path: %s\n' "$temporary_directory" >&2
            if [[ "$result" -eq 0 ]]; then
                result=1
            fi
            ;;
    esac
    exit "$result"
}
trap cleanup EXIT

(
    cd "$repository_root/go"
    TMPDIR="$temporary_directory" go test -c -o "$temporary_directory/go-capture.test" ./internal/capture
)

(
    cd "$repository_root"
    TMPDIR="$temporary_directory" cargo test \
        --manifest-path rust/Cargo.toml \
        -p sqlite-replication-core \
        --lib \
        --no-run \
        --message-format=json > "$temporary_directory/rust-build.jsonl"
)
python3 - "$temporary_directory/rust-build.jsonl" "$temporary_directory/rust-core-tests" <<'PY'
import json
import os
import shutil
import sys

build_output, destination = sys.argv[1:]
executable = None
with open(build_output, encoding="utf-8") as stream:
    for line in stream:
        record = json.loads(line)
        target = record.get("target", {})
        profile = record.get("profile", {})
        if (
            record.get("reason") == "compiler-artifact"
            and target.get("name") == "sqlite_replication_core"
            and "lib" in target.get("kind", [])
            and profile.get("test")
            and record.get("executable")
        ):
            executable = record["executable"]
if executable is None:
    raise SystemExit("Cargo did not report the sqlite-replication-core test executable")
shutil.copyfile(executable, destination)
os.chmod(destination, 0o755)
PY

"${docker_command[@]}" run \
    --pull=never \
    --rm \
    --network none \
    --read-only \
    --memory 512m \
    --pids-limit 128 \
    --tmpfs /tmp:rw,size=32m,mode=1777 \
    --tmpfs /cp01:rw,size=8m,mode=1777 \
    --mount "type=bind,src=$temporary_directory/go-capture.test,dst=/go-capture.test,readonly" \
    --mount "type=bind,src=$temporary_directory/rust-core-tests,dst=/rust-core-tests,readonly" \
    --env TMPDIR=/tmp \
    --entrypoint /bin/sh \
    "$image" \
    -eu -c '
        SQLITE_REPLICATION_ENOSPC_ROOT=/cp01 /go-capture.test \
            -test.run "^TestCaptureRecoversAfterRealENOSPCOnBoundedTmpfs$" -test.v
        SQLITE_REPLICATION_ENOSPC_ROOT=/cp01 /rust-core-tests \
            --ignored --exact capture::tests::capture_recovers_after_real_enospc_on_bounded_tmpfs --nocapture
    '
