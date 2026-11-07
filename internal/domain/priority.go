package domain

import "fmt"

type Priority int

const (
	Urgent Priority = iota
	High
	Medium
	Low
)

var AllPriorities = [...]Priority{Urgent, High, Medium, Low}

func (p Priority) Next() Priority { return (p + 1) % 4 }
func (p Priority) Prev() Priority { return (p + 3) % 4 }

func (p Priority) Label() string {
	return [...]string{"Urgent", "High", "Medium", "Low"}[p]
}

func (p Priority) ShortLabel() string {
	return [...]string{"Urg", "High", "Med", "Low"}[p]
}

func (p Priority) Glyph() string {
	return [...]string{"▲", "▲", "◆", "▽"}[p]
}

func PriorityFromDigit(r rune) (Priority, bool) {
	if r < '1' || r > '4' {
		return 0, false
	}
	return Priority(r - '1'), true
}

func (p Priority) MarshalText() ([]byte, error) { return []byte(p.Label()), nil }

func (p *Priority) UnmarshalText(b []byte) error {
	for _, q := range AllPriorities {
		if q.Label() == string(b) {
			*p = q
			return nil
		}
	}
	return fmt.Errorf("unknown priority %q", b)
}
