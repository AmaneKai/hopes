package domain

type Status int

const (
	Todo Status = iota
	InProgress
	Complete
)

var (
	AllStatuses       = [...]Status{Todo, InProgress, Complete}
	statusNames       = [...]string{"Todo", "InProgress", "Complete"}
	statusLabels      = [...]string{"To Do", "In Progress", "Done"}
	statusBoardLabels = [...]string{"To Do", "In Progress", "Completed"}
	statusGlyphs      = [...]string{"○", "◐", "✓"}
)

func (s Status) Next() Status       { return cycleEnum(s, len(statusNames), 1) }
func (s Status) Prev() Status       { return cycleEnum(s, len(statusNames), -1) }
func (s Status) Column() int        { return max(0, min(int(s), len(AllStatuses)-1)) }
func (s Status) Label() string      { return enumAt(s, statusLabels[:]) }
func (s Status) BoardLabel() string { return enumAt(s, statusBoardLabels[:]) }
func (s Status) Glyph() string      { return enumAt(s, statusGlyphs[:]) }

func StatusFromColumn(col int) Status {
	if col < 0 || col >= len(AllStatuses) {
		return Complete
	}
	return AllStatuses[col]
}

func (s Status) MarshalText() ([]byte, error)  { return marshalEnum(s, statusNames[:]) }
func (s *Status) UnmarshalText(b []byte) error { return unmarshalEnum(b, statusNames[:], s) }
