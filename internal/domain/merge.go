package domain

import "slices"

func Merge(base, mine, theirs []Item) []Item {
	baseByID, mineByID, theirsByID := indexByID(base), indexByID(mine), indexByID(theirs)
	merged := make([]Item, 0, len(theirs)+len(mine))
	for _, remote := range theirs {
		if item, keep := mergeOne(baseByID, mineByID, remote); keep {
			merged = append(merged, item)
		}
	}
	for _, local := range mine {
		if _, inTheirs := theirsByID[local.ID]; inTheirs {
			continue
		}
		if _, inBase := baseByID[local.ID]; !inBase || local != baseByID[local.ID] {
			merged = append(merged, local)
		}
	}
	return merged
}

func mergeOne(baseByID, mineByID map[string]Item, remote Item) (Item, bool) {
	base, inBase := baseByID[remote.ID]
	local, inMine := mineByID[remote.ID]
	switch {
	case !inBase:
		return remote, true
	case !inMine:
		return remote, remote != base
	}
	return mergeFields(base, local, remote), true
}

func mergeFields(base, mine, theirs Item) Item {
	merged := theirs
	pick(&merged.Title, base.Title, mine.Title)
	pick(&merged.Description, base.Description, mine.Description)
	pick(&merged.Priority, base.Priority, mine.Priority)
	pick(&merged.Status, base.Status, mine.Status)
	pick(&merged.Workspace, base.Workspace, mine.Workspace)
	return merged
}

func pick[T comparable](merged *T, base, mine T) {
	if mine != base {
		*merged = mine
	}
}

func indexByID(items []Item) map[string]Item {
	byID := make(map[string]Item, len(items))
	for _, item := range items {
		byID[item.ID] = item
	}
	return byID
}

func CloneItems(items []Item) []Item { return slices.Clone(items) }
