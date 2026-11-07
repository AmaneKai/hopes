//go:build !windows

package proc

import (
	"os/exec"
	"syscall"
)

func Detach(cmd *exec.Cmd) {
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
}
