package sqliteadapter

import "testing"

func TestMinimumSQLiteVersion(t *testing.T) {
	for _, test := range []struct {
		version string
		allowed bool
	}{
		{version: "3.51.2", allowed: false},
		{version: "3.51.3", allowed: true},
		{version: "3.51.10", allowed: true},
		{version: "3.52.0", allowed: true},
		{version: "4.0.0", allowed: true},
		{version: "3.9.99", allowed: false},
		{version: "3.51", allowed: false},
		{version: "3.51.3.1", allowed: false},
		{version: "3.51.x", allowed: false},
		{version: "3.-1.3", allowed: false},
		{version: "", allowed: false},
	} {
		t.Run(test.version, func(t *testing.T) {
			if got := versionAtLeast(test.version, minimumSQLiteVersion); got != test.allowed {
				t.Fatalf("versionAtLeast(%q, %q) = %t; want %t", test.version, minimumSQLiteVersion, got, test.allowed)
			}
		})
	}
}
