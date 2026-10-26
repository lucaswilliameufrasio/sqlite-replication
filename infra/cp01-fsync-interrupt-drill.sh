#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scratch_root="/tmp/opencode"

if [[ "$(uname -s)" != "Linux" ]]; then
    printf '%s\n' 'The CP-01 in-flight fsync drill requires Linux and FUSE.' >&2
    exit 1
fi
if [[ ! -c /dev/fuse || ! -r /dev/fuse || ! -w /dev/fuse ]]; then
    printf '%s\n' 'The CP-01 in-flight fsync drill requires readable and writable /dev/fuse.' >&2
    exit 1
fi
for tool in cc pkg-config mountpoint fusermount3; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'Required CP-01 drill tool is missing: %s\n' "$tool" >&2
        exit 1
    fi
done
if ! pkg-config --exists fuse3; then
    printf '%s\n' 'The CP-01 in-flight fsync drill requires the fuse3 development package.' >&2
    exit 1
fi
if [[ ! -d "$scratch_root" ]]; then
    printf 'Required temporary directory does not exist: %s\n' "$scratch_root" >&2
    exit 1
fi

temporary_directory="$(mktemp -d "$scratch_root/sqlite-replication-cp01-fsync.XXXXXX")"
mount_directory="$temporary_directory/mount"
backing_directory="$temporary_directory/backing"
barrier_path="$temporary_directory/fsync-entered"
release_path="$temporary_directory/fsync-release"
completed_path="$temporary_directory/fsync-completed"
daemon_pid=""
mkdir -p "$mount_directory" "$backing_directory"

cleanup() {
    result=$?
    trap - EXIT
    if [[ -n "$daemon_pid" ]]; then
        : > "$release_path"
        if mountpoint -q "$mount_directory"; then
            fusermount3 -u "$mount_directory" || true
        fi
        kill "$daemon_pid" 2>/dev/null || true
        wait "$daemon_pid" 2>/dev/null || true
    fi
    case "$temporary_directory" in
        "$scratch_root"/sqlite-replication-cp01-fsync.*)
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

cc -std=c11 -Wall -Wextra -Werror -O2 \
    $(pkg-config --cflags fuse3) \
    "$repository_root/infra/cp01-fsync-fuse.c" \
    -o "$temporary_directory/cp01-fsync-fuse" \
    $(pkg-config --libs fuse3)

CP01_FUSE_BACKING="$backing_directory" \
CP01_FUSE_BARRIER="$barrier_path" \
CP01_FUSE_RELEASE="$release_path" \
CP01_FUSE_COMPLETED="$completed_path" \
    "$temporary_directory/cp01-fsync-fuse" -f -s \
    -o default_permissions,fsname=cp01-fsync \
    "$mount_directory" >"$temporary_directory/fuse.log" 2>&1 &
daemon_pid=$!

mount_deadline=$((SECONDS + 10))
while ! mountpoint -q "$mount_directory"; do
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
        cat "$temporary_directory/fuse.log" >&2
        printf '%s\n' 'FUSE daemon exited before mounting.' >&2
        exit 1
    fi
    if (( SECONDS >= mount_deadline )); then
        cat "$temporary_directory/fuse.log" >&2
        printf '%s\n' 'Timed out waiting for the FUSE mount.' >&2
        exit 1
    fi
    sleep 0.1
done

(
    cd "$repository_root/go"
    TMPDIR="$temporary_directory" \
    SQLITE_REPLICATION_CP01_FUSE_ROOT="$mount_directory/repository" \
    CP01_FUSE_BARRIER="$barrier_path" \
    CP01_FUSE_RELEASE="$release_path" \
    CP01_FUSE_COMPLETED="$completed_path" \
        go test ./internal/spool -run '^TestPublishInterruptedDuringSync$' -count=1 -v
)

rm -f -- "$barrier_path" "$release_path" "$completed_path"
(
    cd "$repository_root"
    TMPDIR="$temporary_directory" \
    SQLITE_REPLICATION_CP01_FUSE_ROOT="$mount_directory/rust-repository" \
    CP01_FUSE_BARRIER="$barrier_path" \
    CP01_FUSE_RELEASE="$release_path" \
    CP01_FUSE_COMPLETED="$completed_path" \
        cargo test --manifest-path rust/Cargo.toml -p sqlite-replication-spool \
            --lib -- --ignored --exact tests::publish_interrupted_during_sync --nocapture
)

if [[ ! -e "$completed_path" ]]; then
    printf '%s\n' 'FUSE fsync callback did not report completion.' >&2
    exit 1
fi
printf '%s\n' 'CP-01 FUSE drill passed for Go and Rust: each publishing process was killed while its fsync syscall was blocked in the FUSE callback, and recovery/retry succeeded.'
