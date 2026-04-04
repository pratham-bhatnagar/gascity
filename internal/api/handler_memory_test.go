package api

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gastownhall/gascity/internal/beads"
)

func TestMemoryCreate(t *testing.T) {
	state := newFakeState(t)
	srv := New(state)

	body := `{"rig":"myrig","title":"Always run tests before pushing","kind":"pattern","confidence":"0.9","scope":"rig"}`
	req := newPostRequest("/v0/memories", bytes.NewBufferString(body))
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusCreated {
		t.Fatalf("status = %d, want %d, body: %s", rec.Code, http.StatusCreated, rec.Body.String())
	}

	var got MemoryResponse
	json.NewDecoder(rec.Body).Decode(&got) //nolint:errcheck
	if got.Title != "Always run tests before pushing" {
		t.Errorf("Title = %q, want %q", got.Title, "Always run tests before pushing")
	}
	if got.Kind != "pattern" {
		t.Errorf("Kind = %q, want %q", got.Kind, "pattern")
	}
	if got.Confidence != "0.9" {
		t.Errorf("Confidence = %q, want %q", got.Confidence, "0.9")
	}
	if got.Scope != "rig" {
		t.Errorf("Scope = %q, want %q", got.Scope, "rig")
	}
	if got.ID == "" {
		t.Error("created memory has no ID")
	}
}

func TestMemoryCreateDefaults(t *testing.T) {
	state := newFakeState(t)
	srv := New(state)

	body := `{"rig":"myrig","title":"Some memory"}`
	req := newPostRequest("/v0/memories", bytes.NewBufferString(body))
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusCreated {
		t.Fatalf("status = %d, want %d, body: %s", rec.Code, http.StatusCreated, rec.Body.String())
	}

	var got MemoryResponse
	json.NewDecoder(rec.Body).Decode(&got) //nolint:errcheck
	if got.Confidence != "0.5" {
		t.Errorf("default Confidence = %q, want %q", got.Confidence, "0.5")
	}
	if got.Scope != "rig" {
		t.Errorf("default Scope = %q, want %q", got.Scope, "rig")
	}
	if got.Kind != "context" {
		t.Errorf("default Kind = %q, want %q", got.Kind, "context")
	}
}

func TestMemoryCreateValidation(t *testing.T) {
	state := newFakeState(t)
	srv := New(state)

	tests := []struct {
		name string
		body string
		want int
	}{
		{"missing title", `{"rig":"myrig"}`, http.StatusBadRequest},
		{"bad confidence", `{"rig":"myrig","title":"x","confidence":"2.0"}`, http.StatusBadRequest},
		{"negative confidence", `{"rig":"myrig","title":"x","confidence":"-0.1"}`, http.StatusBadRequest},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := newPostRequest("/v0/memories", bytes.NewBufferString(tt.body))
			rec := httptest.NewRecorder()
			srv.ServeHTTP(rec, req)
			if rec.Code != tt.want {
				t.Errorf("status = %d, want %d, body: %s", rec.Code, tt.want, rec.Body.String())
			}
		})
	}
}

func TestMemoryList(t *testing.T) {
	state := newFakeState(t)
	store := state.stores["myrig"]

	// Create memory beads directly in the store.
	store.Create(beads.Bead{ //nolint:errcheck
		Title: "Test pattern",
		Type:  "memory",
		Metadata: map[string]string{
			"memory.kind":       "pattern",
			"memory.scope":      "rig",
			"memory.confidence": "0.8",
		},
	})
	store.Create(beads.Bead{ //nolint:errcheck
		Title: "Decision log",
		Type:  "memory",
		Metadata: map[string]string{
			"memory.kind":       "decision",
			"memory.scope":      "town",
			"memory.confidence": "0.9",
		},
	})
	// Non-memory bead should be excluded.
	store.Create(beads.Bead{Title: "Regular task", Type: "task"}) //nolint:errcheck

	srv := New(state)

	// List all memories.
	req := httptest.NewRequest("GET", "/v0/memories", nil)
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("status = %d, want %d", rec.Code, http.StatusOK)
	}

	var resp struct {
		Items []MemoryResponse `json:"items"`
		Total int              `json:"total"`
	}
	json.NewDecoder(rec.Body).Decode(&resp) //nolint:errcheck
	if resp.Total != 2 {
		t.Errorf("Total = %d, want 2", resp.Total)
	}
}

func TestMemoryListScopeFilter(t *testing.T) {
	state := newFakeState(t)
	store := state.stores["myrig"]

	store.Create(beads.Bead{ //nolint:errcheck
		Title: "Rig memory",
		Type:  "memory",
		Metadata: map[string]string{
			"memory.kind":  "pattern",
			"memory.scope": "rig",
		},
	})
	store.Create(beads.Bead{ //nolint:errcheck
		Title: "Town memory",
		Type:  "memory",
		Metadata: map[string]string{
			"memory.kind":  "decision",
			"memory.scope": "town",
		},
	})

	srv := New(state)

	req := httptest.NewRequest("GET", "/v0/memories?scope=rig", nil)
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	var resp struct {
		Items []MemoryResponse `json:"items"`
		Total int              `json:"total"`
	}
	json.NewDecoder(rec.Body).Decode(&resp) //nolint:errcheck
	if resp.Total != 1 {
		t.Errorf("scope filter: Total = %d, want 1", resp.Total)
	}
	if resp.Total > 0 && resp.Items[0].Scope != "rig" {
		t.Errorf("scope = %q, want %q", resp.Items[0].Scope, "rig")
	}
}

func TestMemoryListKeywordSearch(t *testing.T) {
	state := newFakeState(t)
	store := state.stores["myrig"]

	store.Create(beads.Bead{ //nolint:errcheck
		Title: "Always use TDD",
		Type:  "memory",
		Metadata: map[string]string{
			"memory.kind":  "pattern",
			"memory.scope": "rig",
		},
	})
	store.Create(beads.Bead{ //nolint:errcheck
		Title: "Auth tokens expire",
		Type:  "memory",
		Metadata: map[string]string{
			"memory.kind":  "context",
			"memory.scope": "rig",
		},
	})

	srv := New(state)

	req := httptest.NewRequest("GET", "/v0/memories?q=TDD", nil)
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	var resp struct {
		Items []MemoryResponse `json:"items"`
		Total int              `json:"total"`
	}
	json.NewDecoder(rec.Body).Decode(&resp) //nolint:errcheck
	if resp.Total != 1 {
		t.Errorf("keyword search: Total = %d, want 1", resp.Total)
	}
}

func TestMemoryListMinConfidence(t *testing.T) {
	state := newFakeState(t)
	store := state.stores["myrig"]

	store.Create(beads.Bead{ //nolint:errcheck
		Title: "Low confidence",
		Type:  "memory",
		Metadata: map[string]string{
			"memory.kind":       "pattern",
			"memory.scope":      "rig",
			"memory.confidence": "0.3",
		},
	})
	store.Create(beads.Bead{ //nolint:errcheck
		Title: "High confidence",
		Type:  "memory",
		Metadata: map[string]string{
			"memory.kind":       "pattern",
			"memory.scope":      "rig",
			"memory.confidence": "0.9",
		},
	})

	srv := New(state)

	req := httptest.NewRequest("GET", "/v0/memories?min_confidence=0.8", nil)
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	var resp struct {
		Items []MemoryResponse `json:"items"`
		Total int              `json:"total"`
	}
	json.NewDecoder(rec.Body).Decode(&resp) //nolint:errcheck
	if resp.Total != 1 {
		t.Errorf("min_confidence filter: Total = %d, want 1", resp.Total)
	}
	if resp.Total > 0 && resp.Items[0].Title != "High confidence" {
		t.Errorf("Title = %q, want %q", resp.Items[0].Title, "High confidence")
	}
}

func TestMemoryGet(t *testing.T) {
	state := newFakeState(t)
	store := state.stores["myrig"]

	created, err := store.Create(beads.Bead{
		Title: "Test memory",
		Type:  "memory",
		Metadata: map[string]string{
			"memory.kind":  "skill",
			"memory.scope": "agent",
		},
	})
	if err != nil {
		t.Fatal(err)
	}

	srv := New(state)

	req := httptest.NewRequest("GET", "/v0/memory/"+created.ID, nil)
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("status = %d, want %d", rec.Code, http.StatusOK)
	}

	var got MemoryResponse
	json.NewDecoder(rec.Body).Decode(&got) //nolint:errcheck
	if got.Title != "Test memory" {
		t.Errorf("Title = %q, want %q", got.Title, "Test memory")
	}
	if got.Kind != "skill" {
		t.Errorf("Kind = %q, want %q", got.Kind, "skill")
	}
}

func TestMemoryGetNotFound(t *testing.T) {
	state := newFakeState(t)
	srv := New(state)

	req := httptest.NewRequest("GET", "/v0/memory/gc-nonexistent", nil)
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusNotFound {
		t.Errorf("status = %d, want %d", rec.Code, http.StatusNotFound)
	}
}

func TestMemoryGetRejectsNonMemory(t *testing.T) {
	state := newFakeState(t)
	store := state.stores["myrig"]

	created, _ := store.Create(beads.Bead{Title: "Regular task", Type: "task"})

	srv := New(state)

	req := httptest.NewRequest("GET", "/v0/memory/"+created.ID, nil)
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusNotFound {
		t.Errorf("non-memory bead: status = %d, want %d", rec.Code, http.StatusNotFound)
	}
}
