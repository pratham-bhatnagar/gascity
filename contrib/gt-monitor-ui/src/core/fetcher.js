/**
 * Shared data fetcher with auto-refresh for gt-monitor widgets.
 *
 * Each widget calls `startPolling(endpoint, callback, interval)` on
 * connectedCallback and `stopPolling()` on disconnectedCallback.
 */

const DEFAULT_INTERVAL = 30_000; // 30 seconds

export function buildUrl(endpoint, path, params = {}) {
  const base = endpoint.replace(/\/+$/, "");
  const url = new URL(`${base}${path}`);
  for (const [k, v] of Object.entries(params)) {
    if (v != null) url.searchParams.set(k, String(v));
  }
  return url.toString();
}

export async function fetchData(endpoint, path, params = {}) {
  const url = buildUrl(endpoint, path, params);
  const res = await fetch(url);
  if (!res.ok) throw new Error(`gt-monitor ${res.status}: ${res.statusText}`);
  return res.json();
}

/**
 * Mixin that adds polling lifecycle to a LitElement.
 *
 *   class MyWidget extends Pollable(LitElement) { ... }
 *
 * Subclass must define `_fetchData()` returning a Promise.
 */
export const Pollable = (superClass) =>
  class extends superClass {
    static properties = {
      ...superClass.properties,
      endpoint: { type: String },
      interval: { type: Number },
      loading: { type: Boolean, state: true },
      error: { type: String, state: true },
    };

    constructor() {
      super();
      this.endpoint = "";
      this.interval = DEFAULT_INTERVAL;
      this.loading = true;
      this.error = "";
      this._timer = null;
    }

    connectedCallback() {
      super.connectedCallback();
      this._poll();
    }

    disconnectedCallback() {
      super.disconnectedCallback();
      this._stopPoll();
    }

    async _poll() {
      try {
        this.loading = true;
        this.error = "";
        await this._fetchData();
      } catch (e) {
        this.error = e.message;
      } finally {
        this.loading = false;
      }
      this._timer = setTimeout(() => this._poll(), this.interval);
    }

    _stopPoll() {
      if (this._timer) {
        clearTimeout(this._timer);
        this._timer = null;
      }
    }

    /** Override in subclass: fetch data and update reactive properties. */
    async _fetchData() {
      throw new Error("_fetchData() not implemented");
    }
  };
