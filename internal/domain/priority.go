package domain

type Priority int

const (
	Urgent Priority = iota
	High
	Medium
	Low
)

var (
	AllPriorities       = [...]Priority{Urgent, High, Medium, Low}
	priorityLabels      = [...]string{"Urgent", "High", "Medium", "Low"}
	priorityShortLabels = [...]string{"Urg", "High", "Med", "Low"}
	priorityGlyphs      = [...]string{"▲", "▲", "◆", "▽"}
)

func (p Priority) Next() Priority     { return cycleEnum(p, len(priorityLabels), 1) }
func (p Priority) Prev() Priority     { return cycleEnum(p, len(priorityLabels), -1) }
func (p Priority) Label() string      { return enumAt(p, priorityLabels[:]) }
func (p Priority) ShortLabel() string { return enumAt(p, priorityShortLabels[:]) }
func (p Priority) Glyph() string      { return enumAt(p, priorityGlyphs[:]) }

func PriorityFromDigit(r rune) (Priority, bool) {
	idx := int(r - '1')
	if idx < 0 || idx >= len(AllPriorities) {
		return 0, false
	}
	return AllPriorities[idx], true
}

func (p Priority) MarshalText() ([]byte, error)  { return marshalEnum(p, priorityLabels[:]) }
func (p *Priority) UnmarshalText(b []byte) error { return unmarshalEnum(b, priorityLabels[:], p) }
