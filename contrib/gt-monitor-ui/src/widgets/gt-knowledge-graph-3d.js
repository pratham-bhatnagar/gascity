import { LitElement, html, css } from "lit";
import { Pollable } from "../core/fetcher.js";
import { fetchData } from "../core/fetcher.js";
import { theme } from "../core/theme.js";

/**
 * <gt-knowledge-graph-3d endpoint="http://localhost:9090/v1">
 *
 * 3D force-directed knowledge graph using Three.js.
 * Visualizes relationships between beads, agents, rigs, and repos.
 *
 * NOTE: Full 3D implementation tracked separately (gc-z6d).
 * This version provides a 2D SVG force layout as the base widget.
 */
export class GtKnowledgeGraph3d extends Pollable(LitElement) {
  static styles = [
    theme,
    css`
      :host {
        min-height: 400px;
      }
      .graph-container {
        position: relative;
        width: 100%;
        height: 400px;
        overflow: hidden;
        background: var(--gt-bg-card);
        border-radius: var(--gt-radius);
      }
      .graph-container svg {
        width: 100%;
        height: 100%;
      }
      .node {
        cursor: pointer;
      }
      .node circle {
        stroke-width: 1.5;
      }
      .node text {
        font-size: 10px;
        fill: var(--gt-text);
        text-anchor: middle;
        pointer-events: none;
      }
      .link {
        stroke: var(--gt-border);
        stroke-opacity: 0.6;
        stroke-width: 1;
      }
      .legend-box {
        display: flex;
        gap: 16px;
        margin-top: 8px;
        font-size: 11px;
        color: var(--gt-text-muted);
        flex-wrap: wrap;
      }
      .legend-item {
        display: flex;
        align-items: center;
        gap: 4px;
      }
      .legend-dot {
        width: 8px;
        height: 8px;
        border-radius: 50%;
      }
      .tooltip {
        position: absolute;
        background: var(--gt-bg-card);
        border: 1px solid var(--gt-border);
        border-radius: var(--gt-radius);
        padding: 8px 12px;
        font-size: 12px;
        pointer-events: none;
        display: none;
        z-index: 10;
      }
      .tooltip.visible {
        display: block;
      }
    `,
  ];

  static properties = {
    ...super.properties,
    _nodes: { state: true },
    _links: { state: true },
    _tooltip: { state: true },
  };

  constructor() {
    super();
    this._nodes = [];
    this._links = [];
    this._tooltip = { visible: false, x: 0, y: 0, text: "" };
    this._simulation = null;
  }

  async _fetchData() {
    // Fetch multiple entity types to build the graph
    const [beadsRes, agentsRes] = await Promise.allSettled([
      fetchData(this.endpoint, "/beads", { limit: 100 }),
      fetchData(this.endpoint, "/agents", { limit: 50 }),
    ]);

    const beads = beadsRes.status === "fulfilled" ? (beadsRes.value.data || beadsRes.value) : [];
    const agents = agentsRes.status === "fulfilled" ? (agentsRes.value.data || agentsRes.value) : [];

    this._buildGraph(
      Array.isArray(beads) ? beads : [],
      Array.isArray(agents) ? agents : []
    );
  }

  _buildGraph(beads, agents) {
    const nodes = [];
    const links = [];
    const nodeMap = new Map();

    // Add agent nodes
    for (const a of agents) {
      const id = `agent:${a.name || a.id}`;
      if (!nodeMap.has(id)) {
        nodeMap.set(id, nodes.length);
        nodes.push({ id, label: a.name || a.id || "agent", type: "agent", data: a });
      }
    }

    // Add bead nodes and link to assignees
    for (const b of beads) {
      const id = `bead:${b.id}`;
      if (!nodeMap.has(id)) {
        nodeMap.set(id, nodes.length);
        nodes.push({ id, label: b.id || "bead", type: "bead", data: b });
      }

      // Link bead to assignee
      if (b.assignee || b.assigned_to) {
        const agentId = `agent:${b.assignee || b.assigned_to}`;
        if (nodeMap.has(agentId)) {
          links.push({ source: nodeMap.get(id), target: nodeMap.get(agentId) });
        }
      }

      // Link bead dependencies
      if (b.depends_on && Array.isArray(b.depends_on)) {
        for (const dep of b.depends_on) {
          const depId = `bead:${dep}`;
          if (nodeMap.has(depId)) {
            links.push({ source: nodeMap.get(id), target: nodeMap.get(depId) });
          }
        }
      }
    }

    // Run simple force layout
    this._layoutForce(nodes, links);
    this._nodes = nodes;
    this._links = links;
  }

  _layoutForce(nodes, links) {
    // Simple force-directed layout (no Three.js dep for base version)
    const w = 600, h = 400;

    // Initialize positions
    for (const n of nodes) {
      n.x = w / 2 + (Math.random() - 0.5) * w * 0.6;
      n.y = h / 2 + (Math.random() - 0.5) * h * 0.6;
      n.vx = 0;
      n.vy = 0;
    }

    // Run 100 iterations of force simulation
    for (let iter = 0; iter < 100; iter++) {
      // Repulsion between all pairs
      for (let i = 0; i < nodes.length; i++) {
        for (let j = i + 1; j < nodes.length; j++) {
          const dx = nodes[j].x - nodes[i].x;
          const dy = nodes[j].y - nodes[i].y;
          const dist = Math.sqrt(dx * dx + dy * dy) || 1;
          const force = 500 / (dist * dist);
          const fx = (dx / dist) * force;
          const fy = (dy / dist) * force;
          nodes[i].vx -= fx;
          nodes[i].vy -= fy;
          nodes[j].vx += fx;
          nodes[j].vy += fy;
        }
      }

      // Attraction along links
      for (const l of links) {
        const a = nodes[l.source], b = nodes[l.target];
        const dx = b.x - a.x;
        const dy = b.y - a.y;
        const dist = Math.sqrt(dx * dx + dy * dy) || 1;
        const force = (dist - 80) * 0.01;
        const fx = (dx / dist) * force;
        const fy = (dy / dist) * force;
        a.vx += fx;
        a.vy += fy;
        b.vx -= fx;
        b.vy -= fy;
      }

      // Center gravity
      for (const n of nodes) {
        n.vx += (w / 2 - n.x) * 0.001;
        n.vy += (h / 2 - n.y) * 0.001;
      }

      // Apply velocity with damping
      for (const n of nodes) {
        n.vx *= 0.9;
        n.vy *= 0.9;
        n.x += n.vx;
        n.y += n.vy;
        n.x = Math.max(30, Math.min(w - 30, n.x));
        n.y = Math.max(30, Math.min(h - 30, n.y));
      }
    }
  }

  _nodeColor(type) {
    const colors = {
      agent: "#58a6ff",
      bead: "#bc8cff",
      rig: "#3fb950",
      repo: "#d29922",
    };
    return colors[type] || "#8b949e";
  }

  _nodeRadius(type) {
    return type === "agent" ? 8 : 6;
  }

  _showTooltip(e, node) {
    const rect = this.shadowRoot.querySelector(".graph-container").getBoundingClientRect();
    this._tooltip = {
      visible: true,
      x: e.clientX - rect.left + 10,
      y: e.clientY - rect.top - 10,
      text: `${node.type}: ${node.label}`,
    };
  }

  _hideTooltip() {
    this._tooltip = { ...this._tooltip, visible: false };
  }

  render() {
    if (this.loading && !this._nodes.length) return html`<div class="gt-loading">Loading graph...</div>`;
    if (this.error) return html`<div class="gt-error">${this.error}</div>`;
    if (!this._nodes.length) return html`<div class="gt-loading">No graph data</div>`;

    return html`
      <div class="gt-title">Knowledge Graph</div>
      <div class="graph-container">
        <svg viewBox="0 0 600 400">
          ${this._links.map(
            (l) => html`
              <line
                class="link"
                x1="${this._nodes[l.source]?.x || 0}"
                y1="${this._nodes[l.source]?.y || 0}"
                x2="${this._nodes[l.target]?.x || 0}"
                y2="${this._nodes[l.target]?.y || 0}"
              />
            `
          )}
          ${this._nodes.map(
            (n) => html`
              <g
                class="node"
                transform="translate(${n.x}, ${n.y})"
                @mouseenter=${(e) => this._showTooltip(e, n)}
                @mouseleave=${() => this._hideTooltip()}
              >
                <circle
                  r="${this._nodeRadius(n.type)}"
                  fill="${this._nodeColor(n.type)}"
                  stroke="${this._nodeColor(n.type)}"
                  fill-opacity="0.3"
                />
                <text dy="18">${n.label.length > 12 ? n.label.slice(0, 12) + "..." : n.label}</text>
              </g>
            `
          )}
        </svg>
        <div
          class="tooltip ${this._tooltip.visible ? "visible" : ""}"
          style="left:${this._tooltip.x}px;top:${this._tooltip.y}px"
        >
          ${this._tooltip.text}
        </div>
      </div>
      <div class="legend-box">
        <div class="legend-item">
          <div class="legend-dot" style="background:#58a6ff"></div>
          Agent
        </div>
        <div class="legend-item">
          <div class="legend-dot" style="background:#bc8cff"></div>
          Bead
        </div>
        <div class="legend-item">
          <div class="legend-dot" style="background:#3fb950"></div>
          Rig
        </div>
      </div>
    `;
  }
}

customElements.define("gt-knowledge-graph-3d", GtKnowledgeGraph3d);
