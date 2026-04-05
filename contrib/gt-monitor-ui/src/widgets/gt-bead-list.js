import { LitElement, html, css } from "lit";
import { Pollable } from "../core/fetcher.js";
import { fetchData } from "../core/fetcher.js";
import { theme } from "../core/theme.js";

/**
 * <gt-bead-list endpoint="http://localhost:9090/v1">
 *
 * Filterable table of beads (issues/tasks) from /v1/beads.
 */
export class GtBeadList extends Pollable(LitElement) {
  static styles = [
    theme,
    css`
      .filters {
        display: flex;
        gap: 8px;
        margin-bottom: var(--gt-gap);
        flex-wrap: wrap;
      }
      .filter-btn {
        background: none;
        border: 1px solid var(--gt-border);
        border-radius: var(--gt-radius);
        color: var(--gt-text-muted);
        padding: 4px 10px;
        font-size: 12px;
        cursor: pointer;
        font-family: var(--gt-font);
      }
      .filter-btn:hover {
        background: var(--gt-bg-hover);
      }
      .filter-btn--active {
        background: var(--gt-accent-dim);
        color: var(--gt-text);
        border-color: var(--gt-accent);
      }
      .bead-id {
        font-family: var(--gt-font-mono);
        font-size: 12px;
        color: var(--gt-text-link);
      }
      .priority {
        font-family: var(--gt-font-mono);
        font-size: 12px;
      }
      .priority--0, .priority--1 {
        color: var(--gt-error);
        font-weight: 700;
      }
      .priority--2 {
        color: var(--gt-warn);
      }
      .priority--3, .priority--4 {
        color: var(--gt-text-muted);
      }
      .empty {
        text-align: center;
        padding: 24px;
        color: var(--gt-text-muted);
      }
    `,
  ];

  static properties = {
    ...super.properties,
    status: { type: String },
    _beads: { state: true },
    _filter: { state: true },
  };

  constructor() {
    super();
    this.status = "";
    this._beads = null;
    this._filter = "all";
  }

  async _fetchData() {
    const params = {};
    if (this.status) params["filter.status"] = this.status;
    const res = await fetchData(this.endpoint, "/beads", params);
    this._beads = res.data || res;
  }

  _filtered() {
    if (!this._beads) return [];
    if (this._filter === "all") return this._beads;
    return this._beads.filter(
      (b) => (b.status || "").toLowerCase() === this._filter
    );
  }

  _setFilter(f) {
    this._filter = f;
  }

  _statusBadge(status) {
    const s = (status || "").toLowerCase();
    if (s === "closed") return "ok";
    if (s === "blocked" || s === "deferred") return "warn";
    if (s === "open" || s === "in_progress") return "error";
    return "ok";
  }

  render() {
    if (this.loading && !this._beads) return html`<div class="gt-loading">Loading beads...</div>`;
    if (this.error) return html`<div class="gt-error">${this.error}</div>`;

    const statuses = ["all", "open", "in_progress", "blocked", "closed"];
    const filtered = this._filtered();

    return html`
      <div class="gt-title">Beads</div>
      <div class="filters">
        ${statuses.map(
          (s) => html`
            <button
              class="filter-btn ${this._filter === s ? "filter-btn--active" : ""}"
              @click=${() => this._setFilter(s)}
            >
              ${s === "all" ? `All (${this._beads?.length || 0})` : s.replace("_", " ")}
            </button>
          `
        )}
      </div>
      ${filtered.length === 0
        ? html`<div class="empty">No beads matching filter</div>`
        : html`
            <table class="gt-table">
              <thead>
                <tr>
                  <th>ID</th>
                  <th>Title</th>
                  <th>P</th>
                  <th>Status</th>
                  <th>Assignee</th>
                </tr>
              </thead>
              <tbody>
                ${filtered.slice(0, 100).map(
                  (b) => html`
                    <tr>
                      <td><span class="bead-id">${b.id || "—"}</span></td>
                      <td>${b.title || "Untitled"}</td>
                      <td>
                        <span class="priority priority--${b.priority ?? 3}">
                          P${b.priority ?? "—"}
                        </span>
                      </td>
                      <td>
                        <span class="gt-badge gt-badge--${this._statusBadge(b.status)}">
                          ${b.status || "unknown"}
                        </span>
                      </td>
                      <td style="font-size:12px;color:var(--gt-text-muted)">
                        ${b.assignee || b.assigned_to || "—"}
                      </td>
                    </tr>
                  `
                )}
              </tbody>
            </table>
          `}
    `;
  }
}

customElements.define("gt-bead-list", GtBeadList);
