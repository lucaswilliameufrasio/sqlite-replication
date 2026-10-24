package spool

import (
	"bytes"
	"encoding/binary"
	"errors"
	"os"
	"path/filepath"
	"testing"

	"sqlite-replication/internal/walcapture"
	"sqlite-replication/ltx"
	"sqlite-replication/storagefs"
)

func TestPublishTransactionsDurabilityAndFaultBoundaries(t *testing.T) {
	root := filepath.Join(t.TempDir(), "repository")
	transaction, state := sampleCapture()

	if err := PublishTransactions(root, []walcapture.Transaction{transaction}, state, nil); err != nil {
		t.Fatalf("publish LTX transaction: %v", err)
	}
	watermark, err := ReadWatermark(root)
	if err != nil {
		t.Fatalf("read published watermark: %v", err)
	}
	if watermark.TXID != state.Database.TXID || watermark.DatabaseChecksum != state.Database.Checksum || watermark.WAL != state.WAL {
		t.Fatalf("watermark differs from durable state: %+v", watermark)
	}
	ltxPath := storagefs.LTXPath(root, transaction.Header.MinTXID, transaction.Header.MaxTXID)
	encoded, err := os.ReadFile(ltxPath)
	if err != nil {
		t.Fatalf("read published LTX file: %v", err)
	}
	if !bytes.Equal(encoded, transaction.Encoded) {
		t.Fatal("published LTX bytes differ from staged transaction")
	}
	if err := PublishTransactions(root, []walcapture.Transaction{transaction}, state, nil); err != nil {
		t.Fatalf("retry identical published transaction: %v", err)
	}

	for _, point := range []FaultPoint{AfterStageSync, BeforeLTXPublish} {
		faultRoot := filepath.Join(t.TempDir(), "fault-before-publish")
		err := PublishTransactions(faultRoot, []walcapture.Transaction{transaction}, state, func(got FaultPoint, _ int) error {
			if got == point {
				return errors.New("injected I/O failure")
			}
			return nil
		})
		if err == nil {
			t.Fatalf("fault at %s did not fail publication", point)
		}
		if _, err := os.Stat(storagefs.LTXPath(faultRoot, transaction.Header.MinTXID, transaction.Header.MaxTXID)); !errors.Is(err, os.ErrNotExist) {
			t.Fatalf("fault at %s published an LTX file: %v", point, err)
		}
		if _, err := ReadWatermark(faultRoot); err == nil {
			t.Fatalf("fault at %s advanced a watermark", point)
		}
	}

	for _, point := range []FaultPoint{AfterLTXPublish, BeforeWatermark, AfterWatermarkSync} {
		faultRoot := filepath.Join(t.TempDir(), "fault-after-publish")
		err := PublishTransactions(faultRoot, []walcapture.Transaction{transaction}, state, func(got FaultPoint, _ int) error {
			if got == point {
				return errors.New("injected crash boundary")
			}
			return nil
		})
		if err == nil {
			t.Fatalf("fault at %s did not fail publication", point)
		}
		if _, err := os.Stat(storagefs.LTXPath(faultRoot, transaction.Header.MinTXID, transaction.Header.MaxTXID)); err != nil {
			t.Fatalf("fault at %s lost already-published LTX file: %v", point, err)
		}
		if _, err := ReadWatermark(faultRoot); err == nil {
			t.Fatalf("fault at %s advanced an unpublished watermark", point)
		}
	}

	for _, test := range []struct {
		point             FaultPoint
		publishedLTX      bool
		injectedOperation string
	}{
		{point: BeforeStageWrite, injectedOperation: "write: no space left on device"},
		{point: AfterStageWrite, injectedOperation: "fsync: input/output error"},
		{point: BeforeStageSync, injectedOperation: "fsync: input/output error"},
		{point: BeforeWatermarkWrite, publishedLTX: true, injectedOperation: "write: no space left on device"},
		{point: BeforeWatermarkSync, publishedLTX: true, injectedOperation: "fsync: input/output error"},
	} {
		faultRoot := filepath.Join(t.TempDir(), "syscall-boundary-failure")
		faultErr := errors.New(test.injectedOperation)
		err := PublishTransactions(faultRoot, []walcapture.Transaction{transaction}, state, func(got FaultPoint, _ int) error {
			if got == test.point {
				return faultErr
			}
			return nil
		})
		if !errors.Is(err, faultErr) {
			t.Fatalf("injected %s at %s returned %v", test.injectedOperation, test.point, err)
		}
		_, statErr := os.Stat(storagefs.LTXPath(faultRoot, transaction.Header.MinTXID, transaction.Header.MaxTXID))
		if test.publishedLTX && statErr != nil {
			t.Fatalf("failure at %s lost already-published LTX: %v", test.point, statErr)
		}
		if !test.publishedLTX && !errors.Is(statErr, os.ErrNotExist) {
			t.Fatalf("failure at %s unexpectedly published LTX: %v", test.point, statErr)
		}
		if _, err := ReadWatermark(faultRoot); err == nil {
			t.Fatalf("failure at %s advanced watermark", test.point)
		}
	}

	faultRoot := filepath.Join(t.TempDir(), "fault-after-watermark")
	err = PublishTransactions(faultRoot, []walcapture.Transaction{transaction}, state, func(got FaultPoint, _ int) error {
		if got == AfterWatermark {
			return errors.New("simulated acknowledgement loss")
		}
		return nil
	})
	if err == nil {
		t.Fatal("expected post-watermark hook error")
	}
	if watermark, readErr := ReadWatermark(faultRoot); readErr != nil || watermark.TXID != state.Database.TXID {
		t.Fatalf("durable watermark not recoverable after acknowledgement loss: %+v, %v", watermark, readErr)
	}
}

func sampleCapture() (walcapture.Transaction, walcapture.State) {
	pageSize := uint32(512)
	image := make([]byte, 2*int(pageSize))
	copy(image[:16], "SQLite format 3\x00")
	binary.BigEndian.PutUint16(image[16:18], uint16(pageSize))
	binary.BigEndian.PutUint32(image[28:32], 2)
	baseState, err := ltx.LoadDatabaseState(bytes.NewReader(image), int64(len(image)), 1)
	if err != nil {
		panic(err)
	}
	page := bytes.Repeat([]byte{0x56}, int(pageSize))
	header := ltx.Header{
		PageSize:         pageSize,
		Commit:           2,
		MinTXID:          2,
		MaxTXID:          2,
		PreApplyChecksum: baseState.Checksum,
		WALOffset:        32,
		WALSize:          32 + 24 + int64(pageSize),
		WALSalt1:         1,
		WALSalt2:         2,
	}
	pages := []ltx.Page{{PageNumber: 2, Data: page}}
	postState, err := baseState.ApplyPages(header, pages)
	if err != nil {
		panic(err)
	}
	var encoded bytes.Buffer
	if err := ltx.EncodeDelta(header, pages, postState.Checksum, &encoded); err != nil {
		panic(err)
	}
	transaction := walcapture.Transaction{
		StartFrame:   1,
		EndFrame:     1,
		Header:       header,
		Pages:        pages,
		PostChecksum: postState.Checksum,
		Encoded:      encoded.Bytes(),
	}
	state := walcapture.State{
		Database: postState,
		WAL: walcapture.Position{
			Initialized:        true,
			CheckpointSequence: 1,
			Salt1:              1,
			Salt2:              2,
			FrameCount:         1,
			Checksum1:          3,
			Checksum2:          4,
		},
	}
	return transaction, state
}
