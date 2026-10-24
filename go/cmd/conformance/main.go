package main

import (
	"bytes"
	"context"
	"database/sql"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	_ "modernc.org/sqlite"
	"sqlite-replication"
	"sqlite-replication/internal/sqliteadapter"
	"sqlite-replication/ltx"
	"sqlite-replication/storagefs"
)

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run(args []string) error {
	if len(args) < 2 {
		return usageError()
	}
	switch args[0] {
	case "create":
		if len(args) < 2 || len(args) > 3 {
			return usageError()
		}
		pageSize := 4096
		if len(args) == 3 {
			parsed, err := strconv.Atoi(args[2])
			if err != nil {
				return fmt.Errorf("parse page size: %w", err)
			}
			pageSize = parsed
		}
		return createDatabase(args[1], pageSize)
	case "snapshot":
		if len(args) != 3 {
			return usageError()
		}
		_, err := replication.Snapshot(context.Background(), args[1], args[2])
		return err
	case "create-delta":
		if len(args) != 2 {
			return usageError()
		}
		return createDelta(args[1])
	case "restore":
		if len(args) != 3 {
			return usageError()
		}
		return replication.RestoreSnapshot(context.Background(), args[1], args[2])
	case "decode-ltx":
		if len(args) != 3 {
			return usageError()
		}
		return decodeLTX(args[1], args[2])
	case "verify":
		if len(args) != 2 {
			return usageError()
		}
		return verifyDatabase(args[1])
	default:
		return usageError()
	}
}

func createDelta(path string) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o750); err != nil {
		return fmt.Errorf("create LTX delta directory: %w", err)
	}
	file, err := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
	if err != nil {
		return fmt.Errorf("create LTX delta: %w", err)
	}
	defer file.Close()

	pageOne := bytes.Repeat([]byte{0x11}, 512)
	copy(pageOne[:16], "SQLite format 3\x00")
	pageOne[16] = 2
	header := ltx.Header{
		PageSize:         512,
		Commit:           3,
		MinTXID:          2,
		MaxTXID:          2,
		Timestamp:        1_700_000_000_000,
		PreApplyChecksum: ltx.ChecksumFlag | 0x1234,
		WALOffset:        32,
		WALSize:          32 + 2*(24+512),
		WALSalt1:         0x11223344,
		WALSalt2:         0xaabbccdd,
	}
	pages := []ltx.Page{
		{PageNumber: 1, Data: pageOne},
		{PageNumber: 3, Data: bytes.Repeat([]byte{0x33}, 512)},
	}
	if err := ltx.EncodeDelta(header, pages, ltx.ChecksumFlag|0x9876, file); err != nil {
		return fmt.Errorf("encode LTX delta: %w", err)
	}
	if err := file.Sync(); err != nil {
		return fmt.Errorf("sync LTX delta: %w", err)
	}
	if err := file.Close(); err != nil {
		return fmt.Errorf("close LTX delta: %w", err)
	}
	return nil
}

func decodeLTX(sourcePath, destinationPath string) error {
	if err := os.MkdirAll(filepath.Dir(destinationPath), 0o750); err != nil {
		return fmt.Errorf("create restore directory: %w", err)
	}
	source, err := os.Open(sourcePath)
	if err != nil {
		return fmt.Errorf("open LTX file: %w", err)
	}
	defer source.Close()
	stage, err := os.CreateTemp(filepath.Dir(destinationPath), ".conformance-restore-*.sqlite")
	if err != nil {
		return fmt.Errorf("create staged restore: %w", err)
	}
	stagePath := stage.Name()
	defer os.Remove(stagePath)
	header, err := ltx.DecodeSnapshot(source, stage)
	if err != nil {
		_ = stage.Close()
		return fmt.Errorf("decode LTX snapshot: %w", err)
	}
	if err := stage.Sync(); err != nil {
		_ = stage.Close()
		return fmt.Errorf("sync staged restore: %w", err)
	}
	if err := stage.Close(); err != nil {
		return fmt.Errorf("close staged restore: %w", err)
	}
	info, err := os.Stat(stagePath)
	if err != nil {
		return fmt.Errorf("stat staged restore: %w", err)
	}
	if info.Size() != int64(header.PageSize)*int64(header.Commit) {
		return fmt.Errorf("restored SQLite image has an invalid size")
	}
	if err := sqliteadapter.VerifyDatabase(context.Background(), stagePath); err != nil {
		return err
	}
	return storagefs.Publish(stagePath, destinationPath)
}

func createDatabase(path string, pageSize int) error {
	if pageSize < 512 || pageSize > 65536 || pageSize&(pageSize-1) != 0 {
		return fmt.Errorf("invalid SQLite page size: %d", pageSize)
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o750); err != nil {
		return fmt.Errorf("create database directory: %w", err)
	}
	db, err := sql.Open("sqlite", path)
	if err != nil {
		return fmt.Errorf("open conformance database: %w", err)
	}
	db.SetMaxOpenConns(1)
	defer db.Close()
	if _, err := db.Exec(fmt.Sprintf("PRAGMA page_size = %d", pageSize)); err != nil {
		return fmt.Errorf("set SQLite page size: %w", err)
	}
	if _, err := db.Exec("PRAGMA journal_mode=WAL"); err != nil {
		return fmt.Errorf("enable WAL mode: %w", err)
	}
	if _, err := db.Exec("CREATE TABLE records (id INTEGER PRIMARY KEY, payload TEXT NOT NULL)"); err != nil {
		return fmt.Errorf("create conformance table: %w", err)
	}
	transaction, err := db.Begin()
	if err != nil {
		return fmt.Errorf("begin conformance transaction: %w", err)
	}
	for id := 1; id <= 128; id++ {
		payload := fmt.Sprintf("value-%04d-%s", id, strings.Repeat("x", 1024))
		if _, err := transaction.Exec("INSERT INTO records(id, payload) VALUES (?, ?)", id, payload); err != nil {
			_ = transaction.Rollback()
			return fmt.Errorf("insert conformance row: %w", err)
		}
	}
	if err := transaction.Commit(); err != nil {
		return fmt.Errorf("commit conformance rows: %w", err)
	}
	return nil
}

func verifyDatabase(path string) error {
	if err := sqliteadapter.VerifyDatabase(context.Background(), path); err != nil {
		return err
	}
	db, err := sql.Open("sqlite", "file:"+filepath.ToSlash(path)+"?mode=ro")
	if err != nil {
		return fmt.Errorf("open conformance database: %w", err)
	}
	defer db.Close()
	var count, idSum, payloadBytes int
	if err := db.QueryRow("SELECT count(*), sum(id), sum(length(payload)) FROM records").Scan(&count, &idSum, &payloadBytes); err != nil {
		return fmt.Errorf("read conformance database: %w", err)
	}
	if count != 128 || idSum != 8256 || payloadBytes != 132480 {
		return fmt.Errorf("unexpected restored logical contents: count=%d id_sum=%d payload_bytes=%d", count, idSum, payloadBytes)
	}
	return nil
}

func usageError() error {
	return errors.New("usage: conformance <create DATABASE [PAGE_SIZE] | snapshot DATABASE REPOSITORY | create-delta LTX | restore REPOSITORY DESTINATION | decode-ltx LTX DESTINATION | verify DATABASE>")
}
