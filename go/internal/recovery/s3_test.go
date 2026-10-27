package recovery

import (
	"context"
	"database/sql"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/credentials"
	"github.com/aws/aws-sdk-go-v2/service/s3"
	_ "modernc.org/sqlite"
	"sqlite-replication/ltx"
	"sqlite-replication/storagefs"
	"sqlite-replication/storages3"
)

func TestS3RecoveryKeyMappingAndPrefixValidation(t *testing.T) {
	localKey, err := s3KeyToLocalKey("tenant/db", "tenant/db/000a/0000000000000001-0000000000000007.ltx")
	if err != nil {
		t.Fatalf("s3KeyToLocalKey: %v", err)
	}
	if localKey != "ltx/10/0000000000000001-0000000000000007.ltx" {
		t.Fatalf("local key = %q", localKey)
	}
	remoteKey, err := localKeyToS3Key("tenant/db", localKey)
	if err != nil {
		t.Fatalf("localKeyToS3Key: %v", err)
	}
	if remoteKey != "tenant/db/000a/0000000000000001-0000000000000007.ltx" {
		t.Fatalf("remote key = %q", remoteKey)
	}
	for _, prefix := range []string{"/absolute", "trailing/", "a//b", "a/../b", "a/./b", "nul\x00byte"} {
		if err := validateS3Prefix(prefix); err == nil {
			t.Errorf("validateS3Prefix(%q) succeeded", prefix)
		}
	}
	for _, key := range []string{
		"tenant/db/../000a/0000000000000001-0000000000000007.ltx",
		"tenant/db/000a/../../outside.ltx",
		"tenant/db/000a/not-an-ltx-name.ltx",
	} {
		if _, err := s3KeyToLocalKey("tenant/db", key); err == nil {
			t.Errorf("s3KeyToLocalKey(%q) succeeded", key)
		}
	}
}

func TestFlociS3RecoveryRestoreCompactionAndGC(t *testing.T) {
	endpoint := os.Getenv("SQLITE_REPLICATION_FLOCI_ENDPOINT")
	runID := os.Getenv("SQLITE_REPLICATION_FLOCI_RUN_ID")
	if endpoint == "" || runID == "" {
		t.Skip("requires the isolated Floci service from make test-floci")
	}
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Minute)
	defer cancel()
	awsConfig, err := config.LoadDefaultConfig(ctx,
		config.WithRegion("us-east-1"),
		config.WithCredentialsProvider(credentials.NewStaticCredentialsProvider("test", "test", "")),
	)
	if err != nil {
		t.Fatalf("load local S3 SDK config: %v", err)
	}
	client := s3.NewFromConfig(awsConfig, func(options *s3.Options) {
		options.BaseEndpoint = aws.String(endpoint)
		options.UsePathStyle = true
	})
	bucket := "sqlite-replication-m4-" + runID
	if _, err := client.CreateBucket(ctx, &s3.CreateBucketInput{Bucket: aws.String(bucket)}); err != nil {
		t.Fatalf("create isolated recovery bucket: %v", err)
	}
	store, err := storages3.New(client, bucket)
	if err != nil {
		t.Fatalf("initialize S3 adapter: %v", err)
	}
	prefix := "recovery-test"
	remoteKeys := []string{
		joinS3Key(prefix, "_meta/repository.json"),
		joinS3Key(prefix, "0000/0000000000000001-0000000000000007.ltx"),
		joinS3Key(prefix, "0001/0000000000000001-0000000000000007.ltx"),
	}
	defer func() {
		_ = store.DeleteBatch(context.Background(), remoteKeys)
		_, _ = client.DeleteBucket(context.Background(), &s3.DeleteBucketInput{Bucket: aws.String(bucket)})
	}()

	localRepository := t.TempDir()
	if err := storagefs.EnsureRepository(localRepository); err != nil {
		t.Fatalf("initialize fixture repository: %v", err)
	}
	databasePath := filepath.Join(t.TempDir(), "source.sqlite")
	database, err := sql.Open("sqlite", databasePath)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := database.Exec("CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT NOT NULL); INSERT INTO records VALUES (1, 'source')"); err != nil {
		_ = database.Close()
		t.Fatal(err)
	}
	if err := database.Close(); err != nil {
		t.Fatal(err)
	}
	source, err := os.Open(databasePath)
	if err != nil {
		t.Fatal(err)
	}
	info, err := source.Stat()
	if err != nil {
		_ = source.Close()
		t.Fatal(err)
	}
	localSnapshot := storagefs.LTXPath(localRepository, 1, 7)
	if err := os.MkdirAll(filepath.Dir(localSnapshot), 0o750); err != nil {
		_ = source.Close()
		t.Fatal(err)
	}
	staged, err := os.CreateTemp(filepath.Dir(localSnapshot), ".snapshot-*.ltx")
	if err != nil {
		_ = source.Close()
		t.Fatal(err)
	}
	if _, err := ltx.EncodeSnapshotAt(source, info.Size(), staged, 7, 1700000000007); err != nil {
		_ = source.Close()
		_ = staged.Close()
		t.Fatalf("encode source snapshot: %v", err)
	}
	if err := staged.Sync(); err != nil {
		_ = source.Close()
		_ = staged.Close()
		t.Fatal(err)
	}
	stagedPath := staged.Name()
	if err := staged.Close(); err != nil {
		_ = source.Close()
		t.Fatal(err)
	}
	if err := source.Close(); err != nil {
		t.Fatal(err)
	}
	if err := storagefs.Publish(stagedPath, localSnapshot); err != nil {
		t.Fatalf("publish fixture snapshot: %v", err)
	}
	manifest, err := os.Open(filepath.Join(localRepository, "_meta", "repository.json"))
	if err != nil {
		t.Fatal(err)
	}
	if _, err := store.PutImmutable(ctx, remoteKeys[0], manifest); err != nil {
		_ = manifest.Close()
		t.Fatalf("upload S3 repository metadata: %v", err)
	}
	if err := manifest.Close(); err != nil {
		t.Fatal(err)
	}
	base, err := os.Open(localSnapshot)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := store.PutImmutable(ctx, remoteKeys[1], base); err != nil {
		_ = base.Close()
		t.Fatalf("upload initial S3 snapshot: %v", err)
	}
	if err := base.Close(); err != nil {
		t.Fatal(err)
	}

	restoredPath := filepath.Join(t.TempDir(), "restored.sqlite")
	if err := RestoreS3ToTarget(ctx, store, prefix, restoredPath, Target{TXID: 7}); err != nil {
		t.Fatalf("restore TXID from S3: %v", err)
	}
	assertRecoveryRow(t, restoredPath)

	compactedKey, err := CompactS3ToSnapshot(ctx, store, prefix, Target{TXID: 7})
	if err != nil {
		t.Fatalf("compact S3 restore point: %v", err)
	}
	if compactedKey != remoteKeys[2] {
		t.Fatalf("compacted S3 key = %q, want %q", compactedKey, remoteKeys[2])
	}
	plan, err := BuildS3GCPlan(ctx, store, prefix, []Target{{TXID: 7}})
	if err != nil {
		t.Fatalf("build S3 GC dry-run: %v", err)
	}
	if len(plan.CandidateObjectKeys) != 1 || plan.CandidateObjectKeys[0] != remoteKeys[1] {
		t.Fatalf("unexpected S3 GC candidates: %+v", plan.CandidateObjectKeys)
	}
	if err := ApplyS3GCPlan(ctx, store, prefix, plan); err != nil {
		t.Fatalf("apply revalidated S3 GC plan: %v", err)
	}
	if _, err := store.Head(ctx, remoteKeys[1]); err == nil {
		t.Fatal("collected S3 source snapshot still exists")
	}
	restoredAfterGC := filepath.Join(t.TempDir(), "restored-after-gc.sqlite")
	if err := RestoreS3ToTarget(ctx, store, prefix, restoredAfterGC, Target{TXID: 7}); err != nil {
		t.Fatalf("restore retained S3 point after GC: %v", err)
	}
	assertRecoveryRow(t, restoredAfterGC)
}

func assertRecoveryRow(t *testing.T, path string) {
	t.Helper()
	database, err := sql.Open("sqlite", path)
	if err != nil {
		t.Fatal(err)
	}
	var value string
	if err := database.QueryRow("SELECT value FROM records WHERE id = 1").Scan(&value); err != nil {
		t.Fatal(err)
	}
	if value != "source" {
		t.Fatalf("restored row = %q, want source", value)
	}
	if err := database.Close(); err != nil {
		t.Fatal(err)
	}
}
