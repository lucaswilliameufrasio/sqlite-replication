// Package replicator exposes synchronous incremental capture operations.
package replicator

import (
	"context"
	"errors"
	"fmt"
	"runtime"
	"time"

	"github.com/lucaswilliameufrasio/sqlite-replication/internal/capture"
)

// WriteGate pauses every application writer coordinated by the caller. The
// returned release function must restore writes and may report a release error.
// A nil gate is rejected: strict admission is mandatory for incremental capture.
type WriteGate func(context.Context) (release func() error, err error)

// CaptureResult reports the durable local position produced by CaptureOnce.
type CaptureResult struct {
	TXID         uint64
	Transactions int
	WALFrames    int
}

// CaptureOnce captures and durably spools the currently observable SQLite WAL
// and reconciliation delta, then releases the application's write gate.
//
// The repository must already contain an initial snapshot created by
// replication.Snapshot. The caller must route all application writes through
// gate and keep them paused until the returned release function runs. This API
// is supported only on Linux amd64 with the SQLite runtimes validated by CP-01.
// It never requests a checkpoint and does not upload the local spool remotely.
func CaptureOnce(ctx context.Context, databasePath, repositoryRoot string, gate WriteGate) (CaptureResult, error) {
	if runtime.GOOS != "linux" || runtime.GOARCH != "amd64" {
		return CaptureResult{}, fmt.Errorf("incremental capture is supported only on Linux amd64")
	}
	if gate == nil {
		return CaptureResult{}, fmt.Errorf("strict incremental capture requires an application write gate")
	}
	if err := ctx.Err(); err != nil {
		return CaptureResult{}, err
	}
	release, err := gate(ctx)
	if err != nil {
		return CaptureResult{}, fmt.Errorf("acquire application write gate: %w", err)
	}
	if release == nil {
		return CaptureResult{}, fmt.Errorf("application write gate returned no release function")
	}

	result, captureErr := capture.CaptureOnce(ctx, databasePath, repositoryRoot, time.Now().UnixMilli(), nil)
	releaseErr := release()
	if captureErr != nil || releaseErr != nil {
		return CaptureResult{}, errors.Join(captureErr, wrapReleaseError(releaseErr))
	}
	return CaptureResult{
		TXID:         result.State.Database.TXID,
		Transactions: len(result.Transactions),
		WALFrames:    result.WALFrames,
	}, nil
}

func wrapReleaseError(err error) error {
	if err == nil {
		return nil
	}
	return fmt.Errorf("release application write gate: %w", err)
}
