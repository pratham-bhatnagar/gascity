package main

import (
	"testing"

	"github.com/gastownhall/gascity/internal/beads"
)

func TestApplyMemoryFields(t *testing.T) {
	b := beads.Bead{Title: "test memory"}
	fields := MemoryFields{
		Kind:       "pattern",
		Confidence: "0.85",
		Scope:      "rig",
		DecayAt:    "2026-06-01T00:00:00Z",
	}
	applyMemoryFields(&b, fields)

	if b.Metadata == nil {
		t.Fatal("expected metadata to be initialized")
	}
	if got := b.Metadata["memory.kind"]; got != "pattern" {
		t.Errorf("memory.kind = %q, want %q", got, "pattern")
	}
	if got := b.Metadata["memory.confidence"]; got != "0.85" {
		t.Errorf("memory.confidence = %q, want %q", got, "0.85")
	}
	if got := b.Metadata["memory.scope"]; got != "rig" {
		t.Errorf("memory.scope = %q, want %q", got, "rig")
	}
	if got := b.Metadata["memory.decay_at"]; got != "2026-06-01T00:00:00Z" {
		t.Errorf("memory.decay_at = %q, want %q", got, "2026-06-01T00:00:00Z")
	}
	// Empty fields should not be set.
	if _, ok := b.Metadata["memory.source_bead"]; ok {
		t.Error("empty SourceBead should not appear in metadata")
	}
	if _, ok := b.Metadata["memory.source_event"]; ok {
		t.Error("empty SourceEvent should not appear in metadata")
	}
}

func TestApplyMemoryFieldsNilMetadata(t *testing.T) {
	b := beads.Bead{Title: "test"}
	applyMemoryFields(&b, MemoryFields{Kind: "skill"})
	if b.Metadata == nil {
		t.Fatal("expected metadata to be initialized")
	}
	if got := b.Metadata["memory.kind"]; got != "skill" {
		t.Errorf("memory.kind = %q, want %q", got, "skill")
	}
}

func TestApplyMemoryFieldsEmptyFieldsNoMetadata(t *testing.T) {
	b := beads.Bead{Title: "test"}
	applyMemoryFields(&b, MemoryFields{})
	if b.Metadata != nil {
		t.Error("empty fields should not initialize metadata")
	}
}

func TestGetMemoryFields(t *testing.T) {
	b := beads.Bead{
		Title: "test memory",
		Metadata: map[string]string{
			"memory.kind":         "decision",
			"memory.confidence":   "0.7",
			"memory.scope":        "global",
			"memory.decay_at":     "2026-12-31T23:59:59Z",
			"memory.source_bead":  "gc-abc",
			"memory.access_count": "5",
			"unrelated.key":       "should be ignored",
		},
	}
	fields := getMemoryFields(b)

	if fields.Kind != "decision" {
		t.Errorf("Kind = %q, want %q", fields.Kind, "decision")
	}
	if fields.Confidence != "0.7" {
		t.Errorf("Confidence = %q, want %q", fields.Confidence, "0.7")
	}
	if fields.Scope != "global" {
		t.Errorf("Scope = %q, want %q", fields.Scope, "global")
	}
	if fields.DecayAt != "2026-12-31T23:59:59Z" {
		t.Errorf("DecayAt = %q, want %q", fields.DecayAt, "2026-12-31T23:59:59Z")
	}
	if fields.SourceBead != "gc-abc" {
		t.Errorf("SourceBead = %q, want %q", fields.SourceBead, "gc-abc")
	}
	if fields.AccessCount != "5" {
		t.Errorf("AccessCount = %q, want %q", fields.AccessCount, "5")
	}
}

func TestGetMemoryFieldsNilMetadata(t *testing.T) {
	b := beads.Bead{Title: "no metadata"}
	fields := getMemoryFields(b)
	if fields.Kind != "" || fields.Scope != "" {
		t.Error("expected empty fields for nil metadata")
	}
}

func TestMemoryFieldsRoundtrip(t *testing.T) {
	original := MemoryFields{
		Kind:         "incident",
		Confidence:   "0.95",
		DecayAt:      "2026-09-15T12:00:00Z",
		SourceBead:   "gc-xyz",
		SourceEvent:  "evt-123",
		LastAccessed: "2026-04-04T10:30:00Z",
		AccessCount:  "3",
		Scope:        "town",
	}

	b := beads.Bead{Title: "roundtrip test"}
	applyMemoryFields(&b, original)
	recovered := getMemoryFields(b)

	if recovered != original {
		t.Errorf("roundtrip failed:\n  got:  %+v\n  want: %+v", recovered, original)
	}
}

func TestSetMemoryFields(t *testing.T) {
	store := beads.NewMemStore()
	b, err := store.Create(beads.Bead{Title: "mem test", Type: "memory"})
	if err != nil {
		t.Fatalf("Create: %v", err)
	}

	fields := MemoryFields{
		Kind:        "pattern",
		Confidence:  "0.8",
		AccessCount: "1",
	}
	if err := setMemoryFields(store, b.ID, fields); err != nil {
		t.Fatalf("setMemoryFields: %v", err)
	}

	updated, err := store.Get(b.ID)
	if err != nil {
		t.Fatalf("Get: %v", err)
	}

	got := getMemoryFields(updated)
	if got.Kind != "pattern" {
		t.Errorf("Kind = %q, want %q", got.Kind, "pattern")
	}
	if got.Confidence != "0.8" {
		t.Errorf("Confidence = %q, want %q", got.Confidence, "0.8")
	}
	if got.AccessCount != "1" {
		t.Errorf("AccessCount = %q, want %q", got.AccessCount, "1")
	}
}
