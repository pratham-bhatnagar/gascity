import { LitElement, html, css } from "lit";
import { Pollable } from "../core/fetcher.js";
import { fetchData } from "../core/fetcher.js";
import { theme } from "../core/theme.js";

/**
 * <gt-command-metrics endpoint="http://localhost:9090/v1">
 *
 * Horizontal bar chart of command usage from /v1/otel/metrics
 * filtered to command-related metrics.
 */
export class GtCommandMetrics extends Pollable(LitElement) {
  static styles = [
    theme,
    css`
      .bar-chart {
        display: flex;
        flex-direction: column;
        gap: 6px;
      }
      .bar-row {
        display: grid;
        grid-template-columns: 140px 1fr 60px;
        align-items: center;
        gap: 8px;
        font-size: 13px;
      }
      .bar-label {
        font-family: var(--gt-font-mono);
        font-size: 12px;
        text-align: right;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }
      .bar-track {
        height: 18px;
        background: var(--gt-border);
        border-radius: 3px;
        overflow: hidden;
      }
      .bar-fill {
        height: 100%;
        background: var(--gt-accent);
        border-radius: 3px;
        transition: width 0.3s ease;
        min-width: 2px;
      }
      .bar-count {
        font-family: var(--gt-font-mono);
        font-size: 12px;
        color: var(--gt-text-muted);
        text-align: right;
      }
    `,
  ];

  static properties = {
    ...super.properties,
    _metrics: { state: true },
  };

  constructor() {
    super();
    this._metrics = null;
  }

  async _fetchData() {
    const res = await fetchData(this.endpoint, "/otel/metrics");
    const data = res.data || res;
    // Normalize to command-count pairs
    this._metrics = this._extractCommands(data);
  }

  _extractCommands(data) {
    if (!Array.isArray(data)) return [];
    // Attempt to find command-related metrics
    const commands = [];
    for (const m of data) {
      const name = m.name || m.command || m.metric_name || "";
      const count = m.count || m.value || m.total || 0;
      if (name && typeof count === "number") {
        commands.push({ name, count });
      }
    }
    commands.sort((a, b) => b.count - a.count);
    return commands;
  }

  render() {
    if (this.loading && !this._metrics) return html`<div class="gt-loading">Loading metrics...</div>`;
    if (this.error) return html`<div class="gt-error">${this.error}</div>`;
    if (!this._metrics?.length) return html`<div class="gt-loading">No command metrics</div>`;

    const max = Math.max(...this._metrics.map((m) => m.count), 1);

    return html`
      <div class="gt-title">Command Metrics</div>
      <div class="bar-chart">
        ${this._metrics.slice(0, 20).map(
          (m) => html`
            <div class="bar-row">
              <span class="bar-label" title="${m.name}">${m.name}</span>
              <div class="bar-track">
                <div class="bar-fill" style="width:${(m.count / max) * 100}%"></div>
              </div>
              <span class="bar-count">${m.count.toLocaleString()}</span>
            </div>
          `
        )}
      </div>
    `;
  }
}

customElements.define("gt-command-metrics", GtCommandMetrics);
