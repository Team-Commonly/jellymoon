package config

import (
	"os"
	"testing"
)

// SaveUserToken writes the file UserToken reads, 0600 from its first byte, and replaces a looser file whole.
func TestSaveUserTokenIs0600AndAtomic(t *testing.T) {
	s := Store{Dir: t.TempDir()}
	if err := os.WriteFile(s.Path("user-token"), []byte("jmu_old\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := s.SaveUserToken("jmu_new"); err != nil {
		t.Fatal(err)
	}
	fi, err := os.Stat(s.Path("user-token"))
	if err != nil || fi.Mode().Perm() != 0o600 {
		t.Fatalf("user-token: %v mode %v, want 0600", err, fi.Mode().Perm())
	}
	if v, err := s.UserToken("T_NONE"); err != nil || v != "jmu_new" {
		t.Fatalf("UserToken after SaveUserToken: %q %v", v, err)
	}
	if list, _ := os.ReadDir(s.Dir); len(list) != 1 {
		t.Fatalf("the dir holds %d files, want only user-token (no temp file left)", len(list))
	}
}
