import { LitElement, html, css } from "lit";
import { Pollable } from "../core/fetcher.js";
import { fetchData } from "../core/fetcher.js";
import { theme } from "../core/theme.js";

/**
 * <gt-cost-tracker endpoint="http://localhost:9090/v1">
 *
 * Displays session costs from the gt-monitor /v1/costs endpoint.
 * Shows total spend, per-session breakdown, and cost trend.
 */
export class GtCostTracker extends Pollable(LitElement) {
  static styles = [
    theme,
    css`
      .summary {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
        gap: var(--gt-gap);
        margin-bottom: var(--gt-gap);
      }
      .metric {
        text-align: center;
      }
      .metric .gt-value {
        font-size: 24px;
      }
      .sessions {
        max-height: 320px;
        overflow-y: auto;
      }
    `,
  ];

  static properties = {
    ...super.properties,
    _costs: { state: true },
  };

  constructor() {
    super();
    this._costs = null;
  }

  async _fetchData() {
    const res = await fetchData(this.endpoint, "/costs");
    this._costs = res.data || res;
  }

  _totalCost() {
    if (!this._costs?.length) return "0.00";
    const total = this._costs.reduce(
      (s, c) => s + (c.total_cost || c.cost || 0),
      0
    );
    return total.toFixed(2);
  }

  _sessionCount() {
    return this._costs?.length || 0;
  }

  render() {
    if (this.loading && !this._costs) return html`<div class="gt-loading">Loading costs...</div>`;
    if (this.error) return html`<div class="gt-error">${this.error}</div>`;
    if (!this._costs?.length) return html`<div class="gt-loading">No cost data</div>`;

    return html`
      <div class="gt-title">Cost Tracker</div>
      <div class="summary">
        <div class="metric gt-card">
          <div class="gt-title">Total Spend</div>
          <div class="gt-value">$${this._totalCost()}</div>
        </div>
        <div class="metric gt-card">
          <div class="gt-title">Sessions</div>
          <div class="gt-value">${this._sessionCount()}</div>
        </div>
      </div>
      <div class="sessions">
        <table class="gt-table">
          <thead>
            <tr>
              <th>Session</th>
              <th>Cost</th>
              <th>Tokens</th>
            </tr>
          </thead>
          <tbody>
            ${this._costs.slice(0, 50).map(
              (c) => html`
                <tr>
                  <td style="font-family:var(--gt-font-mono);font-size:12px">
                    ${c.session_id?.slice(0, 8) || c.id || "—"}
                  </td>
                  <td>$${(c.total_cost || c.cost || 0).toFixed(4)}</td>
                  <td style="color:var(--gt-text-muted)">
                    ${(c.total_tokens || c.tokens || 0).toLocaleString()}
                  </td>
                </tr>
              `
            )}
          </tbody>
        </table>
      </div>
    `;
  }
}

customElements.define("gt-cost-tracker", GtCostTracker);
