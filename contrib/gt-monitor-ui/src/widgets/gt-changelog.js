import { LitElement, html, css } from "lit";
import { Pollable } from "../core/fetcher.js";
import { fetchData } from "../core/fetcher.js";
import { theme } from "../core/theme.js";

/**
 * <gt-changelog endpoint="http://localhost:9090/v1">
 *
 * Timeline of completed work from /v1/changelog.
 */
export class GtChangelog extends Pollable(LitElement) {
  static styles = [
    theme,
    css`
      .timeline {
        position: relative;
        padding-left: 24px;
      }
      .timeline::before {
        content: "";
        position: absolute;
        left: 8px;
        top: 0;
        bottom: 0;
        width: 2px;
        background: var(--gt-border);
      }
      .entry {
        position: relative;
        margin-bottom: 16px;
        padding-bottom: 16px;
        border-bottom: 1px solid var(--gt-border);
      }
      .entry:last-child {
        border-bottom: none;
        margin-bottom: 0;
        padding-bottom: 0;
      }
      .entry::before {
        content: "";
        position: absolute;
        left: -20px;
        top: 4px;
        width: 10px;
        height: 10px;
        border-radius: 50%;
        background: var(--gt-accent);
        border: 2px solid var(--gt-bg);
      }
      .entry-title {
        font-weight: 600;
        margin-bottom: 4px;
      }
      .entry-meta {
        font-size: 12px;
        color: var(--gt-text-muted);
        display: flex;
        gap: 12px;
        flex-wrap: wrap;
      }
      .entry-desc {
        font-size: 13px;
        color: var(--gt-text-muted);
        margin-top: 6px;
        line-height: 1.4;
      }
      .limit-controls {
        margin-bottom: var(--gt-gap);
        font-size: 12px;
        color: var(--gt-text-muted);
      }
    `,
  ];

  static properties = {
    ...super.properties,
    limit: { type: Number },
    _entries: { state: true },
  };

  constructor() {
    super();
    this.limit = 25;
    this._entries = null;
  }

  async _fetchData() {
    const res = await fetchData(this.endpoint, "/changelog", { limit: this.limit });
    this._entries = res.data || res;
  }

  _formatDate(dateStr) {
    if (!dateStr) return "";
    const d = new Date(dateStr);
    return d.toLocaleDateString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
  }

  render() {
    if (this.loading && !this._entries) return html`<div class="gt-loading">Loading changelog...</div>`;
    if (this.error) return html`<div class="gt-error">${this.error}</div>`;
    if (!this._entries?.length) return html`<div class="gt-loading">No changelog entries</div>`;

    return html`
      <div class="gt-title">Changelog</div>
      <div class="timeline">
        ${this._entries.map(
          (e) => html`
            <div class="entry">
              <div class="entry-title">${e.title || e.summary || e.message || "Untitled"}</div>
              <div class="entry-meta">
                ${e.author ? html`<span>${e.author}</span>` : ""}
                ${e.rig ? html`<span>${e.rig}</span>` : ""}
                ${e.date || e.completed_at || e.created_at
                  ? html`<span>${this._formatDate(e.date || e.completed_at || e.created_at)}</span>`
                  : ""}
                ${e.type ? html`<span class="gt-badge gt-badge--ok">${e.type}</span>` : ""}
              </div>
              ${e.description ? html`<div class="entry-desc">${e.description}</div>` : ""}
            </div>
          `
        )}
      </div>
    `;
  }
}

customElements.define("gt-changelog", GtChangelog);
