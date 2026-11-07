package paths

import (
	"os"
	"path/filepath"
	"runtime"
)

func home() string {
	h, _ := os.UserHomeDir()
	return h
}

func xdg(env, fallback string) string {
	if dir := os.Getenv(env); filepath.IsAbs(dir) {
		return dir
	}
	return filepath.Join(home(), fallback)
}

func Config() string {
	dir, err := os.UserConfigDir()
	if err != nil {
		dir = "."
	}
	return filepath.Join(dir, "hopes")
}

func dataLocal() string {
	switch runtime.GOOS {
	case "darwin":
		return filepath.Join(home(), "Library", "Application Support")
	case "windows":
		if dir := os.Getenv("LOCALAPPDATA"); dir != "" {
			return dir
		}
		return "."
	default:
		return xdg("XDG_DATA_HOME", filepath.Join(".local", "share"))
	}
}

func Data() string { return filepath.Join(dataLocal(), "hopes") }

func State() string {
	if runtime.GOOS != "darwin" && runtime.GOOS != "windows" {
		return filepath.Join(xdg("XDG_STATE_HOME", filepath.Join(".local", "state")), "hopes")
	}
	return Data()
}
