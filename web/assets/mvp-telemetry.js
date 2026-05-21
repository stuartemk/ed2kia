/**
 * ed2kIA MVP Telemetry — Alpine.js Component
 *
 * Reads mvp-telemetry.json or GET /api/mvp/status
 * Zero external dependencies, compatible with file:// and http://localhost
 */

function mvpTelemetry() {
    return {
        data: null,
        loading: true,
        success: false,
        source: 'none',
        pollInterval: null,

        async init() {
            await this.loadData();
            // Poll every 5s if data loaded from API
            if (this.source === 'api') {
                this.startPolling();
            }
        },

        async loadData() {
            this.loading = true;
            // Try API first
            try {
                const resp = await fetch('/api/mvp/status', { signal: AbortSignal.timeout(2000) });
                if (resp.ok) {
                    this.data = await resp.json();
                    this.success = this.data.success || false;
                    this.source = 'api';
                    this.loading = false;
                    return;
                }
            } catch (e) {
                // API not available, try local file
            }

            // Try local JSON file
            try {
                const resp = await fetch('mvp-telemetry.json', { signal: AbortSignal.timeout(2000) });
                if (resp.ok) {
                    this.data = await resp.json();
                    this.success = this.data.success || false;
                    this.source = 'file';
                    this.loading = false;
                    return;
                }
            } catch (e) {
                // File not available
            }

            // Apply mock data for demo
            this.applyMockData();
            this.loading = false;
        },

        applyMockData() {
            this.data = {
                dry_run: true,
                total_duration_ms: 12.5,
                success: true,
                timestamp: new Date().toISOString(),
                consensus: {
                    total_payloads: 3,
                    approved_count: 2,
                    rejected_count: 1,
                    bft_converged: true,
                    total_latency_ms: 8.3,
                },
                nodes: [
                    {
                        id: 'alpha',
                        address: '/ip4/127.0.0.1/tcp/8001',
                        profile: 'Symbiotic',
                        final_state: 'Active',
                    },
                    {
                        id: 'beta',
                        address: '/ip4/127.0.0.1/tcp/8002',
                        profile: 'Perverse',
                        final_state: 'Slashed',
                    },
                    {
                        id: 'gamma',
                        address: '/ip4/127.0.0.1/tcp/8003',
                        profile: 'Symbiotic',
                        final_state: 'Active',
                    },
                ],
            };
            this.success = true;
            this.source = 'mock';
        },

        startPolling() {
            this.pollInterval = setInterval(() => {
                this.loadData();
            }, 5000);
        },

        stopPolling() {
            if (this.pollInterval) {
                clearInterval(this.pollInterval);
            }
        },
    };
}

// Setup visibility API for lazy loading
document.addEventListener('visibilitychange', () => {
    const component = document.querySelector('[x-data]');
    if (component && component.__x) {
        const ctx = component.__x.getUnobservedPeekingScope();
        if (document.hidden) {
            ctx.stopPolling?.();
        } else {
            ctx.loadData?.();
        }
    }
});
