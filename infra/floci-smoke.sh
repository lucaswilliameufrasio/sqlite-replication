#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
compose_file="$repository_root/infra/compose.yaml"
run_id="$(python3 -c 'import uuid; print(uuid.uuid4().hex)')"
project_name="sqlite-replication-floci-${run_id}"
host_port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')"
export FLOCI_PORT="$host_port"

docker_command=(docker)
contexts="$(docker context ls --format '{{.Name}}')"
if grep -Fxq 'rootless' <<< "$contexts"; then
    security_options="$(docker --context rootless info --format '{{json .SecurityOptions}}')"
    if [[ "$security_options" != *name=rootless* ]]; then
        printf '%s\n' 'The configured rootless Docker context is not connected to a verified rootless daemon.' >&2
        exit 1
    fi
    docker_command=(docker --context rootless)
fi

compose=("${docker_command[@]}" compose --project-name "$project_name" --file "$compose_file")
endpoint="http://127.0.0.1:${host_port}"
compose_resources_created=0

cleanup() {
    result=$?
    trap - EXIT
    if [[ "$compose_resources_created" -eq 1 ]]; then
        if ! "${compose[@]}" down --volumes --remove-orphans; then
            printf '%s\n' 'Failed to remove the isolated Floci smoke-test project and its volume.' >&2
            if [[ "$result" -eq 0 ]]; then
                result=1
            fi
        fi
    fi
    exit "$result"
}
trap cleanup EXIT

"${compose[@]}" config --quiet
compose_resources_created=1
"${compose[@]}" up --detach --pull always --wait --wait-timeout 90
python3 "$repository_root/infra/floci_smoke.py" wait "$endpoint" "$run_id"
python3 "$repository_root/infra/floci_smoke.py" write "$endpoint" "$run_id"
export SQLITE_REPLICATION_FLOCI_ENDPOINT="$endpoint"
export SQLITE_REPLICATION_FLOCI_RUN_ID="$run_id"
export SQLITE_REPLICATION_FLOCI_FIXTURE="$repository_root/conformance/fixtures/ltx-v2/snapshot-4096.ltx"
(
    cd "$repository_root/go"
    go test ./storages3 -run '^TestFlociAdapterConformance$' -count=1
    SQLITE_REPLICATION_FLOCI_RESTART_PHASE=write go test ./storages3 -run '^TestFlociRestartMarker$' -count=1
)

"${compose[@]}" kill --signal SIGKILL floci
"${compose[@]}" start floci
python3 "$repository_root/infra/floci_smoke.py" wait "$endpoint" "$run_id"
(
    cd "$repository_root/rust"
    cargo test -p sqlite-replication-storage-s3 tests::floci_adapter_conformance -- --ignored --exact
)
(
    cd "$repository_root/go"
    SQLITE_REPLICATION_FLOCI_RESTART_PHASE=verify go test ./storages3 -run '^TestFlociRestartMarker$' -count=1
)
python3 "$repository_root/infra/floci_smoke.py" verify "$endpoint" "$run_id"
python3 "$repository_root/infra/floci_smoke.py" cleanup "$endpoint" "$run_id"

printf '%s\n' 'Floci persistent S3 smoke test passed; isolated project and named volume will be removed.'
