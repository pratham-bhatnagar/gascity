# gt-monitor-ui Web Components Library - Complete Guide

Comprehensive UI component library for Gas Town monitoring dashboards.

---

## Quick Start

### Installation

```bash
# From local build
cp dist/gt-monitor-ui.js /your/project/
```

```html
<!-- In your HTML -->
<script type="module" src="gt-monitor-ui.js"></script>
<gt-bead-list endpoint="http://localhost:9090/v1"></gt-bead-list>
```

---

## Widgets

### 1. `<gt-bead-list>` - Beads/Issues Table

Displays filterable table of beads from the monitoring API.

**Attributes:**
| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `endpoint` | string | `""` | gt-monitor API base URL |
| `status` | string | `""` | Pre-filter by status |
| `interval` | number | `30000` | Refresh interval (ms) |

**Example:**
```html
<gt-bead-list 
  endpoint="http://localhost:9090/v1"
  status="open">
</gt-bead-list>
```

---

### 2. `<gt-system-vitals>` - System Health Dashboard

Shows provider health status and uptime metrics.

**Attributes:**
| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `endpoint` | string | `""` | gt-monitor API base URL |
| `interval` | number | `30000` | Refresh interval (ms) |

**Example:**
```html
<gt-system-vitals endpoint="http://localhost:9090/v1"></gt-system-vitals>
```

---

### 3. `<gt-agent-grid>` - Agent Session Cards

Grid of active agent sessions with status.

**Attributes:**
| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `endpoint` | string | `""` | gt-monitor API base URL |
| `interval` | number | `30000` | Refresh interval (ms) |

---

### 4. `<gt-cost-tracker>` - Cost Visualization

Session costs with totals and breakdowns.

**Attributes:**
| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `endpoint` | string | `""` | gt-monitor API base URL |
| `period` | string | `"today"` | Time period |

---

### 5. `<gt-commit-heatmap>` - Git Activity

GitHub-style contribution calendar.

**Attributes:**
| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `endpoint` | string | `""` | gt-monitor API base URL |
| `weeks` | number | `20` | Number of weeks to display |

---

### 6. `<gt-changelog>` - Completed Work Timeline

Timeline of completed work.

**Attributes:**
| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `endpoint` | string | `""` | gt-monitor API base URL |
| `limit` | number | `25` | Max entries |

---

### 7. `<gt-command-metrics>` - Command Usage

Horizontal bar chart of command usage.

**Attributes:**
| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `endpoint` | string | `""` | gt-monitor API base URL |

---

### 8. `<gt-wasteland-board>` - Wasteland Kanban

Kanban board of wasteland items.

**Attributes:**
| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `endpoint` | string | `""` | gt-monitor API base URL |

---

### 9. `<gt-knowledge-graph-3d>` - Force-Directed Graph

Three.js 3D knowledge relationship graph.

**Attributes:**
| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `endpoint` | string | `""` | gt-monitor API base URL |

---

## Theming

### CSS Custom Properties

```css
/* Dark theme (default) */
gt-bead-list {
  --gt-bg: #1a1a2e;
  --gt-bg-card: #16213e;
  --gt-bg-hover: #0f3460;
  --gt-accent: #e94560;
  --gt-accent-dim: rgba(233, 69, 96, 0.2);
  --gt-text: #eee;
  --gt-text-muted: #8b949e;
  --gt-text-link: #58a6ff;
  --gt-border: #30363d;
  --gt-success: #22c55e;
  --gt-warn: #f59e0b;
  --gt-error: #ef4444;
  --gt-radius: 6px;
  --gt-gap: 12px;
  --gt-font: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  --gt-font-mono: "SF Mono", Monaco, monospace;
}
```

---

## API Requirements

The widgets expect a gt-monitor API server running with these endpoints:

### Required Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v1/health` | GET | Provider health status |
| `/v1/capabilities` | GET | Available capabilities |
| `/v1/query` | POST | Execute queries |

### Query Request Format

```json
{
  "capability": "beads",
  "limit": 50,
  "offset": 0
}
```

### Response Format

```json
{
  "data": [...],
  "meta": {
    "town_id": "gt-dev",
    "total": 100,
    "provider": "dolt",
    "latency_ms": 12
  }
}
```

---

## Full Example

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>GT Monitor Dashboard</title>
  <script type="module" src="gt-monitor-ui.js"></script>
  <style>
    body {
      background: #010409;
      color: #e6edf3;
      font-family: sans-serif;
      padding: 24px;
    }
    .grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
      gap: 16px;
    }
  </style>
</head>
<body>
  <h1>Gas Town Monitor</h1>
  <div class="grid">
    <gt-system-vitals endpoint="http://localhost:9090/v1"></gt-system-vitals>
    <gt-bead-list endpoint="http://localhost:9090/v1"></gt-bead-list>
    <gt-agent-grid endpoint="http://localhost:9090/v1"></gt-agent-grid>
  </div>
</body>
</html>
```

---

## Development

```bash
# Install dependencies
npm install

# Build
npm run build

# Watch mode
npm run dev

# Serve demo
npx serve . -p 3000
# Open http://localhost:3000/demo.html
```

---

*Generated with Deepwork Intelligence (MiniMax-M2.5)*
