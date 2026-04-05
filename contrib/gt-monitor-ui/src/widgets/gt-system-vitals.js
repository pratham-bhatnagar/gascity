import { LitElement, html, css } from "lit";
import { Pollable } from "../core/fetcher.js";
import { fetchData } from "../core/fetcher.js";
import { theme } from "../core/theme.js";

/**
 * <gt-system-vitals endpoint="http://localhost:9090/v1">
 *
 * Shows CPU, memory, disk gauges from /v1/system.
 */
export class GtSystemVitals extends Pollable(LitElement) {
  static styles = [
    theme,
    css`
      .gauges {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
        gap: var(--gt-gap);
      }
      .gauge {
        text-align: center;
      }
      .gauge-ring {
        position: relative;
        width: 100px;
        height: 100px;
        margin: 0 auto 8px;
      }
      .gauge-ring svg {
        transform: rotate(-90deg);
      }
      .gauge-ring .value {
        position: absolute;
        inset: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        font-family: var(--gt-font-mono);
        font-size: 18px;
        font-weight: 700;
      }
      .gauge-label {
        font-size: 12px;
        color: var(--gt-text-muted);
        text-transform: uppercase;
        letter-spacing: 0.5px;
      }
      .gauge-detail {
        font-size: 11px;
        color: var(--gt-text-muted);
        margin-top: 4px;
      }
    `,
  ];

  static properties = {
    ...super.properties,
    _vitals: { state: true },
  };

  constructor() {
    super();
    this._vitals = null;
  }

  async _fetchData() {
    const res = await fetchData(this.endpoint, "/system");
    this._vitals = res.data || res;
  }

  _gaugeColor(pct) {
    if (pct > 90) return "var(--gt-error)";
    if (pct > 70) return "var(--gt-warn)";
    return "var(--gt-ok)";
  }

  _renderGauge(label, pct, detail = "") {
    const r = 42;
    const circ = 2 * Math.PI * r;
    const offset = circ * (1 - pct / 100);
    const color = this._gaugeColor(pct);

    return html`
      <div class="gauge gt-card">
        <div class="gauge-ring">
          <svg width="100" height="100" viewBox="0 0 100 100">
            <circle cx="50" cy="50" r="${r}" fill="none" stroke="var(--gt-border)" stroke-width="6" />
            <circle
              cx="50" cy="50" r="${r}" fill="none"
              stroke="${color}" stroke-width="6"
              stroke-dasharray="${circ}" stroke-dashoffset="${offset}"
              stroke-linecap="round"
            />
          </svg>
          <div class="value" style="color:${color}">${Math.round(pct)}%</div>
        </div>
        <div class="gauge-label">${label}</div>
        ${detail ? html`<div class="gauge-detail">${detail}</div>` : ""}
      </div>
    `;
  }

  render() {
    if (this.loading && !this._vitals) return html`<div class="gt-loading">Loading vitals...</div>`;
    if (this.error) return html`<div class="gt-error">${this.error}</div>`;

    const v = Array.isArray(this._vitals) ? this._vitals[0] : this._vitals;
    if (!v) return html`<div class="gt-loading">No system data</div>`;

    const cpuPct = v.cpu_percent ?? v.cpu ?? 0;
    const memPct = v.memory_percent ?? v.mem ?? 0;
    const diskPct = v.disk_percent ?? v.disk ?? 0;

    const memDetail = v.memory_used && v.memory_total
      ? `${(v.memory_used / 1e9).toFixed(1)}G / ${(v.memory_total / 1e9).toFixed(1)}G`
      : "";
    const diskDetail = v.disk_used && v.disk_total
      ? `${(v.disk_used / 1e9).toFixed(0)}G / ${(v.disk_total / 1e9).toFixed(0)}G`
      : "";

    return html`
      <div class="gt-title">System Vitals</div>
      <div class="gauges">
        ${this._renderGauge("CPU", cpuPct)}
        ${this._renderGauge("Memory", memPct, memDetail)}
        ${this._renderGauge("Disk", diskPct, diskDetail)}
      </div>
    `;
  }
}

customElements.define("gt-system-vitals", GtSystemVitals);
