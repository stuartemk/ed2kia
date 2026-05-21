/**
 * ed2kIA — Public Observability Dashboard (Sprint19)
 *
 * Lightweight Alpine.js controller for public-read-only metrics.
 * Consumes: GET /api/metrics, GET /api/atlas/stats, GET /api/merit/tiers
 *
 * Features:
 * - requestAnimationFrame for smooth updates
 * - 1s debounce on API calls
 * - Lazy loading (fetch on visibility)
 * - Zero interference with Web Worker inference
 * - Cero telemetría externa
 */

document.addEventListener('alpine:init', () => {
    Alpine.data('publicDashboard', () => ({
        // ─── State ────────────────────────────────────────────────────────
        connected: false,
        lastUpdate: '—',
        refreshInterval: null,
        debounceTimer: null,

        network: {
            peers: '—',
            peersDelta: 0,
            latency: '—',
            slashing: '—',
            wasmWorkers: '—',
        },

        alignment: {
            zPositive: 60,
            zNeutral: 30,
            zNegative: 10,
            rlhfAccepted: '—',
            rlhfRejected: '—',
            bftOutlier: '—',
            sctActive: false,
        },

        merit: {
            novice: '—',
            contributor: '—',
            auditor: '—',
            steward: '—',
            guardian: '—',
            totalCorrections: '—',
        },

        // ─── API Endpoints ────────────────────────────────────────────────
        apiBase: window.location.origin,
        endpoints: {
            metrics: '/api/metrics',
            atlas: '/api/atlas/stats',
            merit: '/api/merit/tiers',
        },

        // ─── Lifecycle ────────────────────────────────────────────────────

        init() {
            this.log('Dashboard initialized');
            this.startPolling();
            this.handleVisibility();
        },

        // ─── Polling (debounced 1s) ───────────────────────────────────────

        startPolling() {
            this.fetchAll();
            // Poll every 5s using requestAnimationFrame for smooth updates
            this.refreshInterval = setInterval(() => {
                requestAnimationFrame(() => this.fetchAll());
            }, 5000);
        },

        stopPolling() {
            if (this.refreshInterval) {
                clearInterval(this.refreshInterval);
                this.refreshInterval = null;
            }
        },

        // ─── Visibility Handling (lazy loading) ──────────────────────────

        handleVisibility() {
            document.addEventListener('visibilitychange', () => {
                if (document.hidden) {
                    this.stopPolling();
                    this.log('Tab hidden — polling paused');
                } else {
                    this.startPolling();
                    this.log('Tab visible — polling resumed');
                }
            });
        },

        // ─── Data Fetching ───────────────────────────────────────────────

        async fetchAll() {
            // Debounce: wait 1s before fetching
            if (this.debounceTimer) {
                clearTimeout(this.debounceTimer);
            }

            this.debounceTimer = setTimeout(async () => {
                await Promise.allSettled([
                    this.fetchMetrics(),
                    this.fetchAtlas(),
                    this.fetchMerit(),
                ]);

                this.updateTimestamp();
            }, 1000);
        },

        async fetchMetrics() {
            try {
                const response = await this.safeFetch(this.endpoints.metrics);
                if (!response) return;

                const data = await response.json();
                this.connected = true;

                // Network metrics
                this.network.peers = data.peers ?? data.active_peers ?? '—';
                this.network.peersDelta = data.peers_delta ?? 0;
                this.network.latency = data.consensus_latency_ms ?? data.latency ?? '—';
                this.network.slashing = (data.slashing_rate ?? 0).toFixed(2);
                this.network.wasmWorkers = data.wasm_workers ?? data.edge_workers ?? '—';

                // Alignment metrics
                if (data.sct) {
                    this.alignment.zPositive = data.sct.z_positive ?? this.alignment.zPositive;
                    this.alignment.zNeutral = data.sct.z_neutral ?? this.alignment.zNeutral;
                    this.alignment.zNegative = data.sct.z_negative ?? this.alignment.zNegative;
                    this.alignment.sctActive = data.sct.active ?? false;
                }

                if (data.rlhf) {
                    this.alignment.rlhfAccepted = data.rlhf.accepted ?? '—';
                    this.alignment.rlhfRejected = data.rlhf.rejected ?? '—';
                }

                if (data.bft) {
                    this.alignment.bftOutlier = (data.bft.outlier_rate ?? 0).toFixed(2);
                }

                this.log('Metrics updated');
            } catch (err) {
                this.log('Metrics fetch failed: ' + err.message);
                this.connected = false;
            }
        },

        async fetchAtlas() {
            try {
                const response = await this.safeFetch(this.endpoints.atlas);
                if (!response) return;

                const data = await response.json();

                // Update alignment from atlas if available
                if (data.concept_density) {
                    this.log('Atlas: concept_density = ' + data.concept_density);
                }

                if (data.active_concepts) {
                    this.log('Atlas: active_concepts = ' + data.active_concepts);
                }
            } catch (err) {
                this.log('Atlas fetch failed: ' + err.message);
            }
        },

        async fetchMerit() {
            try {
                const response = await this.safeFetch(this.endpoints.merit);
                if (!response) return;

                const data = await response.json();

                // Tier counts
                if (data.tiers) {
                    this.merit.novice = data.tiers.novice ?? data.tiers.Novice ?? '—';
                    this.merit.contributor = data.tiers.contributor ?? data.tiers.Contributor ?? '—';
                    this.merit.auditor = data.tiers.auditor ?? data.tiers.Auditor ?? '—';
                    this.merit.steward = data.tiers.steward ?? data.tiers.Steward ?? '—';
                    this.merit.guardian = data.tiers.guardian ?? data.tiers.Guardian ?? '—';
                }

                this.merit.totalCorrections = data.total_corrections ?? data.human_corrections ?? '—';

                this.log('Merit updated');
            } catch (err) {
                this.log('Merit fetch failed: ' + err.message);
            }
        },

        // ─── Safe Fetch (handles offline gracefully) ─────────────────────

        async safeFetch(url) {
            const fullUrl = this.apiBase + url;
            try {
                const controller = new AbortController();
                const timeoutId = setTimeout(() => controller.abort(), 3000);

                const response = await fetch(fullUrl, {
                    signal: controller.signal,
                    headers: { 'Accept': 'application/json' },
                });

                clearTimeout(timeoutId);

                if (!response.ok) {
                    return null;
                }

                return response;
            } catch (err) {
                if (err.name === 'AbortError') {
                    this.log('Fetch timeout: ' + url);
                }
                return null;
            }
        },

        // ─── Utilities ───────────────────────────────────────────────────

        updateTimestamp() {
            const now = new Date();
            this.lastUpdate = now.toLocaleTimeString('es-MX', {
                hour: '2-digit',
                minute: '2-digit',
                second: '2-digit',
            });
        },

        log(msg) {
            // Silent by default — enable for debugging
            // console.log('[public-dashboard]', msg);
        },
    }));
});
