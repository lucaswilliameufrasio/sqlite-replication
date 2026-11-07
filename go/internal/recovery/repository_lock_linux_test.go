//go:build linux

package recovery

import (
	"context"
	"errors"
	"path/filepath"
	"testing"

	"github.com/lucaswilliameufrasio/sqlite-replication/internal/repositorylock"
	"golang.org/x/sys/unix"
)

func TestGCRejectsHeldRepositoryLock(t *testing.T) {
	root := t.TempDir()
	lock, err := repositorylock.AcquireExclusive(root)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = lock.Close() })
	err = ApplyGCPlan(root, GCPlan{RequiredTargets: []Target{{TXID: 1}}})
	if !errors.Is(err, unix.EWOULDBLOCK) {
		t.Fatalf("GC with held lock = %v, want contention", err)
	}
}

func TestRestoreAndCompactionRejectHeldRepositoryLock(t *testing.T) {
	root := t.TempDir()
	lock, err := repositorylock.AcquireExclusive(root)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = lock.Close() })
	if err := RestoreToTarget(context.Background(), root, filepath.Join(t.TempDir(), "restore.sqlite"), Target{TXID: 1}); !errors.Is(err, unix.EWOULDBLOCK) {
		t.Fatalf("restore with held lock = %v, want contention", err)
	}
	if _, err := CompactToSnapshot(root, Target{TXID: 1}); !errors.Is(err, unix.EWOULDBLOCK) {
		t.Fatalf("compaction with held lock = %v, want contention", err)
	}
}
