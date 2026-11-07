package domain

import "fmt"

type Status int

const (
	Todo Status = iota
	InProgress
	Complete
)

var AllStatuses = [...]Status{Todo, InProgress, Complete}

func (s Status) Next() Status { return (s + 1) % 3 }
func (s Status) Prev() Status { return (s + 2) % 3 }

func (s Status) Column() int { return int(s) }

func StatusFromColumn(col int) Status {
	if col < 0 || col > 2 {
		return Complete
	}
	return Status(col)
}

func (s Status) Label() string {
	return [...]string{"To Do", "In Progress", "Done"}[s]
}

func (s Status) BoardLabel() string {
	return [...]string{"To Do", "In Progress", "Completed"}[s]
}

func (s Status) Glyph() string {
	return [...]string{"○", "◐", "✓"}[s]
}

var statusNames = [...]string{"Todo", "InProgress", "Complete"}

func (s Status) MarshalText() ([]byte, error) { return []byte(statusNames[s]), nil }

func (s *Status) UnmarshalText(b []byte) error {
	for i, name := range statusNames {
		if name == string(b) {
			*s = Status(i)
			return nil
		}
	}
	return fmt.Errorf("unknown status %q", b)
}
