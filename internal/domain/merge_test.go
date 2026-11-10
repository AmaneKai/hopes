package domain

import "testing"

func item(id, title string) Item {
	it := NewItem(title, "Work")
	it.ID = id
	return it
}

func titles(items []Item) []string {
	out := make([]string, len(items))
	for i, it := range items {
		out[i] = it.Title
	}
	return out
}

func equalTitles(items []Item, want ...string) bool {
	got := titles(items)
	if len(got) != len(want) {
		return false
	}
	for i := range got {
		if got[i] != want[i] {
			return false
		}
	}
	return true
}

func TestMergeKeepsAdditionsFromBothSides(t *testing.T) {
	base := []Item{item("1", "one")}
	mine := []Item{item("1", "one"), item("2", "mine")}
	theirs := []Item{item("1", "one"), item("3", "theirs")}
	if got := Merge(base, mine, theirs); !equalTitles(got, "one", "theirs", "mine") {
		t.Fatalf("got %v", titles(got))
	}
}

func TestMergeCombinesEditsToDifferentFieldsOfTheSameTask(t *testing.T) {
	base := item("1", "one")
	mine, theirs := base, base
	mine.Title = "renamed"
	theirs.Priority = Urgent
	got := Merge([]Item{base}, []Item{mine}, []Item{theirs})
	if len(got) != 1 || got[0].Title != "renamed" || got[0].Priority != Urgent {
		t.Fatalf("got %+v", got)
	}
}

func TestMergeLocalWinsWhenBothChangeTheSameField(t *testing.T) {
	base := item("1", "one")
	mine, theirs := base, base
	mine.Title, theirs.Title = "mine", "theirs"
	if got := Merge([]Item{base}, []Item{mine}, []Item{theirs}); got[0].Title != "mine" {
		t.Fatalf("got %+v", got)
	}
}

func TestMergeDeletionsWinOnlyAgainstUntouchedCopies(t *testing.T) {
	base := []Item{item("1", "one"), item("2", "two")}
	edited := item("2", "two edited")
	mine := []Item{edited}
	theirs := []Item{item("2", "two")}
	got := Merge(base, mine, theirs)
	if !equalTitles(got, "two edited") {
		t.Fatalf("local delete of 1 and edit of 2 should both survive, got %v", titles(got))
	}
	remoteDeletesEdited := Merge(base, []Item{item("1", "one"), edited}, []Item{item("1", "one")})
	if !equalTitles(remoteDeletesEdited, "one", "two edited") {
		t.Fatalf("a locally edited task must outlive a remote delete, got %v", titles(remoteDeletesEdited))
	}
}

func TestLegacyIDsAreStableAcrossLoads(t *testing.T) {
	load := func() []Item {
		items := []Item{NewItem("same", "Work"), NewItem("same", "Work"), NewItem("other", "Work")}
		for i := range items {
			items[i].ID = ""
		}
		AssignMissingIDs(items)
		return items
	}
	first, second := load(), load()
	for i := range first {
		if first[i].ID != second[i].ID || first[i].ID == "" {
			t.Fatalf("id %d differs between loads: %q vs %q", i, first[i].ID, second[i].ID)
		}
	}
	if first[0].ID == first[1].ID {
		t.Fatal("duplicate titles must still get distinct ids")
	}
}
