import { LitElement, html, css } from "lit";
import { Pollable } from "../core/fetcher.js";
import { fetchData } from "../core/fetcher.js";
import { theme } from "../core/theme.js";

/**
 * <gt-wasteland-board endpoint="http://localhost:9090/v1">
 *
 * Kanban-style board of wasteland items from /v1/wasteland.
 */
export class GtWastelandBoard extends Pollable(LitElement) {
  static styles = [
    theme,
    css`
      .board {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
        gap: var(--gt-gap);
      }
      .column {
        min-width: 0;
      }
      .col-header {
        font-size: 12px;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.5px;
        color: var(--gt-text-muted);
        padding: 8px 0;
        border-bottom: 2px solid var(--gt-border);
        margin-bottom: 8px;
        display: flex;
        justify-content: space-between;
      }
      .col-count {
        font-family: var(--gt-font-mono);
        color: var(--gt-accent);
      }
      .card {
        margin-bottom: 8px;
        font-size: 13px;
      }
      .card-title {
        font-weight: 500;
        margin-bottom: 4px;
        word-break: break-word;
      }
      .card-meta {
        font-size: 11px;
        color: var(--gt-text-muted);
        display: flex;
        gap: 8px;
      }
      .card-scores {
        font-family: var(--gt-font-mono);
        font-size: 11px;
        margin-top: 4px;
      }
      .card-scores span {
        margin-right: 8px;
      }
    `,
  ];

  static properties = {
    ...super.properties,
    _items: { state: true },
  };

  constructor() {
    super();
    this._items = null;
  }

  async _fetchData() {
    const res = await fetchData(this.endpoint, "/wasteland");
    this._items = res.data || res;
  }

  _groupByStatus(items) {
    const groups = { open: [], review: [], stamped: [], completed: [] };
    for (const item of items) {
      const s = (item.status || "open").toLowerCase();
      if (s.includes("complet") || s === "closed" || s === "done") {
        groups.completed.push(item);
      } else if (s.includes("stamp")) {
        groups.stamped.push(item);
      } else if (s.includes("review")) {
        groups.review.push(item);
      } else {
        groups.open.push(item);
      }
    }
    return groups;
  }

  render() {
    if (this.loading && !this._items) return html`<div class="gt-loading">Loading wasteland...</div>`;
    if (this.error) return html`<div class="gt-error">${this.error}</div>`;
    if (!this._items?.length) return html`<div class="gt-loading">No wasteland items</div>`;

    const groups = this._groupByStatus(this._items);
    const columns = [
      { label: "Open", items: groups.open },
      { label: "Review", items: groups.review },
      { label: "Stamped", items: groups.stamped },
      { label: "Completed", items: groups.completed },
    ];

    return html`
      <div class="gt-title">Wasteland Board</div>
      <div class="board">
        ${columns.map(
          (col) => html`
            <div class="column">
              <div class="col-header">
                <span>${col.label}</span>
                <span class="col-count">${col.items.length}</span>
              </div>
              ${col.items.slice(0, 20).map(
                (item) => html`
                  <div class="card gt-card">
                    <div class="card-title">${item.title || item.name || item.id || "Untitled"}</div>
                    <div class="card-meta">
                      ${item.rig ? html`<span>${item.rig}</span>` : ""}
                      ${item.priority ? html`<span>P${item.priority}</span>` : ""}
                    </div>
                    ${item.q != null || item.r != null || item.c != null
                      ? html`
                          <div class="card-scores">
                            ${item.q != null ? html`<span>Q:${item.q}</span>` : ""}
                            ${item.r != null ? html`<span>R:${item.r}</span>` : ""}
                            ${item.c != null ? html`<span>C:${item.c}</span>` : ""}
                          </div>
                        `
                      : ""}
                  </div>
                `
              )}
            </div>
          `
        )}
      </div>
    `;
  }
}

customElements.define("gt-wasteland-board", GtWastelandBoard);
