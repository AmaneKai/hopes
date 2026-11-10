package domain

import (
	"encoding/json/v2"
	"testing"
)

func TestItemRoundTripsAndFoldsLegacyTags(t *testing.T) {
	in := NewItem("write tests", "Work")
	data, err := json.Marshal(in)
	if err != nil {
		t.Fatal(err)
	}
	var out Item
	if err := json.Unmarshal(data, &out); err != nil {
		t.Fatal(err)
	}
	if out != in {
		t.Fatalf("round trip: got %+v, want %+v", out, in)
	}
}

func TestItemRejectsMissingRequiredFields(t *testing.T) {
	var it Item
	if err := json.Unmarshal([]byte(`{"description":"x"}`), &it); err == nil {
		t.Fatal("expected an error for an item without title/priority/status")
	}
}
