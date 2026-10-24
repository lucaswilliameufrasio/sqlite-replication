package storagefs

import (
	"os"
	"path/filepath"
	"testing"
)

func TestEnsureRepositoryIsVersionedAndStable(t *testing.T) {
	root := t.TempDir()
	if err := EnsureRepository(root); err != nil {
		t.Fatalf("ensure repository: %v", err)
	}
	first, err := ReadRepository(root)
	if err != nil {
		t.Fatalf("read repository: %v", err)
	}
	if first.FormatVersion != FormatVersion || first.RepositoryID == "" || first.DatabaseID == "" {
		t.Fatalf("invalid repository metadata: %+v", first)
	}
	if err := EnsureRepository(root); err != nil {
		t.Fatalf("ensure repository again: %v", err)
	}
	second, err := ReadRepository(root)
	if err != nil {
		t.Fatalf("read repository again: %v", err)
	}
	if first != second {
		t.Fatal("repository identity changed on repeated initialization")
	}
}

func TestPublishIsImmutableAndAcceptsOnlyIdenticalRetry(t *testing.T) {
	dir := t.TempDir()
	finalPath := filepath.Join(dir, "key.ltx")
	first, err := os.CreateTemp(dir, "stage-*")
	if err != nil {
		t.Fatal(err)
	}
	firstPath := first.Name()
	if _, err := first.WriteString("content"); err != nil {
		t.Fatal(err)
	}
	if err := first.Sync(); err != nil {
		t.Fatal(err)
	}
	if err := first.Close(); err != nil {
		t.Fatal(err)
	}
	if err := Publish(firstPath, finalPath); err != nil {
		t.Fatalf("publish first object: %v", err)
	}

	identical, err := os.CreateTemp(dir, "stage-*")
	if err != nil {
		t.Fatal(err)
	}
	identicalPath := identical.Name()
	_, _ = identical.WriteString("content")
	_ = identical.Sync()
	_ = identical.Close()
	if err := Publish(identicalPath, finalPath); err != nil {
		t.Fatalf("publish identical retry: %v", err)
	}

	different, err := os.CreateTemp(dir, "stage-*")
	if err != nil {
		t.Fatal(err)
	}
	differentPath := different.Name()
	_, _ = different.WriteString("different")
	_ = different.Sync()
	_ = different.Close()
	if err := Publish(differentPath, finalPath); err == nil {
		t.Fatal("expected divergent immutable retry to fail")
	}
	stored, err := os.ReadFile(finalPath)
	if err != nil {
		t.Fatal(err)
	}
	if string(stored) != "content" {
		t.Fatal("existing immutable object changed")
	}
}
