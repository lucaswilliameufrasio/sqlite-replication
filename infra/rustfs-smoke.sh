#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
go_repository_root="${GO_REPO:-$repository_root/../sqlite-replication-go}"
run_id="$(python3 -c 'import uuid; print(uuid.uuid4().hex)')"
project_name="sqlite-replication-rustfs-${run_id}"
export RUSTFS_PORT="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')"
endpoint="http://127.0.0.1:${RUSTFS_PORT}"

docker_command=(docker --context rootless)
security_options="$("${docker_command[@]}" info --format '{{json .SecurityOptions}}')"
if [[ "$security_options" != *name=rootless* ]]; then
    printf '%s\n' 'RustFS smoke requires a verified rootless Docker daemon.' >&2
    exit 1
fi
compose=("${docker_command[@]}" compose --project-name "$project_name" --file "$repository_root/infra/rustfs-compose.yaml")
created=0
cleanup() {
    result=$?
    trap - EXIT
    if [[ "$created" -eq 1 ]]; then
        if ! "${compose[@]}" down --volumes --remove-orphans; then
            printf '%s\n' 'Failed to remove the isolated RustFS project and its volume.' >&2
            result=1
        fi
    fi
    exit "$result"
}
trap cleanup EXIT

"${compose[@]}" config --quiet
created=1
"${compose[@]}" up --detach --pull always
wait_for_s3() {
PYTHONPATH="$repository_root/infra" python3 -B - "$endpoint" <<'PY'
import sys
import time
import floci_smoke

floci_smoke.validate_local_endpoint(sys.argv[1])
deadline = time.monotonic() + 90
while time.monotonic() < deadline:
    try:
        floci_smoke.request(sys.argv[1], "GET", "/")
        print("RustFS signed S3 endpoint is ready")
        break
    except (OSError, RuntimeError) as error:
        if isinstance(error, RuntimeError) and "HTTP 503" not in str(error):
            raise
        time.sleep(1)
else:
    raise SystemExit("RustFS S3 did not become ready")
PY
}
wait_for_s3
python3 "$repository_root/infra/floci_smoke.py" write "$endpoint" "$run_id"
export SQLITE_REPLICATION_FLOCI_ENDPOINT="$endpoint"
export SQLITE_REPLICATION_FLOCI_RUN_ID="$run_id"
export SQLITE_REPLICATION_FLOCI_FIXTURE="$repository_root/conformance/fixtures/ltx-v2/snapshot-4096.ltx"
export SQLITE_REPLICATION_FLOCI_CROSS_ENGINE=1
(
    cd "$go_repository_root"
    go test -v ./storages3 -run '^TestFlociAdapterConformance$' -count=1
    go test -v ./internal/recovery -run '^TestFlociS3RecoveryRestoreCompactionAndGC$' -count=1
    go test -v ./replicator -run '^TestFlociPublicS3Sync$' -count=1
    SQLITE_REPLICATION_FLOCI_RESTART_PHASE=write go test -v ./storages3 -run '^TestFlociRestartMarker$' -count=1
)
"${compose[@]}" kill --signal SIGKILL rustfs
"${compose[@]}" start rustfs
wait_for_s3
python3 "$repository_root/infra/floci_smoke.py" verify "$endpoint" "$run_id"
(
    cd "$repository_root/rust"
    cargo test -p sqlite-replication-storage-s3 tests::floci_adapter_conformance -- --ignored --exact
    cargo test -p sqlite-replication-core recovery::tests::floci_s3_recovery_restore_compaction_and_gc -- --ignored --exact
    cargo test -p sqlite-replication-core --test remote_cancellation cancellation_during_successful_remote_sync_drains_and_confirms -- --ignored --exact
    cargo test -p sqlite-replication-core --test flush_barrier barrier_captures_sqlite_and_confirms_remote_position -- --ignored --exact
)
(
    cd "$go_repository_root"
    go test -v ./internal/recovery -run '^TestFlociS3CrossEngineRestore$' -count=1
    go test -v ./replicator -run '^TestFlociPublicS3Restore$' -count=1
    SQLITE_REPLICATION_FLOCI_RESTART_PHASE=verify go test -v ./storages3 -run '^TestFlociRestartMarker$' -count=1
)
python3 "$repository_root/infra/floci_smoke.py" cleanup "$endpoint" "$run_id"
printf '%s\n' 'RustFS persistent S3 smoke test passed.'
