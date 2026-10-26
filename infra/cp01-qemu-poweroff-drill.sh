#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
temporary_root="/tmp/opencode"
kernel="/usr/lib/modules/7.3.0-5-cachyos/vmlinuz"
busybox="/usr/lib/initcpio/busybox"

if [[ "$(uname -s)" != "Linux" ]]; then
    printf '%s\n' 'The CP-01 QEMU power-off drill requires Linux.' >&2
    exit 1
fi
if [[ ! -c /dev/kvm || ! -r /dev/kvm || ! -w /dev/kvm ]]; then
    printf '%s\n' 'The CP-01 QEMU power-off drill requires accessible /dev/kvm.' >&2
    exit 1
fi
for tool in qemu-system-x86_64 qemu-img mkfs.ext4 cpio gzip go cargo python3 ldd mount; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'Required CP-01 QEMU drill tool is missing: %s\n' "$tool" >&2
        exit 1
    fi
done
if [[ ! -r "$kernel" || ! -r "$busybox" || ! -d "$temporary_root" ]]; then
    printf '%s\n' 'The host kernel, initramfs BusyBox, or approved temporary directory is unavailable.' >&2
    exit 1
fi

work_directory="$(mktemp -d "$temporary_root/sqlite-replication-cp01-qemu.XXXXXX")"
case "$work_directory" in
    "$temporary_root"/sqlite-replication-cp01-qemu.*) ;;
    *)
        printf 'Refusing unexpected work directory: %s\n' "$work_directory" >&2
        exit 1
        ;;
esac
guest_root="$work_directory/guest"
qemu_pid=""
mkdir -p "$guest_root/bin" "$guest_root/dev" "$guest_root/proc" "$guest_root/sys" "$guest_root/mnt/disk"

cleanup() {
    result=$?
    trap - EXIT
    if [[ -n "$qemu_pid" ]] && kill -0 "$qemu_pid" 2>/dev/null; then
        kill -KILL "$qemu_pid" 2>/dev/null || true
        wait "$qemu_pid" 2>/dev/null || true
    fi
    case "$work_directory" in
        "$temporary_root"/sqlite-replication-cp01-qemu.*)
            rm -rf -- "$work_directory"
            ;;
        *)
            printf 'Refusing to remove unexpected work directory: %s\n' "$work_directory" >&2
            if [[ "$result" -eq 0 ]]; then
                result=1
            fi
            ;;
    esac
    exit "$result"
}
trap cleanup EXIT

go_binary="$work_directory/go-spool.test"
(
    cd "$repository_root/go"
    go test -c -o "$go_binary" ./internal/spool
)

rust_target="$work_directory/rust-target"
cargo test --manifest-path "$repository_root/rust/Cargo.toml" \
    --target-dir "$rust_target" -p sqlite-replication-spool --lib --no-run \
    --message-format=json >"$work_directory/cargo-artifacts.jsonl"
rust_binary="$(python3 - "$work_directory/cargo-artifacts.jsonl" <<'PY'
import json
import sys

matches = []
with open(sys.argv[1], encoding="utf-8") as stream:
    for line in stream:
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if message.get("reason") != "compiler-artifact":
            continue
        target = message.get("target", {})
        profile = message.get("profile", {})
        package_id = message.get("package_id", "")
        if (
            target.get("name") == "sqlite_replication_spool"
            and package_id.startswith("path+")
            and profile.get("test") is True
        ):
            executable = message.get("executable")
            if executable:
                matches.append(executable)

if len(matches) != 1:
    raise SystemExit(f"expected exactly one Rust spool test executable, found {len(matches)}")
print(matches[0])
PY
)"

install -m 0755 "$busybox" "$guest_root/bin/busybox"
ln -s busybox "$guest_root/bin/sh"
while IFS= read -r applet; do
    if [[ "$applet" != sh ]]; then
        ln -s busybox "$guest_root/bin/$applet"
    fi
done < <("$busybox" --list)
install -m 0755 "$go_binary" "$guest_root/bin/go-spool.test"
install -m 0755 "$rust_binary" "$guest_root/bin/rust-spool.test"
install -m 0755 "$(command -v mount)" "$guest_root/bin/mount"
for executable in "$guest_root/bin/busybox" "$guest_root/bin/mount" "$guest_root/bin/go-spool.test" "$guest_root/bin/rust-spool.test"; do
    while IFS= read -r library; do
        if [[ -n "$library" && -f "$library" ]]; then
            install -D -m 0755 "$library" "$guest_root$library"
        fi
    done < <(ldd "$executable" 2>/dev/null | awk '/=> \/|^[[:space:]]*\// { for (i = 1; i <= NF; i++) if ($i ~ /^\//) { print $i; break } }')
done

cat > "$guest_root/init" <<'GUEST_INIT'
#!/bin/sh
PATH=/bin
export PATH
mount -t devtmpfs devtmpfs /dev
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t ext4 -o noatime /dev/vda /mnt/disk || exec sh

mode=""
for argument in $(cat /proc/cmdline); do
    case "$argument" in
        cp01.mode=*)
            mode="${argument#cp01.mode=}"
            ;;
    esac
done
if [ "$mode" != prepare ] && [ "$mode" != recover ]; then
    echo "Invalid CP-01 QEMU mode: $mode" >&2
    exec sh
fi
export SQLITE_REPLICATION_CP01_QEMU_MODE="$mode"
export SQLITE_REPLICATION_CP01_QEMU_ROOT=/mnt/disk/go-repository
/bin/go-spool.test -test.run '^TestRecoverAfterAbruptQEMUExit$' -test.v || exec sh
export SQLITE_REPLICATION_CP01_QEMU_ROOT=/mnt/disk/rust-repository
/bin/rust-spool.test --exact tests::recover_after_abrupt_qemu_exit --ignored --nocapture || exec sh
sync

if [ "$mode" = prepare ]; then
    echo CP01_PREPARED_FOR_ABRUPT_QEMU_EXIT
    while :; do
        sleep 1
    done
fi

echo CP01_RECOVERED_AFTER_ABRUPT_QEMU_EXIT
sync
poweroff -f
exec sh
GUEST_INIT
chmod 0755 "$guest_root/init"

(
    cd "$guest_root"
    find . -print0 | cpio --null -o --format=newc 2>/dev/null | gzip -1 > "$work_directory/initramfs.cpio.gz"
)

disk_image="$work_directory/cp01-ext4.raw"
truncate -s 256M "$disk_image"
mkfs.ext4 -F -q -L CP01QEMU "$disk_image"

boot_guest() {
    local mode="$1"
    local serial_log="$2"
    qemu-system-x86_64 \
        -machine q35,accel=kvm \
        -cpu host -m 512 -smp 2 \
        -kernel "$kernel" \
        -initrd "$work_directory/initramfs.cpio.gz" \
        -append "console=ttyS0 rdinit=/init cp01.mode=$mode panic=-1" \
        -drive "file=$disk_image,if=virtio,format=raw,cache=writeback" \
        -display none -serial "file:$serial_log" -monitor none -no-reboot
}

prepare_log="$work_directory/prepare.serial"
boot_guest prepare "$prepare_log" >"$work_directory/prepare.qemu.log" 2>&1 &
qemu_pid=$!
prepare_deadline=$((SECONDS + 120))
while ! grep -q 'CP01_PREPARED_FOR_ABRUPT_QEMU_EXIT' "$prepare_log" 2>/dev/null; do
    if ! kill -0 "$qemu_pid" 2>/dev/null; then
        cat "$prepare_log" >&2 || true
        cat "$work_directory/prepare.qemu.log" >&2 || true
        printf '%s\n' 'QEMU guest exited before publishing both durable captures.' >&2
        exit 1
    fi
    if (( SECONDS >= prepare_deadline )); then
        printf '%s\n' 'Timed out waiting for the guest to publish both captures.' >&2
        cat "$prepare_log" >&2 || true
        cat "$work_directory/prepare.qemu.log" >&2 || true
        exit 1
    fi
    sleep 0.1
done

kill -KILL "$qemu_pid"
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""
printf '%s\n' 'QEMU process was killed abruptly after both guests synced and validated their captures.'

recovery_log="$work_directory/recovery.serial"
boot_guest recover "$recovery_log" >"$work_directory/recovery.qemu.log" 2>&1 || recovery_status=$?
recovery_status="${recovery_status:-0}"
if [[ "$recovery_status" -ne 0 ]] || ! grep -q 'CP01_RECOVERED_AFTER_ABRUPT_QEMU_EXIT' "$recovery_log" 2>/dev/null; then
    cat "$recovery_log" >&2 || true
    cat "$work_directory/recovery.qemu.log" >&2 || true
    printf 'QEMU recovery guest failed (status %s).\n' "$recovery_status" >&2
    exit 1
fi
printf '%s\n' 'CP-01 QEMU abrupt-exit drill passed for Go and Rust recovery on ext4 over virtio-blk.'
printf '%s\n' 'Scope: QEMU/KVM + virtual disk + guest kernel/filesystem only; not physical power loss or hardware-cache loss.'
