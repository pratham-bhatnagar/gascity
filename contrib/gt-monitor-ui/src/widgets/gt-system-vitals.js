import { LitElement, html, css } from "lit";
import { Pollable } from "../core/fetcher.js";
import { theme } from "../core/theme.js";

export class GtSystemVitals extends Pollable(LitElement) {
  static styles = [
    theme,
    css`
      .vitals { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
      .vital { text-align: center; padding: 16px; background: var(--gt-bg-card); border-radius: var(--gt-radius); }
      .vital-value { font-size: 24px; font-weight: 700; color: var(--gt-accent); }
      .vital-label { font-size: 11px; color: var(--gt-text-muted); text-transform: uppercase; margin-top: 4px; }
      .health-ok { color: var(--gt-success); }
      .health-warn { color: var(--gt-warn); }
      .health-error { color: var(--gt-error); }
    `,
  ];

  static properties = {
    ...super.properties,
    _health: { state: true },
  };

  constructor() {
    super();
    this._health = null;
  }

  async _fetchData() {
    const res = await fetch(`${this.endpoint}/health`);
    if (!res.ok) throw new Error(`${res.status}: ${res.statusText}`);
    const json = await res.json();
    this._health = json.data;
  }

  render() {
    if (this.loading && !this._health) return html`<div class="gt-loading">Loading vitals...</div>`;
    if (this.error) return html`<div class="gt-error">${this.error}</div>`;
    if (!this._health) return html`<div class="gt-loading">No data</div>`;

    const providers = this._health.providers || [];
    const healthy = providers.filter(p => p.status === 'healthy').length;
    const total = providers.length;

    return html`
      <div class="gt-title">System Vitals</div>
      <div class="vitals">
        <div class="vital">
          <div class="vital-value">${total}</div>
          <div class="vital-label">Providers</div>
        </div>
        <div class="vital">
          <div class="vital-value health-ok">${healthy}</div>
          <div class="vital-label">Healthy</div>
        </div>
        <div class="vital">
          <div class="vital-value ${healthy === total ? 'health-ok' : 'health-warn'}">${total > 0 ? Math.round((healthy/total)*100) : 0}%</div>
          <div class="vital-label">Uptime</div>
        </div>
      </div>
      <div style="margin-top:12px;font-size:12px;color:var(--gt-text-muted)">
        Town: ${this._health.town_id}
      </div>
    `;
  }
}

customElements.define("gt-system-vitals", GtSystemVitals);
