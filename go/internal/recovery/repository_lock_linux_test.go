//go:build linux

package recovery

import (
	"errors"
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
