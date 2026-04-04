package main

import (
	"encoding/json"
	"fmt"
	"io"
	"strconv"
	"strings"
	"text/tabwriter"
	"time"

	"github.com/gastownhall/gascity/internal/beads"
	"github.com/spf13/cobra"
)

func newMemoryCmd(stdout, stderr io.Writer) *cobra.Command {
	cmd := &cobra.Command{
		Use:   "memory",
		Short: "Manage persistent agent memories",
		Long: `Manage persistent agent memories stored as beads.

Memories are beads of type "memory" with structured metadata:
  memory.kind         pattern, decision, incident, skill, context, anti-pattern
  memory.confidence   0.0–1.0 reliability score
  memory.decay_at     timestamp after which the memory should be archived
  memory.scope        agent, rig, town, global
  memory.source_bead  originating bead ID
  memory.source_event originating event ID

Use "gc memory remember" to create, "gc memory recall" to search,
and "gc memory forget" to archive stale memories.`,
		Args: cobra.ArbitraryArgs,
		RunE: func(_ *cobra.Command, args []string) error {
			if len(args) == 0 {
				fmt.Fprintln(stderr, "gc memory: missing subcommand (remember, recall, forget)") //nolint:errcheck
			} else {
				fmt.Fprintf(stderr, "gc memory: unknown subcommand %q\n", args[0]) //nolint:errcheck
			}
			return errExit
		},
	}
	cmd.AddCommand(
		newMemoryRememberCmd(stdout, stderr),
		newMemoryRecallCmd(stdout, stderr),
		newMemoryForgetCmd(stdout, stderr),
	)
	return cmd
}

func newMemoryRememberCmd(stdout, stderr io.Writer) *cobra.Command {
	var (
		kind        string
		confidence  string
		scope       string
		decayAt     string
		sourceBead  string
		sourceEvent string
		labels      []string
		jsonOutput  bool
	)
	cmd := &cobra.Command{
		Use:   "remember <title>",
		Short: "Create a memory bead",
		Long: `Create a persistent memory bead from observed knowledge.

A memory captures a pattern, decision, incident, skill, context, or
anti-pattern observed during work. Memories have confidence scores and
optional decay timestamps for automatic staleness detection.`,
		Example: `  gc memory remember "Always run tests before pushing" --kind pattern --confidence 0.9 --scope rig
  gc memory remember "Use --json flag for bd output" --kind skill --scope agent
  gc memory remember "Auth tokens expire after 1h" --kind context --decay-at 2026-05-01T00:00:00Z`,
		Args: cobra.ExactArgs(1),
		RunE: func(_ *cobra.Command, args []string) error {
			title := args[0]
			store, code := openCityStore(stderr, "gc memory")
			if code != 0 {
				return errExit
			}

			if confidence != "" {
				c, err := strconv.ParseFloat(confidence, 64)
				if err != nil || c < 0 || c > 1 {
					fmt.Fprintln(stderr, "gc memory remember: --confidence must be a number between 0.0 and 1.0") //nolint:errcheck
					return errExit
				}
			}

			b := beads.Bead{
				Title:  title,
				Type:   "memory",
				Labels: labels,
			}

			fields := MemoryFields{
				Kind:        kind,
				Confidence:  confidence,
				DecayAt:     decayAt,
				SourceBead:  sourceBead,
				SourceEvent: sourceEvent,
				Scope:       scope,
				AccessCount: "0",
			}
			applyMemoryFields(&b, fields)

			created, err := store.Create(b)
			if err != nil {
				fmt.Fprintf(stderr, "gc memory remember: %v\n", err) //nolint:errcheck
				return errExit
			}

			if jsonOutput {
				enc := json.NewEncoder(stdout)
				enc.SetIndent("", "  ")
				_ = enc.Encode(created)
			} else {
				fmt.Fprintf(stdout, "✓ Remembered: %s — %s\n", created.ID, created.Title) //nolint:errcheck
				if kind != "" {
					fmt.Fprintf(stdout, "  Kind: %s\n", kind) //nolint:errcheck
				}
				if scope != "" {
					fmt.Fprintf(stdout, "  Scope: %s\n", scope) //nolint:errcheck
				}
				if confidence != "" {
					fmt.Fprintf(stdout, "  Confidence: %s\n", confidence) //nolint:errcheck
				}
			}
			return nil
		},
	}
	cmd.Flags().StringVar(&kind, "kind", "pattern", "memory kind: pattern, decision, incident, skill, context, anti-pattern")
	cmd.Flags().StringVar(&confidence, "confidence", "", "confidence score (0.0–1.0)")
	cmd.Flags().StringVar(&scope, "scope", "rig", "scope: agent, rig, town, global")
	cmd.Flags().StringVar(&decayAt, "decay-at", "", "RFC 3339 decay timestamp")
	cmd.Flags().StringVar(&sourceBead, "source-bead", "", "originating bead ID")
	cmd.Flags().StringVar(&sourceEvent, "source-event", "", "originating event ID")
	cmd.Flags().StringSliceVarP(&labels, "label", "l", nil, "labels (repeatable)")
	cmd.Flags().BoolVar(&jsonOutput, "json", false, "output as JSON")
	return cmd
}

func newMemoryRecallCmd(stdout, stderr io.Writer) *cobra.Command {
	var (
		scope      string
		kind       string
		minConf    string
		limit      int
		jsonOutput bool
	)
	cmd := &cobra.Command{
		Use:   "recall [keyword...]",
		Short: "Search memories by scope, kind, label, or keyword",
		Long: `Search persistent memory beads.

Searches by metadata filters (scope, kind, min-confidence) and title
keyword matching. Each recalled memory has its access count bumped and
last_accessed timestamp updated.`,
		Example: `  gc memory recall                              # all memories
  gc memory recall --scope rig                   # rig-scoped only
  gc memory recall --kind pattern                # patterns only
  gc memory recall --min-confidence 0.8          # high confidence
  gc memory recall testing                       # keyword search
  gc memory recall --scope global --limit 5      # top 5 global`,
		Args: cobra.ArbitraryArgs,
		RunE: func(_ *cobra.Command, args []string) error {
			store, code := openCityStore(stderr, "gc memory")
			if code != 0 {
				return errExit
			}

			// Build metadata filters for memory beads.
			filters := map[string]string{}
			if scope != "" {
				filters["memory.scope"] = scope
			}
			if kind != "" {
				filters["memory.kind"] = kind
			}

			if limit <= 0 {
				limit = 50
			}

			// Use ListByMetadata if we have metadata filters, else ListByLabel.
			var memories []beads.Bead
			var err error
			if len(filters) > 0 {
				memories, err = store.ListByMetadata(filters, limit)
			} else {
				// Fall back to listing by type via metadata.
				// Memory beads always have memory.kind set.
				memories, err = store.ListByMetadata(map[string]string{}, limit)
			}
			if err != nil {
				fmt.Fprintf(stderr, "gc memory recall: %v\n", err) //nolint:errcheck
				return errExit
			}

			// Filter to memory type beads only.
			var filtered []beads.Bead
			for _, b := range memories {
				if b.Type != "memory" {
					continue
				}
				filtered = append(filtered, b)
			}

			// Apply keyword filter if provided.
			if len(args) > 0 {
				keyword := strings.ToLower(strings.Join(args, " "))
				var matched []beads.Bead
				for _, b := range filtered {
					if strings.Contains(strings.ToLower(b.Title), keyword) {
						matched = append(matched, b)
					}
				}
				filtered = matched
			}

			// Apply min-confidence filter.
			if minConf != "" {
				threshold, err := strconv.ParseFloat(minConf, 64)
				if err != nil {
					fmt.Fprintln(stderr, "gc memory recall: --min-confidence must be a number") //nolint:errcheck
					return errExit
				}
				var highConf []beads.Bead
				for _, b := range filtered {
					mf := getMemoryFields(b)
					if mf.Confidence != "" {
						c, err := strconv.ParseFloat(mf.Confidence, 64)
						if err == nil && c >= threshold {
							highConf = append(highConf, b)
						}
					}
				}
				filtered = highConf
			}

			// Apply limit.
			if len(filtered) > limit {
				filtered = filtered[:limit]
			}

			// Bump access count and last_accessed for each recalled memory.
			now := time.Now().UTC().Format(time.RFC3339)
			for _, b := range filtered {
				mf := getMemoryFields(b)
				count := 0
				if mf.AccessCount != "" {
					count, _ = strconv.Atoi(mf.AccessCount)
				}
				count++
				_ = setMemoryFields(store, b.ID, MemoryFields{
					LastAccessed: now,
					AccessCount:  strconv.Itoa(count),
				})
			}

			if jsonOutput {
				enc := json.NewEncoder(stdout)
				enc.SetIndent("", "  ")
				_ = enc.Encode(filtered)
				return nil
			}

			if len(filtered) == 0 {
				fmt.Fprintln(stdout, "No memories found.") //nolint:errcheck
				return nil
			}

			w := tabwriter.NewWriter(stdout, 0, 4, 2, ' ', 0)
			fmt.Fprintln(w, "ID\tKIND\tSCOPE\tCONF\tTITLE") //nolint:errcheck
			for _, b := range filtered {
				mf := getMemoryFields(b)
				conf := mf.Confidence
				if conf == "" {
					conf = "-"
				}
				mfKind := mf.Kind
				if mfKind == "" {
					mfKind = "-"
				}
				mfScope := mf.Scope
				if mfScope == "" {
					mfScope = "-"
				}
				fmt.Fprintf(w, "%s\t%s\t%s\t%s\t%s\n", b.ID, mfKind, mfScope, conf, b.Title) //nolint:errcheck
			}
			_ = w.Flush()
			return nil
		},
	}
	cmd.Flags().StringVar(&scope, "scope", "", "filter by scope: agent, rig, town, global")
	cmd.Flags().StringVar(&kind, "kind", "", "filter by kind: pattern, decision, incident, skill, context, anti-pattern")
	cmd.Flags().StringVar(&minConf, "min-confidence", "", "minimum confidence threshold (0.0–1.0)")
	cmd.Flags().IntVar(&limit, "limit", 50, "max results")
	cmd.Flags().BoolVar(&jsonOutput, "json", false, "output as JSON")
	return cmd
}

func newMemoryForgetCmd(stdout, stderr io.Writer) *cobra.Command {
	var (
		stale      bool
		jsonOutput bool
	)
	cmd := &cobra.Command{
		Use:   "forget [memory-id...]",
		Short: "Archive stale memories",
		Long: `Archive (close) memory beads that are stale or no longer useful.

By ID: closes specific memory beads.
With --stale: closes all memory beads past their decay_at timestamp.`,
		Example: `  gc memory forget gc-abc gc-def     # archive specific memories
  gc memory forget --stale           # archive all decayed memories`,
		Args: cobra.ArbitraryArgs,
		RunE: func(_ *cobra.Command, args []string) error {
			if !stale && len(args) == 0 {
				fmt.Fprintln(stderr, "gc memory forget: provide memory IDs or use --stale") //nolint:errcheck
				return errExit
			}

			store, code := openCityStore(stderr, "gc memory")
			if code != 0 {
				return errExit
			}

			var archived []string

			if stale {
				// Find all memory beads past decay_at.
				memories, err := store.ListByMetadata(map[string]string{}, 0)
				if err != nil {
					fmt.Fprintf(stderr, "gc memory forget: %v\n", err) //nolint:errcheck
					return errExit
				}
				now := time.Now().UTC()
				for _, b := range memories {
					if b.Type != "memory" || b.Status == "closed" {
						continue
					}
					mf := getMemoryFields(b)
					if mf.DecayAt == "" {
						continue
					}
					decayTime, err := time.Parse(time.RFC3339, mf.DecayAt)
					if err != nil {
						continue
					}
					if now.After(decayTime) {
						if err := store.Close(b.ID); err == nil {
							archived = append(archived, b.ID)
						}
					}
				}
			}

			// Close specific IDs.
			for _, id := range args {
				if err := store.Close(id); err != nil {
					fmt.Fprintf(stderr, "gc memory forget: closing %s: %v\n", id, err) //nolint:errcheck
					continue
				}
				archived = append(archived, id)
			}

			if jsonOutput {
				enc := json.NewEncoder(stdout)
				enc.SetIndent("", "  ")
				_ = enc.Encode(map[string]interface{}{
					"archived": archived,
					"count":    len(archived),
				})
			} else {
				if len(archived) == 0 {
					fmt.Fprintln(stdout, "No memories archived.") //nolint:errcheck
				} else {
					fmt.Fprintf(stdout, "✓ Archived %d memories: %s\n", len(archived), strings.Join(archived, ", ")) //nolint:errcheck
				}
			}
			return nil
		},
	}
	cmd.Flags().BoolVar(&stale, "stale", false, "archive all memories past their decay_at")
	cmd.Flags().BoolVar(&jsonOutput, "json", false, "output as JSON")
	return cmd
}
