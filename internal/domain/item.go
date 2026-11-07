package domain

import (
	"encoding/json"
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
	return Item{Title: title, Priority: Medium, Status: Todo, Workspace: workspace}
}

type storedItem struct {
	Title       *string   `json:"title"`
	Description string    `json:"description"`
	Priority    *Priority `json:"priority"`
	Status      *Status   `json:"status"`
	Workspace   *string   `json:"workspace"`
	Tags        []string  `json:"tags"`
}

func (it *Item) UnmarshalJSON(data []byte) error {
	var s storedItem
	if err := json.Unmarshal(data, &s); err != nil {
		return err
	}
	switch {
	case s.Title == nil:
		return errors.New("task is missing a title")
	case s.Priority == nil:
		return errors.New("task is missing a priority")
	case s.Status == nil:
		return errors.New("task is missing a status")
	}
	workspace := ""
	if s.Workspace != nil {
		workspace = strings.TrimSpace(*s.Workspace)
	} else if len(s.Tags) > 0 {
		workspace = strings.TrimSpace(s.Tags[0])
	}
	if workspace == "" {
		workspace = DefaultWorkspace
	}
	*it = Item{
		Title:       *s.Title,
		Description: s.Description,
		Priority:    *s.Priority,
		Status:      *s.Status,
		Workspace:   workspace,
	}
	return nil
}
