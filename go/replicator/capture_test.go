package replicator_test

import (
	"context"
	"database/sql"
	"errors"
	"path/filepath"
	"runtime"
	"testing"

	"github.com/lucaswilliameufrasio/sqlite-replication"
	"github.com/lucaswilliameufrasio/sqlite-replication/internal/spool"
	"github.com/lucaswilliameufrasio/sqlite-replication/replicator"
	_ "modernc.org/sqlite"
)

func TestCaptureOnceRequiresStrictWriteGateAndSpoolsBeforeRelease(t *testing.T) {
	ctx := context.Background()
	dir := t.TempDir()
	databasePath := filepath.Join(dir, "source.db")
	createSourceDatabase(t, databasePath)
	repositoryRoot := filepath.Join(dir, "repository")
	if _, err := replication.Snapshot(ctx, databasePath, repositoryRoot); err != nil {
		t.Fatalf("create initial snapshot: %v", err)
	}

	if _, err := replicator.CaptureOnce(ctx, databasePath, repositoryRoot, nil); err == nil {
		t.Fatal("CaptureOnce accepted a missing strict write gate")
	}
	if runtime.GOOS != "linux" || runtime.GOARCH != "amd64" {
		if _, err := replicator.CaptureOnce(ctx, databasePath, repositoryRoot, func(context.Context) (func() error, error) {
			t.Fatal("unsupported platform acquired the write gate")
			return nil, nil
		}); err == nil {
			t.Fatal("CaptureOnce accepted an unsupported platform")
		}
		return
	}

	db, err := sql.Open("sqlite", databasePath)
	if err != nil {
		t.Fatal(err)
	}
	db.SetMaxOpenConns(1)
	if _, err := db.ExecContext(ctx, "PRAGMA wal_autocheckpoint=0"); err != nil {
		t.Fatalf("disable test autocheckpoint: %v", err)
	}
	if _, err := db.ExecContext(ctx, "INSERT INTO records(value) VALUES (?)", "captured"); err != nil {
		t.Fatalf("commit source change: %v", err)
	}
	if err := db.Close(); err != nil {
		t.Fatal(err)
	}

	gateAcquired := false
	gateReleased := false
	result, err := replicator.CaptureOnce(ctx, databasePath, repositoryRoot, func(context.Context) (func() error, error) {
		gateAcquired = true
		return func() error {
			watermark, err := spool.ReadWatermark(repositoryRoot)
			if err != nil {
				return err
			}
			if watermark.TXID != 2 {
				t.Errorf("write gate released at TXID %d, want 2", watermark.TXID)
			}
			gateReleased = true
			return nil
		}, nil
	})
	if err != nil {
		t.Fatalf("capture local WAL: %v", err)
	}
	if !gateAcquired || !gateReleased {
		t.Fatalf("write gate lifecycle: acquired=%t released=%t", gateAcquired, gateReleased)
	}
	if result.TXID != 2 || result.Transactions == 0 {
		t.Fatalf("capture result = %+v, want durable TXID 2 and a transaction", result)
	}
}

func TestCaptureOnceReleasesWriteGateWhenCaptureFails(t *testing.T) {
	if runtime.GOOS != "linux" || runtime.GOARCH != "amd64" {
		t.Skip("capture execution is intentionally limited to Linux amd64")
	}
	dir := t.TempDir()
	databasePath := filepath.Join(dir, "missing.db")
	repositoryRoot := filepath.Join(dir, "missing-repository")
	gateReleased := false
	_, err := replicator.CaptureOnce(context.Background(), databasePath, repositoryRoot, func(context.Context) (func() error, error) {
		return func() error {
			gateReleased = true
			return nil
		}, nil
	})
	if err == nil {
		t.Fatal("capture succeeded without an initialized repository")
	}
	if !gateReleased {
		t.Fatal("write gate was not released after capture failure")
	}
}

func TestCaptureOnceCombinesCaptureAndGateReleaseErrors(t *testing.T) {
	if runtime.GOOS != "linux" || runtime.GOARCH != "amd64" {
		t.Skip("capture execution is intentionally limited to Linux amd64")
	}
	dir := t.TempDir()
	databasePath := filepath.Join(dir, "missing.db")
	repositoryRoot := filepath.Join(dir, "missing-repository")
	gateErr := errors.New("gate release failed")
	_, err := replicator.CaptureOnce(context.Background(), databasePath, repositoryRoot, func(context.Context) (func() error, error) {
		return func() error {
			return gateErr
		}, nil
	})
	if !errors.Is(err, gateErr) {
		t.Fatalf("CaptureOnce error = %v, want gate release error", err)
	}
}

func TestCaptureOnceReportsGateReleaseErrorAfterDurableCapture(t *testing.T) {
	if runtime.GOOS != "linux" || runtime.GOARCH != "amd64" {
		t.Skip("capture execution is intentionally limited to Linux amd64")
	}
	ctx := context.Background()
	dir := t.TempDir()
	databasePath := filepath.Join(dir, "source.db")
	createSourceDatabase(t, databasePath)
	repositoryRoot := filepath.Join(dir, "repository")
	if _, err := replication.Snapshot(ctx, databasePath, repositoryRoot); err != nil {
		t.Fatalf("create initial snapshot: %v", err)
	}
	db, err := sql.Open("sqlite", databasePath)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := db.ExecContext(ctx, "INSERT INTO records(value) VALUES (?)", "captured"); err != nil {
		t.Fatalf("commit source change: %v", err)
	}
	if err := db.Close(); err != nil {
		t.Fatal(err)
	}

	releaseErr := errors.New("release failed after local durability")
	_, err = replicator.CaptureOnce(ctx, databasePath, repositoryRoot, func(context.Context) (func() error, error) {
		return func() error {
			return releaseErr
		}, nil
	})
	if !errors.Is(err, releaseErr) {
		t.Fatalf("CaptureOnce error = %v, want release error", err)
	}
	watermark, err := spool.ReadWatermark(repositoryRoot)
	if err != nil {
		t.Fatalf("read durable watermark after gate release error: %v", err)
	}
	if watermark.TXID != 2 {
		t.Fatalf("durable watermark TXID = %d, want 2", watermark.TXID)
	}
}

func createSourceDatabase(t *testing.T, path string) {
	t.Helper()
	db, err := sql.Open("sqlite", path)
	if err != nil {
		t.Fatal(err)
	}
	db.SetMaxOpenConns(1)
	for _, statement := range []string{
		"PRAGMA journal_mode=WAL",
		"PRAGMA synchronous=FULL",
		"CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT NOT NULL)",
	} {
		if _, err := db.Exec(statement); err != nil {
			t.Fatalf("execute %q: %v", statement, err)
		}
	}
	if _, err := db.Exec("INSERT INTO records(value) VALUES (?)", "base"); err != nil {
		t.Fatal(err)
	}
	if err := db.Close(); err != nil {
		t.Fatal(err)
	}
}
