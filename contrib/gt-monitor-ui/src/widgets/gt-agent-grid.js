import { LitElement, html, css } from "lit";
import { Pollable } from "../core/fetcher.js";
import { fetchData } from "../core/fetcher.js";
import { theme } from "../core/theme.js";

/**
 * <gt-agent-grid endpoint="http://localhost:9090/v1">
 *
 * Displays active agent sessions in a card grid.
 */
export class GtAgentGrid extends Pollable(LitElement) {
  static styles = [
    theme,
    css`
      .grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
        gap: var(--gt-gap);
      }
      .agent {
        position: relative;
        overflow: hidden;
      }
      .agent-name {
        font-weight: 600;
        font-size: 15px;
        margin-bottom: 4px;
      }
      .agent-role {
        font-size: 12px;
        color: var(--gt-text-muted);
        margin-bottom: 8px;
      }
      .agent-meta {
        display: flex;
        justify-content: space-between;
        font-size: 12px;
        color: var(--gt-text-muted);
      }
      .status-dot {
        display: inline-block;
        width: 8px;
        height: 8px;
        border-radius: 50%;
        margin-right: 6px;
        vertical-align: middle;
      }
      .status-dot--active {
        background: var(--gt-ok);
        box-shadow: 0 0 6px var(--gt-ok);
      }
      .status-dot--idle {
        background: var(--gt-warn);
      }
      .status-dot--stopped {
        background: var(--gt-text-muted);
      }
    `,
  ];

  static properties = {
    ...super.properties,
    _agents: { state: true },
  };

  constructor() {
    super();
    this._agents = null;
  }

  async _fetchData() {
    const res = await fetchData(this.endpoint, "/agents");
    this._agents = res.data || res;
  }

  _statusClass(agent) {
    const s = (agent.status || agent.state || "").toLowerCase();
    if (s === "active" || s === "running") return "active";
    if (s === "idle" || s === "waiting") return "idle";
    return "stopped";
  }

  render() {
    if (this.loading && !this._agents) return html`<div class="gt-loading">Loading agents...</div>`;
    if (this.error) return html`<div class="gt-error">${this.error}</div>`;
    if (!this._agents?.length) return html`<div class="gt-loading">No active agents</div>`;

    return html`
      <div class="gt-title">Agent Grid</div>
      <div class="grid">
        ${this._agents.map(
          (a) => html`
            <div class="agent gt-card">
              <div class="agent-name">
                <span class="status-dot status-dot--${this._statusClass(a)}"></span>
                ${a.name || a.role || "agent"}
              </div>
              <div class="agent-role">${a.rig || ""}${a.role ? ` / ${a.role}` : ""}</div>
              <div class="agent-meta">
                <span>${a.provider || ""}</span>
                <span class="gt-badge gt-badge--${this._statusClass(a) === "active" ? "ok" : this._statusClass(a) === "idle" ? "warn" : "error"}">
                  ${a.status || a.state || "unknown"}
                </span>
              </div>
            </div>
          `
        )}
      </div>
    `;
  }
}

customElements.define("gt-agent-grid", GtAgentGrid);
