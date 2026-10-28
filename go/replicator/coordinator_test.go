package replicator_test

import (
	"context"
	"database/sql"
	"errors"
	"path/filepath"
	"reflect"
	"sync/atomic"
	"testing"
	"time"

	_ "modernc.org/sqlite"

	replication "github.com/lucaswilliameufrasio/sqlite-replication"
	"github.com/lucaswilliameufrasio/sqlite-replication/replicator"
)

func TestCoordinatorStatusSeparatesObservedAndDurablePositions(t *testing.T) {
	ctx := context.Background()
	directory := t.TempDir()
	databasePath := filepath.Join(directory, "source.sqlite")
	createSourceDatabase(t, databasePath)
	repositoryRoot := filepath.Join(directory, "repository")
	if _, err := replication.Snapshot(ctx, databasePath, repositoryRoot); err != nil {
		t.Fatalf("create initial snapshot: %v", err)
	}
	coordinator, err := replicator.NewCoordinator(coordinatorConfig(repositoryRoot, databasePath, 2))
	if err != nil {
		t.Fatalf("construct coordinator: %v", err)
	}
	status, err := coordinator.Status(ctx)
	if err != nil {
		t.Fatalf("read status: %v", err)
	}
	if status.ObservedTXID == nil || *status.ObservedTXID != 2 {
		t.Fatalf("observed position = %v, want 2", status.ObservedTXID)
	}
	if status.LocalDurable == nil || status.LocalDurable.TXID != 1 {
		t.Fatalf("local durable position = %+v, want TXID 1", status.LocalDurable)
	}
	if status.RemoteConfirmed != nil || status.RemoteAckMatches {
		t.Fatalf("local-only coordinator reported remote confirmation: %+v", status)
	}
	if status.UnprotectedAge < 0 {
		t.Fatalf("unprotected age is negative: %s", status.UnprotectedAge)
	}
	health := coordinator.Health(ctx)
	if health.Local != replicator.ComponentHealthy || health.Remote != replicator.ComponentUnknown {
		t.Fatalf("component health = %+v", health)
	}
}

func TestCoordinatorStatusTreatsUnavailableObservationConservatively(t *testing.T) {
	ctx := context.Background()
	directory := t.TempDir()
	databasePath := filepath.Join(directory, "source.sqlite")
	createSourceDatabase(t, databasePath)
	repositoryRoot := filepath.Join(directory, "repository")
	if _, err := replication.Snapshot(ctx, databasePath, repositoryRoot); err != nil {
		t.Fatalf("create initial snapshot: %v", err)
	}
	config := coordinatorConfig(repositoryRoot, databasePath, 1)
	config.ObserveTXID = func(context.Context) (uint64, bool, error) {
		return 0, false, nil
	}
	coordinator, err := replicator.NewCoordinator(config)
	if err != nil {
		t.Fatalf("construct coordinator: %v", err)
	}
	status, err := coordinator.Status(ctx)
	if err != nil {
		t.Fatalf("read status: %v", err)
	}
	if status.ObservedTXID != nil {
		t.Fatalf("unavailable observed TXID was represented as %d", *status.ObservedTXID)
	}
	if status.UnprotectedAge < config.Limits.MaxUnprotectedAge {
		t.Fatalf("unknown observed position has insufficiently conservative age: %s", status.UnprotectedAge)
	}
}

func TestCoordinatorFlushSerializesAdmissionAroundTarget(t *testing.T) {
	ctx := context.Background()
	directory := t.TempDir()
	databasePath := filepath.Join(directory, "source.sqlite")
	createSourceDatabase(t, databasePath)
	repositoryRoot := filepath.Join(directory, "repository")
	if _, err := replication.Snapshot(ctx, databasePath, repositoryRoot); err != nil {
		t.Fatalf("create initial snapshot: %v", err)
	}
	var admission []bool
	var appliedLimits []replicator.Limits
	config := coordinatorConfig(repositoryRoot, databasePath, 1)
	config.SetAdmission = func(_ context.Context, enabled bool, limits replicator.Limits) error {
		admission = append(admission, enabled)
		appliedLimits = append(appliedLimits, limits)
		return nil
	}
	coordinator, err := replicator.NewCoordinator(config)
	if err != nil {
		t.Fatalf("construct coordinator: %v", err)
	}
	if err := coordinator.Flush(ctx); err != nil {
		t.Fatalf("flush already-durable target: %v", err)
	}
	if !reflect.DeepEqual(admission, []bool{false, true}) {
		t.Fatalf("admission transitions = %v, want [false true]", admission)
	}
	if len(appliedLimits) != 2 || appliedLimits[0] != config.Limits || appliedLimits[1] != config.Limits {
		t.Fatalf("admission limits = %+v, want configured limits %+v", appliedLimits, config.Limits)
	}
}

func TestCoordinatorSerializesConcurrentFlushOperations(t *testing.T) {
	ctx := context.Background()
	directory := t.TempDir()
	databasePath := filepath.Join(directory, "source.sqlite")
	createSourceDatabase(t, databasePath)
	repositoryRoot := filepath.Join(directory, "repository")
	if _, err := replication.Snapshot(ctx, databasePath, repositoryRoot); err != nil {
		t.Fatalf("create initial snapshot: %v", err)
	}
	database, err := sql.Open("sqlite", databasePath)
	if err != nil {
		t.Fatalf("open source database: %v", err)
	}
	if _, err := database.ExecContext(ctx, "UPDATE records SET value = ? WHERE id = 1", "flush-target"); err != nil {
		t.Fatalf("commit Flush target: %v", err)
	}
	if err := database.Close(); err != nil {
		t.Fatalf("close source database: %v", err)
	}
	captureEntered := make(chan struct{}, 2)
	releaseCapture := make(chan struct{})
	var admission []bool
	config := coordinatorConfig(repositoryRoot, databasePath, 2)
	config.SetAdmission = func(_ context.Context, enabled bool, _ replicator.Limits) error {
		admission = append(admission, enabled)
		return nil
	}
	config.Capture = func(context.Context) (replicator.CaptureResult, error) {
		captureEntered <- struct{}{}
		select {
		case <-releaseCapture:
		case <-time.After(2 * time.Second):
			return replicator.CaptureResult{}, errors.New("test capture release timed out")
		}
		result, err := replicator.CaptureOnce(ctx, databasePath, repositoryRoot, func(context.Context) (func() error, error) {
			return func() error { return nil }, nil
		})
		return result, err
	}
	coordinator, err := replicator.NewCoordinator(config)
	if err != nil {
		t.Fatalf("construct coordinator: %v", err)
	}
	firstResult := make(chan error, 1)
	go func() {
		firstResult <- coordinator.Flush(ctx)
	}()
	select {
	case <-captureEntered:
	case <-time.After(time.Second):
		t.Fatal("first Flush did not start capture")
	}
	secondResult := make(chan error, 1)
	go func() {
		secondResult <- coordinator.Flush(ctx)
	}()
	select {
	case <-captureEntered:
		t.Fatal("second Flush overlapped the first capture")
	case <-time.After(50 * time.Millisecond):
	}
	close(releaseCapture)
	if err := <-firstResult; err != nil {
		t.Fatalf("first Flush: %v", err)
	}
	if err := <-secondResult; err != nil {
		t.Fatalf("second Flush: %v", err)
	}
	if !reflect.DeepEqual(admission, []bool{false, true, false, true}) {
		t.Fatalf("serialized admission transitions = %v", admission)
	}
}

func TestCoordinatorRequiresExplicitPositiveLimitsAndStrictAdapter(t *testing.T) {
	directory := t.TempDir()
	valid := coordinatorConfig(filepath.Join(directory, "repository"), filepath.Join(directory, "source.sqlite"), 1)
	invalidLimits := valid
	invalidLimits.Limits.MaxWALBytes = 0
	if _, err := replicator.NewCoordinator(invalidLimits); err == nil {
		t.Fatal("coordinator accepted a zero WAL limit")
	}
	missingAdmission := valid
	missingAdmission.SetAdmission = nil
	if _, err := replicator.NewCoordinator(missingAdmission); err == nil {
		t.Fatal("strict coordinator accepted a missing admission controller")
	}
	missingCapture := valid
	missingCapture.Capture = nil
	if _, err := replicator.NewCoordinator(missingCapture); err == nil {
		t.Fatal("coordinator accepted a missing capture operation")
	}
}

func TestCoordinatorFlushLeavesWritesBlockedWhenSpoolLimitIsExceeded(t *testing.T) {
	ctx := context.Background()
	directory := t.TempDir()
	databasePath := filepath.Join(directory, "source.sqlite")
	createSourceDatabase(t, databasePath)
	repositoryRoot := filepath.Join(directory, "repository")
	if _, err := replication.Snapshot(ctx, databasePath, repositoryRoot); err != nil {
		t.Fatalf("create initial snapshot: %v", err)
	}
	var admission []bool
	config := coordinatorConfig(repositoryRoot, databasePath, 1)
	config.Limits.MaxSpoolBytes = 1
	config.SetAdmission = func(_ context.Context, enabled bool, _ replicator.Limits) error {
		admission = append(admission, enabled)
		return nil
	}
	coordinator, err := replicator.NewCoordinator(config)
	if err != nil {
		t.Fatalf("construct coordinator: %v", err)
	}
	if err := coordinator.Flush(ctx); !errors.Is(err, replicator.ErrBackpressure) {
		t.Fatalf("flush error = %v, want backpressure", err)
	}
	if !reflect.DeepEqual(admission, []bool{false}) {
		t.Fatalf("admission transitions = %v, want writes to remain blocked", admission)
	}
}

func TestCoordinatorFlushLeavesWritesBlockedAfterCaptureErrorOrCancellation(t *testing.T) {
	ctx := context.Background()
	directory := t.TempDir()
	databasePath := filepath.Join(directory, "source.sqlite")
	createSourceDatabase(t, databasePath)
	repositoryRoot := filepath.Join(directory, "repository")
	if _, err := replication.Snapshot(ctx, databasePath, repositoryRoot); err != nil {
		t.Fatalf("create initial snapshot: %v", err)
	}
	var admission []bool
	config := coordinatorConfig(repositoryRoot, databasePath, 2)
	config.Capture = func(context.Context) (replicator.CaptureResult, error) {
		return replicator.CaptureResult{}, errors.New("injected capture failure")
	}
	config.SetAdmission = func(_ context.Context, enabled bool, _ replicator.Limits) error {
		admission = append(admission, enabled)
		return nil
	}
	coordinator, err := replicator.NewCoordinator(config)
	if err != nil {
		t.Fatalf("construct coordinator: %v", err)
	}
	if err := coordinator.Flush(ctx); err == nil {
		t.Fatal("Flush succeeded after capture failure")
	}
	if !reflect.DeepEqual(admission, []bool{false}) {
		t.Fatalf("admission transitions after capture failure = %v", admission)
	}

	admission = nil
	canceled, cancel := context.WithCancel(ctx)
	cancel()
	if err := coordinator.Flush(canceled); !errors.Is(err, context.Canceled) {
		t.Fatalf("canceled Flush error = %v, want context cancellation", err)
	}
	if !reflect.DeepEqual(admission, []bool{false}) {
		t.Fatalf("admission transitions after cancellation = %v", admission)
	}
}

func TestCoordinatorRunPerformsInitialFlushAndClosesAdmissionOnExit(t *testing.T) {
	ctx := context.Background()
	directory := t.TempDir()
	databasePath := filepath.Join(directory, "source.sqlite")
	createSourceDatabase(t, databasePath)
	repositoryRoot := filepath.Join(directory, "repository")
	if _, err := replication.Snapshot(ctx, databasePath, repositoryRoot); err != nil {
		t.Fatalf("create initial snapshot: %v", err)
	}
	var admission []bool
	config := coordinatorConfig(repositoryRoot, databasePath, 1)
	config.SetAdmission = func(_ context.Context, enabled bool, _ replicator.Limits) error {
		admission = append(admission, enabled)
		return nil
	}
	coordinator, err := replicator.NewCoordinator(config)
	if err != nil {
		t.Fatalf("construct coordinator: %v", err)
	}
	events := make(chan struct{})
	close(events)
	if err := coordinator.Run(ctx, events); err != nil {
		t.Fatalf("run coordinator: %v", err)
	}
	if !reflect.DeepEqual(admission, []bool{false, false, true, false}) {
		t.Fatalf("admission transitions = %v, want [false false true false]", admission)
	}
}

func TestCoordinatorRunCapturesAfterCoalescibleCommitEvent(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	directory := t.TempDir()
	databasePath := filepath.Join(directory, "source.sqlite")
	createSourceDatabase(t, databasePath)
	repositoryRoot := filepath.Join(directory, "repository")
	if _, err := replication.Snapshot(context.Background(), databasePath, repositoryRoot); err != nil {
		t.Fatalf("create initial snapshot: %v", err)
	}
	var observed atomic.Uint64
	observed.Store(1)
	admissionEvents := make(chan bool, 8)
	config := coordinatorConfig(repositoryRoot, databasePath, 1)
	config.ObserveTXID = func(context.Context) (uint64, bool, error) {
		return observed.Load(), true, nil
	}
	config.Capture = func(ctx context.Context) (replicator.CaptureResult, error) {
		return replicator.CaptureOnce(ctx, databasePath, repositoryRoot, func(context.Context) (func() error, error) {
			return func() error { return nil }, nil
		})
	}
	config.SetAdmission = func(_ context.Context, enabled bool, _ replicator.Limits) error {
		admissionEvents <- enabled
		return nil
	}
	coordinator, err := replicator.NewCoordinator(config)
	if err != nil {
		t.Fatalf("construct coordinator: %v", err)
	}
	events := make(chan struct{}, 1)
	runResult := make(chan error, 1)
	go func() {
		runResult <- coordinator.Run(ctx, events)
	}()
	awaitAdmission(t, admissionEvents, false)
	awaitAdmission(t, admissionEvents, false)
	awaitAdmission(t, admissionEvents, true)

	database, err := sql.Open("sqlite", databasePath)
	if err != nil {
		t.Fatalf("open source database: %v", err)
	}
	if _, err := database.ExecContext(context.Background(), "UPDATE records SET value = ? WHERE id = 1", "event-target"); err != nil {
		t.Fatalf("commit event target: %v", err)
	}
	if err := database.Close(); err != nil {
		t.Fatalf("close source database: %v", err)
	}
	observed.Store(2)
	events <- struct{}{}
	awaitAdmission(t, admissionEvents, false)
	awaitAdmission(t, admissionEvents, true)
	cancel()
	if err := <-runResult; !errors.Is(err, context.Canceled) {
		t.Fatalf("Run result = %v, want context cancellation", err)
	}
	awaitAdmission(t, admissionEvents, false)
	status, err := coordinator.Status(context.Background())
	if err != nil {
		t.Fatalf("read status after event capture: %v", err)
	}
	if status.LocalDurable == nil || status.LocalDurable.TXID != 2 {
		t.Fatalf("local durable position after event = %+v, want TXID 2", status.LocalDurable)
	}
}

func awaitAdmission(t *testing.T, events <-chan bool, want bool) {
	t.Helper()
	select {
	case got := <-events:
		if got != want {
			t.Fatalf("admission state = %t, want %t", got, want)
		}
	case <-time.After(2 * time.Second):
		t.Fatalf("timed out waiting for admission state %t", want)
	}
}

func coordinatorConfig(repositoryRoot, databasePath string, observed uint64) replicator.CoordinatorConfig {
	return replicator.CoordinatorConfig{
		RepositoryRoot: repositoryRoot,
		DatabasePath:   databasePath,
		ObserveTXID: func(context.Context) (uint64, bool, error) {
			return observed, true, nil
		},
		Capture: func(context.Context) (replicator.CaptureResult, error) {
			return replicator.CaptureResult{TXID: observed}, nil
		},
		SetAdmission: func(context.Context, bool, replicator.Limits) error { return nil },
		Limits: replicator.Limits{
			MaxSpoolBytes:      1 << 30,
			MaxWALBytes:        64 << 20,
			MaxUnprotectedAge:  time.Hour,
			MaxUploadsInFlight: 1,
		},
	}
}
