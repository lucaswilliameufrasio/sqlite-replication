package capture

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"os"
	"path/filepath"

	"github.com/lucaswilliameufrasio/sqlite-replication/internal/spool"
	"github.com/lucaswilliameufrasio/sqlite-replication/internal/sqliteadapter"
	"github.com/lucaswilliameufrasio/sqlite-replication/internal/wal"
	"github.com/lucaswilliameufrasio/sqlite-replication/internal/walcapture"
	"github.com/lucaswilliameufrasio/sqlite-replication/ltx"
)

type FaultPoint string

const (
	AfterLeaseAcquire FaultPoint = "after-lease-acquire"
	AfterWALRead      FaultPoint = "after-wal-read"
	AfterSnapshot     FaultPoint = "after-source-snapshot"
	AfterTranslation  FaultPoint = "after-wal-translation"
	AfterPublication  FaultPoint = "after-spool-publication"
)

type FaultHook func(point FaultPoint, transactionIndex int) error

type Result struct {
	Transactions []walcapture.Transaction
	State        walcapture.State
	WALFrames    int
}

// CaptureOnce pins SQLite, translates a stable WAL, reconciles against an
// Online Backup image, and durably publishes LTX before releasing either lock.
// It intentionally does not run a checkpoint; callers may only checkpoint
// after durable publication and after separately proving CP-01 for their setup.
func CaptureOnce(ctx context.Context, sourcePath, repositoryRoot string, timestampMillis int64, hook FaultHook) (result Result, returnErr error) {
	if err := ctx.Err(); err != nil {
		return Result{}, err
	}
	lease, err := sqliteadapter.BeginCaptureLease(ctx, sourcePath)
	if err != nil {
		return Result{}, err
	}
	defer func() {
		if err := lease.Close(); err != nil {
			returnErr = errors.Join(returnErr, fmt.Errorf("release SQLite capture lease: %w", err))
		}
	}()
	if err := invokeHook(hook, AfterLeaseAcquire, 0); err != nil {
		return Result{}, err
	}
	if err := ctx.Err(); err != nil {
		return Result{}, err
	}

	stableWAL, err := lease.ReadStableWAL(ctx)
	if err != nil {
		return Result{}, err
	}
	if err := ctx.Err(); err != nil {
		return Result{}, err
	}
	if err := validateCheckpointObservation(stableWAL); err != nil {
		return Result{}, err
	}
	if err := invokeHook(hook, AfterWALRead, 0); err != nil {
		return Result{}, err
	}
	if err := ctx.Err(); err != nil {
		return Result{}, err
	}

	state, err := spool.Recover(repositoryRoot, stableWAL.Bytes)
	if err != nil {
		return Result{}, fmt.Errorf("recover local LTX spool: %w", err)
	}

	stagingDirectory := filepath.Join(repositoryRoot, ".staging")
	if err := os.MkdirAll(stagingDirectory, 0o750); err != nil {
		return Result{}, fmt.Errorf("create source image staging directory: %w", err)
	}
	image, err := os.CreateTemp(stagingDirectory, ".capture-source-*.sqlite")
	if err != nil {
		return Result{}, fmt.Errorf("create source image staging file: %w", err)
	}
	imagePath := image.Name()
	if err := image.Close(); err != nil {
		_ = os.Remove(imagePath)
		return Result{}, fmt.Errorf("close source image staging file: %w", err)
	}
	defer os.Remove(imagePath)
	if err := sqliteadapter.CreateOnlineBackup(ctx, sourcePath, imagePath); err != nil {
		return Result{}, fmt.Errorf("create source image under capture lease: %w", err)
	}
	if err := ctx.Err(); err != nil {
		return Result{}, err
	}
	if err := invokeHook(hook, AfterSnapshot, 0); err != nil {
		return Result{}, err
	}
	if err := ctx.Err(); err != nil {
		return Result{}, err
	}
	imageFile, err := os.Open(imagePath)
	if err != nil {
		return Result{}, fmt.Errorf("open consistent source image: %w", err)
	}
	imageInfo, err := imageFile.Stat()
	if err != nil {
		_ = imageFile.Close()
		return Result{}, fmt.Errorf("stat consistent source image: %w", err)
	}
	targetState, err := ltx.LoadDatabaseState(imageFile, imageInfo.Size(), state.Database.TXID)
	if err != nil {
		_ = imageFile.Close()
		return Result{}, fmt.Errorf("calculate source snapshot checksum: %w", err)
	}

	transactions, nextState, err := walcapture.TranslateWAL(stableWAL.Bytes, state, timestampMillis)
	if err != nil {
		_ = imageFile.Close()
		return Result{}, fmt.Errorf("translate captured WAL: %w", err)
	}
	if nextState.Database.Checksum != targetState.Checksum || nextState.Database.Commit != targetState.Commit {
		header, pages, reconciledState, diffErr := state.Database.DeltaToDatabaseImage(imageFile, imageInfo.Size(), timestampMillis)
		if diffErr != nil {
			_ = imageFile.Close()
			return Result{}, fmt.Errorf("reconcile source image with staged LTX state: %w", diffErr)
		}
		transactions = nil
		if header != nil {
			var encoded bytes.Buffer
			if err := ltx.EncodeDelta(*header, pages, reconciledState.Checksum, &encoded); err != nil {
				_ = imageFile.Close()
				return Result{}, fmt.Errorf("encode source reconciliation delta: %w", err)
			}
			transactions = append(transactions, walcapture.Transaction{
				Header:       *header,
				Pages:        pages,
				PostChecksum: reconciledState.Checksum,
				Encoded:      encoded.Bytes(),
			})
		}
		position, err := walcapture.PositionFromWAL(stableWAL.Bytes)
		if err != nil {
			_ = imageFile.Close()
			return Result{}, fmt.Errorf("record reconciled WAL boundary: %w", err)
		}
		nextState = walcapture.State{Database: reconciledState, WAL: position}
	}
	if err := imageFile.Close(); err != nil {
		return Result{}, fmt.Errorf("close consistent source image: %w", err)
	}
	if err := invokeHook(hook, AfterTranslation, len(transactions)); err != nil {
		return Result{}, err
	}
	if err := ctx.Err(); err != nil {
		return Result{}, err
	}
	if err := spool.PublishTransactions(repositoryRoot, transactions, nextState, func(point spool.FaultPoint, index int) error {
		if point == spool.AfterWatermark {
			if hook == nil {
				return nil
			}
			return hook(FaultPoint(point), index)
		}
		if err := ctx.Err(); err != nil {
			return err
		}
		if hook != nil {
			if err := hook(FaultPoint(point), index); err != nil {
				return err
			}
		}
		return ctx.Err()
	}); err != nil {
		return Result{}, fmt.Errorf("publish capture spool: %w", err)
	}
	if err := invokeHook(hook, AfterPublication, len(transactions)); err != nil {
		return Result{}, err
	}
	return Result{
		Transactions: transactions,
		State:        nextState,
		WALFrames:    int(nextState.WAL.FrameCount),
	}, nil
}

func validateCheckpointObservation(stable sqliteadapter.StableWAL) error {
	if len(stable.Bytes) == 0 {
		if stable.Observation.LogFrames > 0 {
			return fmt.Errorf("SQLite reports %d WAL frames but returned no sidecar bytes", stable.Observation.LogFrames)
		}
		return nil
	}
	scan, err := wal.Scan(stable.Bytes)
	if err != nil {
		return fmt.Errorf("validate stable WAL bytes: %w", err)
	}
	if stable.Observation.LogFrames >= 0 && stable.Observation.LogFrames != len(scan.Frames) {
		return fmt.Errorf("SQLite NOOP reports %d WAL frames, file contains %d complete frames", stable.Observation.LogFrames, len(scan.Frames))
	}
	return nil
}

func invokeHook(hook FaultHook, point FaultPoint, transactionIndex int) error {
	if hook == nil {
		return nil
	}
	if err := hook(point, transactionIndex); err != nil {
		return fmt.Errorf("injected capture failure at %s: %w", point, err)
	}
	return nil
}
