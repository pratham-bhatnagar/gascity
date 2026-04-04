package api

import (
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/gastownhall/gascity/internal/beads"
)

// MemoryResponse is the JSON shape for a single memory in API responses.
type MemoryResponse struct {
	ID           string   `json:"id"`
	Title        string   `json:"title"`
	Description  string   `json:"description,omitempty"`
	Status       string   `json:"status"`
	Kind         string   `json:"kind,omitempty"`
	Confidence   string   `json:"confidence,omitempty"`
	Scope        string   `json:"scope,omitempty"`
	DecayAt      string   `json:"decay_at,omitempty"`
	SourceBead   string   `json:"source_bead,omitempty"`
	SourceEvent  string   `json:"source_event,omitempty"`
	LastAccessed string   `json:"last_accessed,omitempty"`
	AccessCount  string   `json:"access_count,omitempty"`
	Labels       []string `json:"labels,omitempty"`
	CreatedAt    string   `json:"created_at,omitempty"`
}

func beadToMemoryResponse(b beads.Bead) MemoryResponse {
	r := MemoryResponse{
		ID:          b.ID,
		Title:       b.Title,
		Description: b.Description,
		Status:      b.Status,
		Labels:      b.Labels,
	}
	if !b.CreatedAt.IsZero() {
		r.CreatedAt = b.CreatedAt.Format(time.RFC3339)
	}
	if b.Metadata != nil {
		r.Kind = b.Metadata["memory.kind"]
		r.Confidence = b.Metadata["memory.confidence"]
		r.Scope = b.Metadata["memory.scope"]
		r.DecayAt = b.Metadata["memory.decay_at"]
		r.SourceBead = b.Metadata["memory.source_bead"]
		r.SourceEvent = b.Metadata["memory.source_event"]
		r.LastAccessed = b.Metadata["memory.last_accessed"]
		r.AccessCount = b.Metadata["memory.access_count"]
	}
	return r
}

// handleMemoryList handles GET /v0/memories.
// Query params: q (keyword search), scope, kind, min_confidence, rig, limit.
func (s *Server) handleMemoryList(w http.ResponseWriter, r *http.Request) {
	q := r.URL.Query()
	keyword := q.Get("q")
	scope := q.Get("scope")
	kind := q.Get("kind")
	minConf := q.Get("min_confidence")
	qRig := q.Get("rig")
	limit := 50
	if l := q.Get("limit"); l != "" {
		if n, err := strconv.Atoi(l); err == nil && n > 0 {
			limit = n
		}
	}

	stores := s.state.BeadStores()
	var rigNames []string
	if qRig != "" {
		if _, ok := stores[qRig]; ok {
			rigNames = []string{qRig}
		}
	} else {
		rigNames = sortedRigNames(stores)
	}

	// Build metadata filters.
	filters := map[string]string{}
	if scope != "" {
		filters["memory.scope"] = scope
	}
	if kind != "" {
		filters["memory.kind"] = kind
	}

	var memories []beads.Bead
	for _, rigName := range rigNames {
		store := stores[rigName]
		list, err := store.ListByMetadata(filters, 0)
		if err != nil {
			continue
		}
		for _, b := range list {
			if b.Type != "memory" {
				continue
			}
			memories = append(memories, b)
		}
	}

	// Keyword search (title + description).
	if keyword != "" {
		kw := strings.ToLower(keyword)
		var matched []beads.Bead
		for _, b := range memories {
			if strings.Contains(strings.ToLower(b.Title), kw) ||
				strings.Contains(strings.ToLower(b.Description), kw) {
				matched = append(matched, b)
			}
		}
		memories = matched
	}

	// Min confidence filter.
	if minConf != "" {
		threshold, err := strconv.ParseFloat(minConf, 64)
		if err == nil {
			var filtered []beads.Bead
			for _, b := range memories {
				if c, ok := b.Metadata["memory.confidence"]; ok {
					if conf, err := strconv.ParseFloat(c, 64); err == nil && conf >= threshold {
						filtered = append(filtered, b)
					}
				}
			}
			memories = filtered
		}
	}

	// Apply limit.
	if len(memories) > limit {
		memories = memories[:limit]
	}

	// Bump access stats for recalled memories.
	now := time.Now().UTC().Format(time.RFC3339)
	for _, rigName := range rigNames {
		store := stores[rigName]
		for _, b := range memories {
			count := 0
			if ac, ok := b.Metadata["memory.access_count"]; ok {
				count, _ = strconv.Atoi(ac)
			}
			count++
			_ = store.SetMetadataBatch(b.ID, map[string]string{
				"memory.last_accessed": now,
				"memory.access_count":  strconv.Itoa(count),
			})
		}
	}

	// Build response.
	items := make([]MemoryResponse, 0, len(memories))
	for _, b := range memories {
		items = append(items, beadToMemoryResponse(b))
	}
	writeListJSON(w, s.latestIndex(), items, len(items))
}

// handleMemoryCreate handles POST /v0/memories.
func (s *Server) handleMemoryCreate(w http.ResponseWriter, r *http.Request) {
	var body struct {
		Rig         string   `json:"rig"`
		Title       string   `json:"title"`
		Description string   `json:"description"`
		Kind        string   `json:"kind"`
		Confidence  string   `json:"confidence"`
		Scope       string   `json:"scope"`
		DecayAt     string   `json:"decay_at"`
		SourceBead  string   `json:"source_bead"`
		SourceEvent string   `json:"source_event"`
		Labels      []string `json:"labels"`
	}
	if err := decodeBody(r, &body); err != nil {
		writeError(w, http.StatusBadRequest, "invalid", err.Error())
		return
	}
	if body.Title == "" {
		writeError(w, http.StatusBadRequest, "invalid", "title is required")
		return
	}

	// Validate confidence if provided.
	if body.Confidence != "" {
		c, err := strconv.ParseFloat(body.Confidence, 64)
		if err != nil || c < 0 || c > 1 {
			writeError(w, http.StatusBadRequest, "invalid", "confidence must be between 0.0 and 1.0")
			return
		}
	}

	// Auto-confidence: default to 0.5 if not specified.
	confidence := body.Confidence
	if confidence == "" {
		confidence = "0.5"
	}

	// Default scope to "rig" if not specified.
	scope := body.Scope
	if scope == "" {
		scope = "rig"
	}

	// Default kind to "context" if not specified.
	kind := body.Kind
	if kind == "" {
		kind = "context"
	}

	// Idempotency check.
	idemKey := scopedIdemKey(r, r.Header.Get("Idempotency-Key"))
	var bodyHash string
	if idemKey != "" {
		bodyHash = hashBody(body)
		if s.idem.handleIdempotent(w, idemKey, bodyHash) {
			return
		}
	}

	store := s.findStore(body.Rig)
	if store == nil {
		s.idem.unreserve(idemKey)
		writeError(w, http.StatusBadRequest, "invalid", "rig is required when multiple rigs are configured")
		return
	}

	b := beads.Bead{
		Title:       body.Title,
		Description: body.Description,
		Type:        "memory",
		Labels:      body.Labels,
		Metadata: map[string]string{
			"memory.kind":         kind,
			"memory.confidence":   confidence,
			"memory.scope":        scope,
			"memory.access_count": "0",
		},
	}
	if body.DecayAt != "" {
		b.Metadata["memory.decay_at"] = body.DecayAt
	}
	if body.SourceBead != "" {
		b.Metadata["memory.source_bead"] = body.SourceBead
	}
	if body.SourceEvent != "" {
		b.Metadata["memory.source_event"] = body.SourceEvent
	}

	created, err := store.Create(b)
	if err != nil {
		s.idem.unreserve(idemKey)
		writeError(w, http.StatusInternalServerError, "internal", err.Error())
		return
	}
	resp := beadToMemoryResponse(created)
	s.idem.storeResponse(idemKey, bodyHash, http.StatusCreated, resp)
	writeJSON(w, http.StatusCreated, resp)
}

// handleMemoryGet handles GET /v0/memory/{id}.
func (s *Server) handleMemoryGet(w http.ResponseWriter, r *http.Request) {
	id := r.PathValue("id")
	for _, store := range s.beadStoresForID(id) {
		b, err := store.Get(id)
		if err != nil {
			continue
		}
		if b.Type != "memory" {
			writeError(w, http.StatusNotFound, "not_found", "memory "+id+" not found")
			return
		}
		writeIndexJSON(w, s.latestIndex(), beadToMemoryResponse(b))
		return
	}
	writeError(w, http.StatusNotFound, "not_found", "memory "+id+" not found")
}
