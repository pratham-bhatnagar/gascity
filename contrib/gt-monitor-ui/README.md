# gt-monitor-ui

Web Components widget library for [gt-monitor](https://github.com/gastownhall/gt-monitor) API data visualization.

Drop-in widgets — any project adds `<script type="module" src="gt-monitor-ui.js">` and gets rich monitoring widgets.

## Widgets

| Widget | Element | API Endpoint | Description |
|--------|---------|-------------|-------------|
| Cost Tracker | `<gt-cost-tracker>` | `/v1/costs` | Session costs with totals and per-session breakdown |
| Agent Grid | `<gt-agent-grid>` | `/v1/agents` | Card grid of active agent sessions |
| System Vitals | `<gt-system-vitals>` | `/v1/system` | CPU/memory/disk gauge rings |
| Commit Heatmap | `<gt-commit-heatmap>` | `/v1/commits` | GitHub-style contribution calendar |
| Changelog | `<gt-changelog>` | `/v1/changelog` | Timeline of completed work |
| Command Metrics | `<gt-command-metrics>` | `/v1/otel/metrics` | Horizontal bar chart of command usage |
| Wasteland Board | `<gt-wasteland-board>` | `/v1/wasteland` | Kanban board of wasteland items |
| Bead List | `<gt-bead-list>` | `/v1/beads` | Filterable table of issues/tasks |
| Knowledge Graph | `<gt-knowledge-graph-3d>` | `/v1/beads`, `/v1/agents` | Force-directed relationship graph |

## Usage

```html
<script type="module" src="gt-monitor-ui.js"></script>

<gt-cost-tracker endpoint="http://localhost:9090/v1"></gt-cost-tracker>
<gt-agent-grid endpoint="http://localhost:9090/v1"></gt-agent-grid>
<gt-system-vitals endpoint="http://localhost:9090/v1"></gt-system-vitals>
```

## Attributes

All widgets accept:

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `endpoint` | String | `""` | Base URL of gt-monitor API (required) |
| `interval` | Number | `30000` | Auto-refresh interval in milliseconds |

Widget-specific attributes:

| Widget | Attribute | Default | Description |
|--------|-----------|---------|-------------|
| `<gt-commit-heatmap>` | `weeks` | `20` | Number of weeks to display |
| `<gt-changelog>` | `limit` | `25` | Max entries to show |
| `<gt-bead-list>` | `status` | `""` | Pre-filter by bead status |

## Theming

Dark theme by default. Override CSS custom properties:

```css
gt-cost-tracker {
  --gt-bg: #1a1a2e;
  --gt-bg-card: #16213e;
  --gt-accent: #e94560;
  --gt-text: #eee;
}
```

Full variable list in `src/core/theme.js`.

## Build

```bash
npm install
npm run build
# Output: dist/gt-monitor-ui.js
```

## Development

```bash
# Serve demo.html with any static server:
npx serve .
# Open http://localhost:3000/demo.html
```

## Architecture

Built with [Lit](https://lit.dev/) for lightweight Web Components. Each widget:

1. Takes an `endpoint` attribute pointing at a gt-monitor API
2. Polls the API on a configurable interval (default 30s)
3. Renders data using Shadow DOM (no style leaks)
4. Supports CSS custom properties for theming

Bundled as a single ES module via Rollup.
