import { LitElement, html, css } from "lit";
import { Pollable } from "../core/fetcher.js";
import { fetchData } from "../core/fetcher.js";
import { theme } from "../core/theme.js";

/**
 * <gt-commit-heatmap endpoint="http://localhost:9090/v1">
 *
 * GitHub-style contribution heatmap from /v1/commits.
 */
export class GtCommitHeatmap extends Pollable(LitElement) {
  static styles = [
    theme,
    css`
      :host {
        overflow-x: auto;
      }
      .heatmap {
        display: flex;
        gap: 3px;
      }
      .week {
        display: flex;
        flex-direction: column;
        gap: 3px;
      }
      .day {
        width: 12px;
        height: 12px;
        border-radius: 2px;
        background: var(--gt-border);
      }
      .day--1 { background: #0e4429; }
      .day--2 { background: #006d32; }
      .day--3 { background: #26a641; }
      .day--4 { background: #39d353; }
      .day[title] { cursor: help; }
      .legend {
        display: flex;
        align-items: center;
        gap: 4px;
        margin-top: 8px;
        font-size: 11px;
        color: var(--gt-text-muted);
      }
      .legend .day { width: 10px; height: 10px; }
      .stats {
        display: flex;
        gap: var(--gt-gap);
        margin-bottom: var(--gt-gap);
      }
      .stat {
        font-family: var(--gt-font-mono);
        font-size: 13px;
      }
      .stat-label {
        font-size: 11px;
        color: var(--gt-text-muted);
      }
    `,
  ];

  static properties = {
    ...super.properties,
    weeks: { type: Number },
    _commits: { state: true },
  };

  constructor() {
    super();
    this.weeks = 20;
    this._commits = null;
  }

  async _fetchData() {
    const res = await fetchData(this.endpoint, "/commits", { limit: 5000 });
    this._commits = res.data || res;
  }

  _buildGrid() {
    const counts = new Map();
    if (this._commits) {
      for (const c of this._commits) {
        const d = (c.date || c.committed_at || c.created_at || "").slice(0, 10);
        if (d) counts.set(d, (counts.get(d) || 0) + 1);
      }
    }

    const today = new Date();
    const totalDays = this.weeks * 7;
    const grid = [];
    let week = [];

    for (let i = totalDays - 1; i >= 0; i--) {
      const d = new Date(today);
      d.setDate(d.getDate() - i);
      const key = d.toISOString().slice(0, 10);
      const count = counts.get(key) || 0;
      week.push({ date: key, count, dow: d.getDay() });

      if (week.length === 7) {
        grid.push(week);
        week = [];
      }
    }
    if (week.length) grid.push(week);
    return { grid, counts };
  }

  _level(count) {
    if (count === 0) return 0;
    if (count <= 2) return 1;
    if (count <= 5) return 2;
    if (count <= 10) return 3;
    return 4;
  }

  render() {
    if (this.loading && !this._commits) return html`<div class="gt-loading">Loading commits...</div>`;
    if (this.error) return html`<div class="gt-error">${this.error}</div>`;

    const { grid, counts } = this._buildGrid();
    const totalCommits = Array.from(counts.values()).reduce((a, b) => a + b, 0);
    const activeDays = counts.size;

    return html`
      <div class="gt-title">Commit Heatmap</div>
      <div class="stats">
        <div>
          <span class="stat">${totalCommits}</span>
          <span class="stat-label"> commits</span>
        </div>
        <div>
          <span class="stat">${activeDays}</span>
          <span class="stat-label"> active days</span>
        </div>
      </div>
      <div class="heatmap">
        ${grid.map(
          (w) => html`
            <div class="week">
              ${w.map(
                (d) => html`
                  <div
                    class="day day--${this._level(d.count)}"
                    title="${d.date}: ${d.count} commit${d.count !== 1 ? "s" : ""}"
                  ></div>
                `
              )}
            </div>
          `
        )}
      </div>
      <div class="legend">
        Less
        <div class="day"></div>
        <div class="day day--1"></div>
        <div class="day day--2"></div>
        <div class="day day--3"></div>
        <div class="day day--4"></div>
        More
      </div>
    `;
  }
}

customElements.define("gt-commit-heatmap", GtCommitHeatmap);
