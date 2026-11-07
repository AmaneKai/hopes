package main

import (
	"fmt"
	"os"

	tea "charm.land/bubbletea/v2"

	"github.com/AmaneKai/hopes/internal/config"
	"github.com/AmaneKai/hopes/internal/gitsync"
	"github.com/AmaneKai/hopes/internal/storage"
	"github.com/AmaneKai/hopes/internal/ui"
	"github.com/AmaneKai/hopes/internal/viewstate"
)

func main() {
	if len(os.Args) == 5 && os.Args[1] == gitsync.FlushArg {
		gitsync.Flush(os.Args[2], os.Args[3], os.Args[4])
		return
	}
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, "hopes:", err)
		os.Exit(1)
	}
}

func run() error {
	cfg := config.Load()
	store, err := storage.Open(cfg.Storage, cfg.Sync.DataDir)
	if err != nil {
		return err
	}
	engine := gitsync.Spawn(gitsync.Options{
		Dir:           store.DataDir(),
		Remote:        cfg.Sync.Remote,
		Branch:        cfg.Sync.Branch,
		Debounce:      cfg.Sync.Debounce,
		PullOnStartup: cfg.Sync.PullOnStartup,
		Enabled:       cfg.Sync.Enabled && cfg.Storage.Backend == config.BackendJSON,
	})
	app := ui.New(store, engine)
	if view, ok := viewstate.Load(); ok {
		app.RestoreView(view)
	}
	_, err = tea.NewProgram(ui.Model{App: app}).Run()
	app.SaveViewState()
	app.ShutdownSync()
	return err
}
