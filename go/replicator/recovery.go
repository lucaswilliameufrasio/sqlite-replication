package replicator

import (
	"context"

	"github.com/lucaswilliameufrasio/sqlite-replication/internal/recovery"
	"github.com/lucaswilliameufrasio/sqlite-replication/storages3"
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

// RestoreFromS3 stages and verifies an S3-compatible repository, then restores
// the requested point without overwriting an existing database. Provider
// compatibility remains the caller's responsibility; Floci is the only tested
// implementation and remote restore requires local temporary space proportional
// to the repository size.
func RestoreFromS3(ctx context.Context, store *storages3.Store, prefix, destinationPath string, target RestoreTarget) error {
	return recovery.RestoreS3ToTarget(ctx, store, prefix, destinationPath, recovery.Target{
		TXID:            target.TXID,
		TimestampMillis: target.TimestampMillis,
	})
}
