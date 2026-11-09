// Command reference is a test-only LTX v2 oracle pinned to superfly/ltx v0.4.0.
// It is intentionally outside the Go runtime library module.
package main

import (
	"bytes"
	"encoding/binary"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strconv"

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
	switch args[0] {
	case "create":
		if len(args) != 3 {
			return usageError()
		}
		return createSnapshot(args[1], args[2])
	case "create-at":
		if len(args) != 6 {
			return usageError()
		}
		minTXID, err := ltx.ParseTXID(args[3])
		if err != nil {
			return fmt.Errorf("parse minimum snapshot TXID: %w", err)
		}
		maxTXID, err := ltx.ParseTXID(args[4])
		if err != nil {
			return fmt.Errorf("parse maximum snapshot TXID: %w", err)
		}
		timestamp, err := strconv.ParseInt(args[5], 10, 64)
		if err != nil {
			return fmt.Errorf("parse snapshot timestamp: %w", err)
		}
		return createSnapshotAt(args[1], args[2], minTXID, maxTXID, timestamp)
	case "create-delta":
		if len(args) != 6 {
			return usageError()
		}
		txid, err := ltx.ParseTXID(args[4])
		if err != nil {
			return fmt.Errorf("parse delta TXID: %w", err)
		}
		timestamp, err := strconv.ParseInt(args[5], 10, 64)
		if err != nil {
			return fmt.Errorf("parse delta timestamp: %w", err)
		}
		return createDelta(args[1], args[2], args[3], txid, timestamp)
	case "verify":
		if len(args) != 3 {
			return usageError()
		}
		return verifyAndRestore(args[1], args[2])
	default:
		return usageError()
	}
}

func usageError() error {
	return errors.New("usage: reference <create DATABASE LTX | create-at DATABASE LTX MIN_TXID MAX_TXID TIMESTAMP_MILLIS | create-delta PREVIOUS_DATABASE CURRENT_DATABASE LTX TXID TIMESTAMP_MILLIS | verify LTX DATABASE_DESTINATION | verify-ltx LTX>")
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
	return createSnapshotAt(databasePath, ltxPath, 1, 1, fixtureTimestampMillis)
}

func createSnapshotAt(databasePath, ltxPath string, minTXID, maxTXID ltx.TXID, timestamp int64) error {
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
		MinTXID:   minTXID,
		MaxTXID:   maxTXID,
		Timestamp: timestamp,
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

func createDelta(previousPath, currentPath, ltxPath string, txid ltx.TXID, timestamp int64) error {
	previous, previousPageSize, previousPageCount, err := readDatabaseImage(previousPath)
	if err != nil {
		return fmt.Errorf("read previous SQLite image: %w", err)
	}
	current, currentPageSize, currentPageCount, err := readDatabaseImage(currentPath)
	if err != nil {
		return fmt.Errorf("read current SQLite image: %w", err)
	}
	if previousPageSize != currentPageSize {
		return fmt.Errorf("SQLite page size changed from %d to %d", previousPageSize, currentPageSize)
	}
	preChecksum := checksumDatabaseImage(previous, previousPageSize, previousPageCount)
	postChecksum := checksumDatabaseImage(current, currentPageSize, currentPageCount)
	pageSize := int(currentPageSize)
	type changedPage struct {
		header ltx.PageHeader
		data   []byte
	}
	pages := make([]changedPage, 0)
	for pageNumber := uint32(1); pageNumber <= currentPageCount; pageNumber++ {
		if pageNumber == ltx.LockPgno(currentPageSize) {
			continue
		}
		start := int(pageNumber-1) * pageSize
		end := start + pageSize
		currentPage := current[start:end]
		if pageNumber <= previousPageCount {
			previousStart := int(pageNumber-1) * pageSize
			if bytes.Equal(previous[previousStart:previousStart+pageSize], currentPage) {
				continue
			}
		}
		pages = append(pages, changedPage{header: ltx.PageHeader{Pgno: pageNumber}, data: currentPage})
	}
	if len(pages) == 0 {
		return errors.New("delta fixture has no changed pages")
	}
	if err := os.MkdirAll(filepath.Dir(ltxPath), 0o750); err != nil {
		return fmt.Errorf("create LTX delta directory: %w", err)
	}
	output, err := os.OpenFile(ltxPath, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
	if err != nil {
		return fmt.Errorf("create LTX delta: %w", err)
	}
	defer output.Close()
	encoder, err := ltx.NewEncoder(output)
	if err != nil {
		return fmt.Errorf("create LTX delta encoder: %w", err)
	}
	header := ltx.Header{
		Version:          ltx.Version,
		PageSize:         currentPageSize,
		Commit:           currentPageCount,
		MinTXID:          txid,
		MaxTXID:          txid,
		Timestamp:        timestamp,
		PreApplyChecksum: preChecksum,
	}
	if err := encoder.EncodeHeader(header); err != nil {
		return fmt.Errorf("encode LTX delta header: %w", err)
	}
	for _, page := range pages {
		if err := encoder.EncodePage(page.header, page.data); err != nil {
			return fmt.Errorf("encode LTX delta page %d: %w", page.header.Pgno, err)
		}
	}
	encoder.SetPostApplyChecksum(postChecksum)
	if err := encoder.Close(); err != nil {
		return fmt.Errorf("close LTX delta: %w", err)
	}
	if err := output.Sync(); err != nil {
		return fmt.Errorf("sync LTX delta: %w", err)
	}
	return nil
}

func readDatabaseImage(path string) ([]byte, uint32, uint32, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, 0, 0, err
	}
	if len(data) < 100 || string(data[:16]) != "SQLite format 3\x00" {
		return nil, 0, 0, errors.New("invalid SQLite database header")
	}
	pageSize := uint32(binary.BigEndian.Uint16(data[16:18]))
	if pageSize == 1 {
		pageSize = 65536
	}
	if !ltx.IsValidPageSize(pageSize) || len(data)%int(pageSize) != 0 {
		return nil, 0, 0, errors.New("SQLite image has invalid page size or length")
	}
	pageCount := uint32(len(data) / int(pageSize))
	if headerPageCount := binary.BigEndian.Uint32(data[28:32]); headerPageCount != 0 && headerPageCount != pageCount {
		return nil, 0, 0, errors.New("SQLite header page count does not match file length")
	}
	return data, pageSize, pageCount, nil
}

func checksumDatabaseImage(data []byte, pageSize, pageCount uint32) ltx.Checksum {
	checksum := ltx.ChecksumFlag
	for pageNumber := uint32(1); pageNumber <= pageCount; pageNumber++ {
		if pageNumber == ltx.LockPgno(pageSize) {
			continue
		}
		start := int(pageNumber-1) * int(pageSize)
		checksum ^= ltx.ChecksumPage(pageNumber, data[start:start+int(pageSize)])
	}
	return checksum
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
