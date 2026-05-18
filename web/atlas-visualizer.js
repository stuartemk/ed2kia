/**
 * Atlas 3D Visualizer — Vanilla/ES6 module for semantic graph visualization.
 *
 * Uses 3d-force-graph (WebGL) to render the semantic graph from the Rosetta API.
 * Nodes are sized by weight, edges show activation strength via opacity.
 * Camera flies to queried nodes with smooth transitions.
 */
(function () {
    'use strict';

    const API_BASE = window.ATLAS_API_BASE || '';
    const GRAPH_URL = 'https://unpkg.com/3d-force-graph@1/3d-force-graph.min.js';

    // ─── DOM refs ──────────────────────────────────────────────────────────────
    const canvas = document.getElementById('atlas-canvas');
    const searchInput = document.getElementById('search-input');
    const loadingEl = document.getElementById('loading');
    const nodeCountEl = document.getElementById('node-count');
    const edgeCountEl = document.getElementById('edge-count');

    // ─── State ─────────────────────────────────────────────────────────────────
    let Graph = null;
    let graphRef = null;
    let graphData = { nodes: [], links: [] };

    // ─── Dynamic script load ──────────────────────────────────────────────────
    function loadScript(src) {
        return new Promise((resolve, reject) => {
            const script = document.createElement('script');
            script.src = src;
            script.async = true;
            script.onload = resolve;
            script.onerror = reject;
            document.head.appendChild(script);
        });
    }

    // ─── Init ──────────────────────────────────────────────────────────────────
    async function init() {
        try {
            await loadScript(GRAPH_URL);
            Graph = window.ForceGraph3D;
            graphRef = new Graph()
                .canvas(canvas)
                .nodeLabel('label')
                .nodeColor(nodeColor)
                .nodeRelSize(4)
                .linkWidth(linkWidth)
                .linkOpacity(linkOpacity)
                .linkColor(() => '#3a3a5e')
                .linkDirectionalParticles(2)
                .linkDirectionalParticleWidth(1)
                .linkDirectionalParticleColor(() => '#00d4ff')
                .onNodeClick(onNodeClick)
                .d3Force('link', (d3) => d3.forceLink().distance(120))
                .d3Force('charge', (d3) => d3.forceManyBody().strength(-200))
                .graphData(graphData);

            loadingEl.style.display = 'none';
            fetchGraphStats();
            setupSearch();
        } catch (err) {
            loadingEl.textContent = 'Failed to load visualizer: ' + err.message;
            console.error('Atlas init error:', err);
        }
    }

    // ─── Node styling ─────────────────────────────────────────────────────────
    function nodeColor(node) {
        return node.type === 'Token' ? '#00d4ff' : '#ff6b6b';
    }

    function linkWidth(link) {
        return Math.max(0.5, Math.min(3, Math.abs(link.weight || 1)));
    }

    function linkOpacity(link) {
        return Math.max(0.15, Math.min(0.8, Math.abs(link.weight || 0.5)));
    }

    // ─── Camera fly-to ────────────────────────────────────────────────────────
    function onNodeClick(node) {
        if (!graphRef) return;
        const sprite = graphRef.nodeThreeObject(node);
        if (!sprite) return;
        const pos = sprite.position;
        graphRef.cameraPosition(
            { x: pos.x + 80, y: pos.y + 60, z: pos.z + 80 },
            { x: pos.x, y: pos.y, z: pos.z },
            1200
        );
    }

    // ─── Fetch helpers ────────────────────────────────────────────────────────
    async function fetchGraphStats() {
        try {
            const res = await fetch(`${API_BASE}/api/atlas/stats`);
            if (!res.ok) return;
            const data = await res.json();
            nodeCountEl.textContent = data.node_count || 0;
            edgeCountEl.textContent = data.edge_count || 0;
        } catch (e) {
            // Stats endpoint may not be available
        }
    }

    async function fetchForQuery(query) {
        try {
            // Try as feature first
            let res = await fetch(`${API_BASE}/api/feature/${encodeURIComponent(query)}`);
            if (res.ok) {
                const data = await res.json();
                return buildSubGraph(data, 'feature');
            }
            // Try as token
            res = await fetch(`${API_BASE}/api/token/${encodeURIComponent(query)}`);
            if (res.ok) {
                const data = await res.json();
                return buildSubGraph(data, 'token');
            }
        } catch (e) {
            console.warn('Fetch error:', e);
        }
        return null;
    }

    function buildSubGraph(data, mode) {
        const nodes = [];
        const links = [];
        const seen = new Set();

        if (mode === 'feature') {
            // Center node: feature
            const centerId = data.feature_id || 'unknown';
            if (!seen.has(centerId)) {
                nodes.push({ id: centerId, label: centerId, type: 'Feature', weight: 1.0 });
                seen.add(centerId);
            }
            (data.top_tokens || []).forEach((entry) => {
                if (!seen.has(entry.token)) {
                    nodes.push({ id: entry.token, label: entry.token, type: 'Token', weight: entry.weight });
                    seen.add(entry.token);
                }
                links.push({
                    source: centerId,
                    target: entry.token,
                    weight: entry.weight,
                });
            });
        } else {
            // Center node: token
            const centerId = data.token || 'unknown';
            if (!seen.has(centerId)) {
                nodes.push({ id: centerId, label: centerId, type: 'Token', weight: 1.0 });
                seen.add(centerId);
            }
            (data.top_features || []).forEach((entry) => {
                if (!seen.has(entry.feature_id)) {
                    nodes.push({ id: entry.feature_id, label: entry.feature_id, type: 'Feature', weight: entry.weight });
                    seen.add(entry.feature_id);
                }
                links.push({
                    source: centerId,
                    target: entry.feature_id,
                    weight: entry.weight,
                });
            });
        }

        return { nodes, links };
    }

    // ─── Search ───────────────────────────────────────────────────────────────
    let debounceTimer = null;
    function setupSearch() {
        if (!searchInput) return;
        searchInput.addEventListener('input', () => {
            clearTimeout(debounceTimer);
            debounceTimer = setTimeout(() => {
                const query = searchInput.value.trim();
                if (query.length < 2) return;
                performSearch(query);
            }, 350);
        });
        searchInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                clearTimeout(debounceTimer);
                const query = searchInput.value.trim();
                if (query.length >= 2) performSearch(query);
            }
        });
    }

    async function performSearch(query) {
        loadingEl.textContent = 'Searching...';
        loadingEl.style.display = 'block';
        const subGraph = await fetchForQuery(query);
        loadingEl.style.display = 'none';
        if (!subGraph || subGraph.nodes.length === 0) {
            return;
        }
        if (!graphRef) return;
        graphRef.graphData(subGraph);

        // Fly to center node
        const center = subGraph.nodes[0];
        if (center) {
            const sprite = graphRef.nodeThreeObject(center);
            if (sprite) {
                const pos = sprite.position;
                graphRef.cameraPosition(
                    { x: pos.x + 100, y: pos.y + 80, z: pos.z + 100 },
                    { x: pos.x, y: pos.y, z: pos.z },
                    1500
                );
            }
        }
    }

    // ─── Boot ─────────────────────────────────────────────────────────────────
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
