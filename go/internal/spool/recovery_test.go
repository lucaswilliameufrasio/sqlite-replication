package spool

import (
	"bytes"
	"encoding/binary"
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"testing"

	"sqlite-replication/internal/walcapture"
	"sqlite-replication/ltx"
	"sqlite-replication/storagefs"
)

func TestRecoverPublishedDeltaAheadOfWatermark(t *testing.T) {
	root := filepath.Join(t.TempDir(), "repository")
	baseState := initializeRecoveryRepository(t, root)
	page := bytes.Repeat([]byte{0x6a}, int(baseState.PageSize))
	header := ltx.Header{
		PageSize:         baseState.PageSize,
		Commit:           baseState.Commit,
		MinTXID:          2,
		MaxTXID:          2,
		Timestamp:        123,
		PreApplyChecksum: baseState.Checksum,
	}
	pages := []ltx.Page{{PageNumber: 2, Data: page}}
	updated, err := baseState.ApplyPages(header, pages)
	if err != nil {
		t.Fatalf("calculate recovery delta: %v", err)
	}
	var encoded bytes.Buffer
	if err := ltx.EncodeDelta(header, pages, updated.Checksum, &encoded); err != nil {
		t.Fatalf("encode recovery delta: %v", err)
	}
	transaction := walcapture.Transaction{
		StartFrame:   1,
		EndFrame:     1,
		Header:       header,
		Pages:        pages,
		PostChecksum: updated.Checksum,
		Encoded:      encoded.Bytes(),
	}
	state := walcapture.State{Database: updated}
	err = PublishTransactions(root, []walcapture.Transaction{transaction}, state, func(point FaultPoint, _ int) error {
		if point == AfterLTXPublish {
			return errors.New("simulated crash before watermark")
		}
		return nil
	})
	if err == nil {
		t.Fatal("expected injected failure after LTX publication")
	}
	watermark, err := ReadWatermark(root)
	if err != nil {
		t.Fatalf("read old watermark: %v", err)
	}
	if watermark.TXID != 1 {
		t.Fatalf("watermark advanced before recovery: %+v", watermark)
	}

	recovered, err := Recover(root, nil)
	if err != nil {
		t.Fatalf("recover published delta: %v", err)
	}
	if recovered.Database.TXID != 2 || recovered.Database.Checksum != updated.Checksum {
		t.Fatalf("recovered database state differs from published delta: %+v", recovered.Database)
	}
	recoveredWatermark, err := ReadWatermark(root)
	if err != nil {
		t.Fatalf("read reconciled watermark: %v", err)
	}
	if recoveredWatermark.TXID != 2 || recoveredWatermark.DatabaseChecksum != updated.Checksum {
		t.Fatalf("recovery did not advance watermark to published chain: %+v", recoveredWatermark)
	}
	repeated, err := Recover(root, nil)
	if err != nil {
		t.Fatalf("repeat recovery: %v", err)
	}
	if repeated.Database.TXID != recovered.Database.TXID || repeated.Database.Checksum != recovered.Database.Checksum {
		t.Fatal("recovery was not idempotent")
	}
}

func TestRecoverRejectsGapOrDivergentWatermark(t *testing.T) {
	root := filepath.Join(t.TempDir(), "repository")
	baseState := initializeRecoveryRepository(t, root)
	watermark, err := ReadWatermark(root)
	if err != nil {
		t.Fatalf("read initial watermark: %v", err)
	}
	watermark.DatabaseChecksum ^= 1
	watermarkPath := filepath.Join(root, "_meta", "capture-watermark.json")
	watermarkBytes, err := json.Marshal(watermark)
	if err != nil {
		t.Fatalf("encode divergent watermark fixture: %v", err)
	}
	if err := os.WriteFile(watermarkPath, watermarkBytes, 0o600); err != nil {
		t.Fatalf("write divergent watermark fixture: %v", err)
	}
	if _, err := Recover(root, nil); err == nil {
		t.Fatal("accepted a watermark that differs from the verified snapshot")
	}
	if baseState.TXID != 1 {
		t.Fatalf("unexpected fixture base TXID: %d", baseState.TXID)
	}
}

func initializeRecoveryRepository(t *testing.T, root string) ltx.DatabaseState {
	t.Helper()
	if err := storagefs.EnsureRepository(root); err != nil {
		t.Fatalf("initialize repository: %v", err)
	}
	image := make([]byte, 2*512)
	copy(image[:16], "SQLite format 3\x00")
	binary.BigEndian.PutUint16(image[16:18], 512)
	binary.BigEndian.PutUint32(image[28:32], 2)
	baseState, err := ltx.LoadDatabaseState(bytes.NewReader(image), int64(len(image)), 1)
	if err != nil {
		t.Fatalf("load base state: %v", err)
	}
	snapshotPath := storagefs.LTXPath(root, 1, 1)
	if err := os.MkdirAll(filepath.Dir(snapshotPath), 0o750); err != nil {
		t.Fatalf("create snapshot directory: %v", err)
	}
	snapshotStage, err := os.CreateTemp(filepath.Dir(snapshotPath), ".snapshot-*.stage")
	if err != nil {
		t.Fatalf("create snapshot stage: %v", err)
	}
	if _, err := ltx.EncodeSnapshot(bytes.NewReader(image), int64(len(image)), snapshotStage, 1); err != nil {
		_ = snapshotStage.Close()
		t.Fatalf("encode base snapshot: %v", err)
	}
	if err := snapshotStage.Sync(); err != nil {
		_ = snapshotStage.Close()
		t.Fatalf("sync base snapshot: %v", err)
	}
	stagePath := snapshotStage.Name()
	if err := snapshotStage.Close(); err != nil {
		t.Fatalf("close base snapshot: %v", err)
	}
	if err := storagefs.Publish(stagePath, snapshotPath); err != nil {
		t.Fatalf("publish base snapshot: %v", err)
	}
	if err := PublishTransactions(root, nil, walcapture.State{Database: baseState}, nil); err != nil {
		t.Fatalf("publish base watermark: %v", err)
	}
	return baseState
}
