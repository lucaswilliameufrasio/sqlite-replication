package replicator

import (
	"context"

	"github.com/lucaswilliameufrasio/sqlite-replication/internal/recovery"
)

// RestoreTarget selects exactly one verified recovery point. Set TXID for an
// exact transaction, or TimestampMillis for the latest point at or before that
// UTC Unix timestamp in milliseconds.
type RestoreTarget struct {
	TXID            uint64
	TimestampMillis *int64
}

// RestoreToTarget rebuilds and verifies a snapshot-plus-delta chain into a new
// database path. It never overwrites a database or stale SQLite sidecar.
func RestoreToTarget(ctx context.Context, repositoryRoot, destinationPath string, target RestoreTarget) error {
	return recovery.RestoreToTarget(ctx, repositoryRoot, destinationPath, recovery.Target{
		TXID:            target.TXID,
		TimestampMillis: target.TimestampMillis,
	})
}
