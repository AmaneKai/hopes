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

var version = "dev"

const usage = `hopes - a terminal task board

Usage:
  hopes             start the app
  hopes --version   print the version
  hopes --help      show this help
`

func main() {
	if len(os.Args) == 2 {
		switch os.Args[1] {
		case "-h", "--help", "help":
			fmt.Print(usage)
			return
		case "-v", "--version", "version":
			fmt.Println("hopes", version)
			return
		}
	}
	if len(os.Args) == 5 && os.Args[1] == gitsync.FlushArg {
		if st := gitsync.Flush(os.Args[2], os.Args[3], os.Args[4]); st.Kind == gitsync.Failed {
			fmt.Fprintln(os.Stderr, "hopes: sync:", st.Detail)
			os.Exit(1)
		}
		return
	}
	if len(os.Args) > 1 {
		fmt.Fprintf(os.Stderr, "hopes: unknown argument %q\n\n%s", os.Args[1], usage)
		os.Exit(2)
	}
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, "hopes:", err)
		os.Exit(1)
	}
}

func run() error {
	cfg, err := config.Load()
	if err != nil {
		return err
	}
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
	app.Warn(cfg.Warnings...)
	if view, ok := viewstate.Load(); ok {
		app.RestoreView(view)
	}
	_, err = tea.NewProgram(ui.Model{App: app}).Run()
	app.SaveViewState()
	app.ShutdownSync()
	return err
}
