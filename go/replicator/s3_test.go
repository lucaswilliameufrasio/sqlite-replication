package replicator_test

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
	"github.com/lucaswilliameufrasio/sqlite-replication/replicator"
	"github.com/lucaswilliameufrasio/sqlite-replication/storages3"
)

func TestFlociPublicS3Restore(t *testing.T) {
	endpoint := os.Getenv("SQLITE_REPLICATION_FLOCI_ENDPOINT")
	runID := os.Getenv("SQLITE_REPLICATION_FLOCI_RUN_ID")
	if endpoint == "" || runID == "" {
		t.Skip("requires the isolated Floci service from make test-floci")
	}
	ctx, cancel := context.WithTimeout(context.Background(), time.Minute)
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
	store, err := storages3.New(client, "sqlite-replication-m4-"+runID)
	if err != nil {
		t.Fatalf("initialize S3 adapter: %v", err)
	}
	destination := filepath.Join(t.TempDir(), "restored.sqlite")
	if err := replicator.RestoreFromS3(ctx, store, "rust-recovery-test", destination, replicator.RestoreTarget{TXID: 8}); err != nil {
		t.Fatalf("restore public API from S3: %v", err)
	}
	database, err := sql.Open("sqlite", destination)
	if err != nil {
		t.Fatal(err)
	}
	defer database.Close()
	var value string
	if err := database.QueryRow("SELECT value FROM records WHERE id = 1").Scan(&value); err != nil {
		t.Fatal(err)
	}
	if value != "updated" {
		t.Fatalf("restored value = %q, want updated", value)
	}
}
