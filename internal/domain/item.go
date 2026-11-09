package domain

import (
	"encoding/json/jsontext"
	"encoding/json/v2"
	"errors"
	"strings"
)

const DefaultWorkspace = "Inbox"

type Item struct {
	Title       string   `json:"title"`
	Description string   `json:"description"`
	Priority    Priority `json:"priority"`
	Status      Status   `json:"status"`
	Workspace   string   `json:"workspace"`
}

func NewItem(title, workspace string) Item {
	return Item{
		Title:     title,
		Priority:  Medium,
		Status:    Todo,
		Workspace: firstTrimmed(workspace),
	}
}

func (it *Item) UnmarshalJSONFrom(dec *jsontext.Decoder) error {
	var raw struct {
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
