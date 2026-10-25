// Command reference is a test-only LTX v2 oracle pinned to superfly/ltx v0.4.0.
// It is intentionally outside the Go runtime library module.
package main

import (
	"encoding/binary"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"

	"github.com/superfly/ltx"
)

const fixtureTimestampMillis = 1_700_000_000_000

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run(args []string) error {
	if len(args) == 2 && args[0] == "verify-ltx" {
		return verifyLTX(args[1])
	}
	if len(args) != 3 {
		return usageError()
	}
	switch args[0] {
	case "create":
		return createSnapshot(args[1], args[2])
	case "verify":
		return verifyAndRestore(args[1], args[2])
	default:
		return usageError()
	}
}

func usageError() error {
	return errors.New("usage: reference <create DATABASE LTX | verify LTX DATABASE_DESTINATION | verify-ltx LTX>")
}

func verifyLTX(ltxPath string) error {
	file, err := os.Open(ltxPath)
	if err != nil {
		return fmt.Errorf("open LTX file: %w", err)
	}
	defer file.Close()
	decoder := ltx.NewDecoder(file)
	if err := decoder.Verify(); err != nil {
		return fmt.Errorf("verify LTX file with reference decoder: %w", err)
	}
	return nil
}

func createSnapshot(databasePath, ltxPath string) error {
	database, err := os.Open(databasePath)
	if err != nil {
		return fmt.Errorf("open SQLite image: %w", err)
	}
	defer database.Close()
	info, err := database.Stat()
	if err != nil {
		return fmt.Errorf("stat SQLite image: %w", err)
	}
	if info.Size() < 100 {
		return errors.New("SQLite image is too short")
	}
	var databaseHeader [100]byte
	if _, err := database.ReadAt(databaseHeader[:], 0); err != nil {
		return fmt.Errorf("read SQLite header: %w", err)
	}
	if string(databaseHeader[:16]) != "SQLite format 3\x00" {
		return errors.New("invalid SQLite database header")
	}
	pageSize := uint32(binary.BigEndian.Uint16(databaseHeader[16:18]))
	if pageSize == 1 {
		pageSize = 65536
	}
	pageCount := uint32(info.Size() / int64(pageSize))
	if pageCount == 0 || int64(pageCount)*int64(pageSize) != info.Size() {
		return errors.New("invalid SQLite page count")
	}
	if headerPageCount := binary.BigEndian.Uint32(databaseHeader[28:32]); headerPageCount != 0 && headerPageCount != pageCount {
		return errors.New("SQLite header page count does not match file length")
	}

	if err := os.MkdirAll(filepath.Dir(ltxPath), 0o750); err != nil {
		return fmt.Errorf("create LTX directory: %w", err)
	}
	output, err := os.OpenFile(ltxPath, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
	if err != nil {
		return fmt.Errorf("create LTX file: %w", err)
	}
	defer output.Close()
	encoder, err := ltx.NewEncoder(output)
	if err != nil {
		return fmt.Errorf("create reference encoder: %w", err)
	}
	header := ltx.Header{
		Version:   ltx.Version,
		PageSize:  pageSize,
		Commit:    pageCount,
		MinTXID:   1,
		MaxTXID:   1,
		Timestamp: fixtureTimestampMillis,
	}
	if err := encoder.EncodeHeader(header); err != nil {
		return fmt.Errorf("write reference header: %w", err)
	}
	page := make([]byte, pageSize)
	checksum := ltx.ChecksumFlag
	lockPage := ltx.LockPgno(pageSize)
	for pageNo := uint32(1); pageNo <= pageCount; pageNo++ {
		if pageNo == lockPage {
			continue
		}
		if _, err := database.ReadAt(page, int64(pageNo-1)*int64(pageSize)); err != nil {
			return fmt.Errorf("read SQLite page %d: %w", pageNo, err)
		}
		if err := encoder.EncodePage(ltx.PageHeader{Pgno: pageNo}, page); err != nil {
			return fmt.Errorf("write reference page %d: %w", pageNo, err)
		}
		checksum = ltx.ChecksumFlag | (checksum ^ ltx.ChecksumPage(pageNo, page))
	}
	encoder.SetPostApplyChecksum(checksum)
	if err := encoder.Close(); err != nil {
		return fmt.Errorf("close reference LTX file: %w", err)
	}
	if err := output.Sync(); err != nil {
		return fmt.Errorf("sync reference LTX file: %w", err)
	}
	return nil
}

func verifyAndRestore(ltxPath, destinationPath string) error {
	input, err := os.Open(ltxPath)
	if err != nil {
		return fmt.Errorf("open LTX file: %w", err)
	}
	defer input.Close()
	if err := os.MkdirAll(filepath.Dir(destinationPath), 0o750); err != nil {
		return fmt.Errorf("create restore directory: %w", err)
	}
	destination, err := os.OpenFile(destinationPath, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
	if err != nil {
		return fmt.Errorf("create restored database: %w", err)
	}
	decoder := ltx.NewDecoder(input)
	if err := decoder.DecodeDatabaseTo(destination); err != nil {
		_ = destination.Close()
		_ = os.Remove(destinationPath)
		return fmt.Errorf("decode reference LTX: %w", err)
	}
	if err := destination.Sync(); err != nil {
		_ = destination.Close()
		_ = os.Remove(destinationPath)
		return fmt.Errorf("sync reference restore: %w", err)
	}
	if err := destination.Close(); err != nil {
		_ = os.Remove(destinationPath)
		return fmt.Errorf("close reference restore: %w", err)
	}
	return nil
}

var _ io.ReaderAt = (*os.File)(nil)
