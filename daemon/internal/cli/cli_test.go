package cli

import (
	"os"
	"testing"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
)

// A restart with the computer already stored never reads the user token, and still drops it from the environment.
func TestComputerForDropsTheUserTokenWhenItIsNotRead(t *testing.T) {
	t.Setenv(config.HomeEnv, t.TempDir())
	t.Setenv(userTokenEnv, "jmu_secret")
	store, err := config.Open()
	if err != nil {
		t.Fatal(err)
	}
	a, err := api.New("https://example.invalid")
	if err != nil {
		t.Fatal(err)
	}
	if err := store.SaveComputer(config.Computer{Server: a.Base.String(), ComputerID: "cmp_x", Token: "jmc_x"}); err != nil {
		t.Fatal(err)
	}
	if _, err := computerFor(store, a); err != nil {
		t.Fatal(err)
	}
	if v, ok := os.LookupEnv(userTokenEnv); ok {
		t.Errorf("%s is still set (%d chars) after computerFor on a stored computer", userTokenEnv, len(v))
	}
}

// The same holds when the stored computer belongs to another server and the start is refused.
func TestComputerForDropsTheUserTokenWhenItRefuses(t *testing.T) {
	t.Setenv(config.HomeEnv, t.TempDir())
	t.Setenv(userTokenEnv, "jmu_secret")
	store, _ := config.Open()
	a, _ := api.New("https://example.invalid")
	_ = store.SaveComputer(config.Computer{Server: "https://other.invalid", ComputerID: "cmp_x", Token: "jmc_x"})
	if _, err := computerFor(store, a); err == nil {
		t.Fatal("want a refusal for a computer registered elsewhere")
	}
	if _, ok := os.LookupEnv(userTokenEnv); ok {
		t.Errorf("%s is still set after a refused start", userTokenEnv)
	}
}
