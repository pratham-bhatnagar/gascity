package main

import (
	"github.com/gastownhall/gascity/internal/beads"
)

// MemoryFields holds structured metadata for memory beads. These map to
// individual key-value pairs stored via Store.SetMetadata, following the
// same pattern as ConvoyFields.
type MemoryFields struct {
	Kind         string // memory_kind: pattern, decision, incident, skill, context, anti-pattern
	Confidence   string // 0.0–1.0 reliability score
	DecayAt      string // RFC 3339 timestamp after which this memory should be archived
	SourceBead   string // bead ID that originated this memory
	SourceEvent  string // event ID that originated this memory
	LastAccessed string // RFC 3339 timestamp of last retrieval
	AccessCount  string // number of times recalled
	Scope        string // agent, rig, town, global
}

// memoryFieldKeys maps MemoryFields struct fields to their metadata key names.
var memoryFieldKeys = [...]struct {
	key    string
	getter func(*MemoryFields) string
	setter func(*MemoryFields, string)
}{
	{"memory.kind", func(f *MemoryFields) string { return f.Kind }, func(f *MemoryFields, v string) { f.Kind = v }},
	{"memory.confidence", func(f *MemoryFields) string { return f.Confidence }, func(f *MemoryFields, v string) { f.Confidence = v }},
	{"memory.decay_at", func(f *MemoryFields) string { return f.DecayAt }, func(f *MemoryFields, v string) { f.DecayAt = v }},
	{"memory.source_bead", func(f *MemoryFields) string { return f.SourceBead }, func(f *MemoryFields, v string) { f.SourceBead = v }},
	{"memory.source_event", func(f *MemoryFields) string { return f.SourceEvent }, func(f *MemoryFields, v string) { f.SourceEvent = v }},
	{"memory.last_accessed", func(f *MemoryFields) string { return f.LastAccessed }, func(f *MemoryFields, v string) { f.LastAccessed = v }},
	{"memory.access_count", func(f *MemoryFields) string { return f.AccessCount }, func(f *MemoryFields, v string) { f.AccessCount = v }},
	{"memory.scope", func(f *MemoryFields) string { return f.Scope }, func(f *MemoryFields, v string) { f.Scope = v }},
}

// applyMemoryFields populates a Bead's Metadata map with non-empty MemoryFields.
// Call before store.Create to include metadata atomically in the creation.
func applyMemoryFields(b *beads.Bead, fields MemoryFields) {
	for _, kv := range memoryFieldKeys {
		v := kv.getter(&fields)
		if v == "" {
			continue
		}
		if b.Metadata == nil {
			b.Metadata = make(map[string]string)
		}
		b.Metadata[kv.key] = v
	}
}

// setMemoryFields writes non-empty MemoryFields to the bead store as metadata.
// Used for post-creation updates (e.g., bumping access count on recall).
func setMemoryFields(store beads.Store, id string, fields MemoryFields) error {
	for _, kv := range memoryFieldKeys {
		v := kv.getter(&fields)
		if v == "" {
			continue
		}
		if err := store.SetMetadata(id, kv.key, v); err != nil {
			return err
		}
	}
	return nil
}

// getMemoryFields reads MemoryFields from a bead's Metadata map.
// Returns empty fields for keys that are not set.
func getMemoryFields(b beads.Bead) MemoryFields {
	var fields MemoryFields
	if b.Metadata == nil {
		return fields
	}
	for _, kv := range memoryFieldKeys {
		if v, ok := b.Metadata[kv.key]; ok {
			kv.setter(&fields, v)
		}
	}
	return fields
}
