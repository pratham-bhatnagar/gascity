/**
 * gt-monitor-ui — Web Components widget library for gt-monitor API.
 *
 * Usage:
 *   <script type="module" src="gt-monitor-ui.js"></script>
 *   <gt-cost-tracker endpoint="http://localhost:9090/v1"></gt-cost-tracker>
 *
 * All widgets accept:
 *   - endpoint: base URL of the gt-monitor API (required)
 *   - interval: polling interval in ms (default 30000)
 *
 * Theming: override CSS custom properties on the widget or a parent.
 *   See src/core/theme.js for the full list of --gt-* variables.
 */

export { GtCostTracker } from "./widgets/gt-cost-tracker.js";
export { GtAgentGrid } from "./widgets/gt-agent-grid.js";
export { GtSystemVitals } from "./widgets/gt-system-vitals.js";
export { GtCommitHeatmap } from "./widgets/gt-commit-heatmap.js";
export { GtChangelog } from "./widgets/gt-changelog.js";
export { GtCommandMetrics } from "./widgets/gt-command-metrics.js";
export { GtWastelandBoard } from "./widgets/gt-wasteland-board.js";
export { GtBeadList } from "./widgets/gt-bead-list.js";
export { GtKnowledgeGraph3d } from "./widgets/gt-knowledge-graph-3d.js";
