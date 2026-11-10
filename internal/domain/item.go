package domain

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json/jsontext"
	"encoding/json/v2"
	"errors"
	"fmt"
	"strings"
)

const DefaultWorkspace = "Inbox"

type Item struct {
	ID          string   `json:"id,omitempty"`
	Title       string   `json:"title"`
	Description string   `json:"description"`
	Priority    Priority `json:"priority"`
	Status      Status   `json:"status"`
	Workspace   string   `json:"workspace"`
}

func NewItem(title, workspace string) Item {
	return Item{
		ID:        NewID(),
		Title:     title,
		Priority:  Medium,
		Status:    Todo,
		Workspace: firstTrimmed(workspace),
	}
}

func (it *Item) UnmarshalJSONFrom(dec *jsontext.Decoder) error {
	var raw struct {
		ID          string    `json:"id"`
		Title       *string   `json:"title"`
		Description string    `json:"description"`
		Priority    *Priority `json:"priority"`
		Status      *Status   `json:"status"`
		Workspace   string    `json:"workspace"`
		Tags        []string  `json:"tags"`
	}
	if err := json.UnmarshalDecode(dec, &raw); err != nil {
		return err
	}
	if raw.Title == nil || raw.Priority == nil || raw.Status == nil {
		return errors.New("task is missing required fields")
	}
	*it = Item{
		ID:          raw.ID,
		Title:       *raw.Title,
		Description: raw.Description,
		Priority:    *raw.Priority,
		Status:      *raw.Status,
		Workspace:   firstTrimmed(append([]string{raw.Workspace}, raw.Tags...)...),
	}
	return nil
}

func firstTrimmed(values ...string) string {
	for _, v := range values {
		if v = strings.TrimSpace(v); v != "" {
			return v
		}
	}
	return DefaultWorkspace
}

func NewID() string {
	raw := make([]byte, 8)
	_, _ = rand.Read(raw)
	return hex.EncodeToString(raw)
}

func AssignMissingIDs(items []Item) {
	seen := map[string]int{}
	for i := range items {
		if items[i].ID != "" {
			continue
		}
		key := items[i].Workspace + "\x00" + items[i].Title
		items[i].ID = fmt.Sprintf("legacy-%x-%d", fnv1a(key), seen[key])
		seen[key]++
	}
}

func fnv1a(s string) uint64 {
	hash := uint64(14695981039346656037)
	for i := 0; i < len(s); i++ {
		hash = (hash ^ uint64(s[i])) * 1099511628211
	}
	return hash
}
