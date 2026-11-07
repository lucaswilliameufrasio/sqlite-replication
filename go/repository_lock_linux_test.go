//go:build linux

package replication_test

import (
	"context"
	"errors"
	"path/filepath"
	"testing"

	"github.com/lucaswilliameufrasio/sqlite-replication"
	"github.com/lucaswilliameufrasio/sqlite-replication/internal/repositorylock"
	"github.com/lucaswilliameufrasio/sqlite-replication/storagefs"
	"golang.org/x/sys/unix"
)

func TestSnapshotAndRestoreRejectHeldRepositoryLock(t *testing.T) {
	root := t.TempDir()
	if err := storagefs.EnsureRepository(root); err != nil {
		t.Fatal(err)
	}
	lock, err := repositorylock.AcquireExclusive(root)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = lock.Close() })
	if _, err := replication.Snapshot(context.Background(), filepath.Join(root, "source.sqlite"), root); !errors.Is(err, unix.EWOULDBLOCK) {
		t.Fatalf("snapshot with held lock = %v, want contention", err)
	}
	if err := replication.RestoreSnapshot(context.Background(), root, filepath.Join(t.TempDir(), "restored.sqlite")); !errors.Is(err, unix.EWOULDBLOCK) {
		t.Fatalf("snapshot restore with held lock = %v, want contention", err)
	}
}
