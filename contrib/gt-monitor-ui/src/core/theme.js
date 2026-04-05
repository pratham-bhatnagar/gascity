import { css } from "lit";

/**
 * Shared dark-theme styles exposed as CSS custom properties.
 *
 * Consumers can override any variable on a parent element:
 *
 *   gt-cost-tracker {
 *     --gt-bg: #1a1a2e;
 *   }
 */
export const theme = css`
  :host {
    /* Surfaces */
    --gt-bg: #0d1117;
    --gt-bg-card: #161b22;
    --gt-bg-hover: #1c2333;
    --gt-border: #30363d;

    /* Text */
    --gt-text: #e6edf3;
    --gt-text-muted: #8b949e;
    --gt-text-link: #58a6ff;

    /* Semantic */
    --gt-ok: #3fb950;
    --gt-warn: #d29922;
    --gt-error: #f85149;
    --gt-info: #58a6ff;

    /* Accent */
    --gt-accent: #bc8cff;
    --gt-accent-dim: #6e40c9;

    /* Sizing */
    --gt-radius: 6px;
    --gt-gap: 12px;
    --gt-font: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial,
      sans-serif, "Apple Color Emoji", "Segoe UI Emoji";
    --gt-font-mono: "SFMono-Regular", Consolas, "Liberation Mono", Menlo,
      monospace;
    --gt-font-size: 14px;

    display: block;
    font-family: var(--gt-font);
    font-size: var(--gt-font-size);
    color: var(--gt-text);
    background: var(--gt-bg);
    border-radius: var(--gt-radius);
    border: 1px solid var(--gt-border);
    padding: var(--gt-gap);
    box-sizing: border-box;
  }

  .gt-card {
    background: var(--gt-bg-card);
    border: 1px solid var(--gt-border);
    border-radius: var(--gt-radius);
    padding: var(--gt-gap);
  }

  .gt-title {
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--gt-text-muted);
    margin: 0 0 var(--gt-gap) 0;
  }

  .gt-value {
    font-size: 28px;
    font-weight: 700;
    font-family: var(--gt-font-mono);
  }

  .gt-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 500;
  }

  .gt-badge--ok {
    background: rgba(63, 185, 80, 0.15);
    color: var(--gt-ok);
  }
  .gt-badge--warn {
    background: rgba(210, 153, 34, 0.15);
    color: var(--gt-warn);
  }
  .gt-badge--error {
    background: rgba(248, 81, 73, 0.15);
    color: var(--gt-error);
  }

  .gt-loading {
    color: var(--gt-text-muted);
    font-style: italic;
    text-align: center;
    padding: 24px;
  }

  .gt-error {
    color: var(--gt-error);
    font-size: 13px;
    padding: 12px;
    background: rgba(248, 81, 73, 0.1);
    border-radius: var(--gt-radius);
  }

  .gt-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  .gt-table th {
    text-align: left;
    color: var(--gt-text-muted);
    font-weight: 600;
    padding: 8px 12px;
    border-bottom: 1px solid var(--gt-border);
  }
  .gt-table td {
    padding: 8px 12px;
    border-bottom: 1px solid var(--gt-border);
  }
  .gt-table tr:hover td {
    background: var(--gt-bg-hover);
  }
`;
