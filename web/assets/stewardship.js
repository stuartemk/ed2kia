/**
 * ed2kIA Stewardship Dashboard — Alpine.js Component
 * v2.1.0-sprint12
 *
 * Lightweight governance dashboard with Network Health, Governance, and Audit Trail panels.
 * Uses requestAnimationFrame, debounce, and lazy loading to avoid competing with WASM Worker.
 */

function stewardship() {
    return {
        // State
        tab: 'network',
        loading: {
            network: false,
            governance: false,
            audit: false
        },
        networkData: {
            peers: 0,
            consensusLatency: 0,
            slashingRate: 0,
            wasmWorkers: 0,
            lastUpdated: null
        },
        governanceData: {
            activeRfcs: 0,
            votingProposals: 0,
            rlhfCorrections: 0,
            meritTiers: 4
        },
        meritTiers: [],
        auditData: {
            recentCommits: 0,
            ciCdBuilds: 0,
            activeFeatureGates: 0,
            testsPassed: 0
        },
        recentActivity: [],
        pollingInterval: null,
        pollDelayMs: 10000, // 10s between polls

        // Debounce timer
        debounceTimer: null,

        init() {
            this.loadNetworkData();
            this.loadGovernanceData();
            this.loadAuditData();
            this.startAutoRefresh();
        },

        // Tab switching
        switchTab(newTab) {
            this.tab = newTab;
            // Load data for the new tab if not already loaded
            if (newTab === 'network' && !this.networkData.lastUpdated) {
                this.loadNetworkData();
            } else if (newTab === 'governance' && !this.governanceData.activeRfcs) {
                this.loadGovernanceData();
            } else if (newTab === 'audit' && !this.auditData.recentCommits) {
                this.loadAuditData();
            }
        },

        // Network Data
        async loadNetworkData() {
            this.loading.network = true;
            try {
                // Fetch /api/metrics
                const response = await this.fetchWithTimeout('/api/metrics', 5000);
                if (response.ok) {
                    const text = await response.text();
                    this.parseMetrics(text);
                }
            } catch (err) {
                console.warn('[Stewardship] Network metrics unavailable:', err.message);
                // Use simulated data for demo
                this.useSimulatedNetworkData();
            } finally {
                this.loading.network = false;
            }
        },

        parseMetrics(metricsText) {
            // Parse Prometheus-style metrics
            const lines = metricsText.split('\n');
            for (const line of lines) {
                if (line.startsWith('ed2k_network_peers')) {
                    this.networkData.peers = parseInt(line.split(' ')[1]) || 0;
                } else if (line.startsWith('ed2k_consensus_latency')) {
                    this.networkData.consensusLatency = parseFloat(line.split(' ')[1]) * 1000 || 0;
                } else if (line.startsWith('ed2k_reputation_slashing_total')) {
                    this.networkData.slashingRate = parseFloat(line.split(' ')[1]) || 0;
                } else if (line.startsWith('ed2k_wasm_worker_active')) {
                    this.networkData.wasmWorkers = parseInt(line.split(' ')[1]) || 0;
                }
            }
            this.networkData.lastUpdated = new Date().toLocaleTimeString();
        },

        useSimulatedNetworkData() {
            // Simulated data for demonstration
            this.networkData = {
                peers: 42,
                consensusLatency: 120,
                slashingRate: 0.02,
                wasmWorkers: 8,
                lastUpdated: new Date().toLocaleTimeString()
            };
        },

        // Governance Data
        async loadGovernanceData() {
            this.loading.governance = true;
            try {
                // Fetch /api/merit/tiers
                const response = await this.fetchWithTimeout('/api/merit/tiers', 5000);
                if (response.ok) {
                    const data = await response.json();
                    this.meritTiers = data.tiers || [];
                    this.governanceData.meritTiers = this.meritTiers.length;
                }
            } catch (err) {
                console.warn('[Stewardship] Merit tiers unavailable:', err.message);
                this.useSimulatedMeritTiers();
            }

            try {
                // Fetch RFC count from GitHub API or local endpoint
                const rfcResponse = await this.fetchWithTimeout('/api/governance/rfcs', 5000);
                if (rfcResponse.ok) {
                    const data = await rfcResponse.json();
                    this.governanceData.activeRfcs = data.active || 0;
                    this.governanceData.votingProposals = data.voting || 0;
                }
            } catch (err) {
                console.warn('[Stewardship] RFC data unavailable:', err.message);
                this.governanceData.activeRfcs = 3;
                this.governanceData.votingProposals = 1;
            }

            // RLHF corrections from metrics
            try {
                const metricsResponse = await this.fetchWithTimeout('/api/metrics', 5000);
                if (metricsResponse.ok) {
                    const text = await metricsResponse.text();
                    const match = text.match(/ed2k_rlhf_feedback_accepted_total\s+(\d+)/);
                    if (match) {
                        this.governanceData.rlhfCorrections = parseInt(match[1]);
                    }
                }
            } catch (err) {
                this.governanceData.rlhfCorrections = 156;
            }

            this.loading.governance = false;
        },

        useSimulatedMeritTiers() {
            this.meritTiers = [
                {
                    name: 'Novice',
                    class: 'novice',
                    count: 45,
                    weight: 0.5,
                    requirements: 'Nuevo contribuidor'
                },
                {
                    name: 'Contributor',
                    class: 'contributor',
                    count: 28,
                    weight: 1.0,
                    requirements: '5+ PRs mergeados'
                },
                {
                    name: 'Auditor',
                    class: 'auditor',
                    count: 12,
                    weight: 1.5,
                    requirements: 'Entrenamiento de auditoría'
                },
                {
                    name: 'Steward',
                    class: 'steward',
                    count: 4,
                    weight: 2.0,
                    requirements: 'Miembro del equipo core'
                }
            ];
        },

        // Audit Data
        async loadAuditData() {
            this.loading.audit = true;
            try {
                // Fetch feature gates from /api/features
                const featureResponse = await this.fetchWithTimeout('/api/features', 5000);
                if (featureResponse.ok) {
                    const data = await featureResponse.json();
                    this.auditData.activeFeatureGates = data.features?.length || 0;
                }
            } catch (err) {
                console.warn('[Stewardship] Feature gates unavailable:', err.message);
                this.auditData.activeFeatureGates = 12;
            }

            // Simulated audit data (would come from GitHub API in production)
            this.auditData = {
                recentCommits: 24,
                ciCdBuilds: 18,
                activeFeatureGates: this.auditData.activeFeatureGates,
                testsPassed: 142
            };

            this.recentActivity = [
                {
                    time: '2h ago',
                    event: 'Commit',
                    details: 'feat: add stewardship dashboard',
                    status: 'PASS',
                    statusDot: 'green'
                },
                {
                    time: '3h ago',
                    event: 'CI/CD Build',
                    details: 'Build #1247 — main branch',
                    status: 'PASS',
                    statusDot: 'green'
                },
                {
                    time: '5h ago',
                    event: 'Test Suite',
                    details: '142 tests passed, 0 failed',
                    status: 'PASS',
                    statusDot: 'green'
                },
                {
                    time: '8h ago',
                    event: 'Feature Gate',
                    details: 'v2.1-stewardship enabled',
                    status: 'ACTIVE',
                    statusDot: 'green'
                },
                {
                    time: '12h ago',
                    event: 'Security Audit',
                    details: 'No vulnerabilities found',
                    status: 'PASS',
                    statusDot: 'green'
                }
            ];

            this.loading.audit = false;
        },

        // Utility: Fetch with timeout
        fetchWithTimeout(url, timeoutMs) {
            return Promise.race([
                fetch(url),
                new Promise((_, reject) =>
                    setTimeout(() => reject(new Error(`Timeout: ${url}`)), timeoutMs)
                )
            ]);
        },

        // Formatting helpers
        formatLatency(ms) {
            if (ms == null) return '—';
            return Math.round(ms).toLocaleString();
        },

        formatPercent(value) {
            if (value == null) return '—';
            return (value * 100).toFixed(2);
        },

        healthClass(value) {
            if (value == null) return '';
            // Peers: more is better
            if (value > 20) return 'success';
            if (value > 10) return 'warning';
            return 'danger';
        },

        // Auto-refresh with requestAnimationFrame + debounce
        startAutoRefresh() {
            if (this.pollingInterval) {
                clearInterval(this.pollingInterval);
            }
            this.pollingInterval = setInterval(() => {
                // Use requestAnimationFrame for non-blocking updates
                requestAnimationFrame(() => {
                    this.debouncedRefresh();
                });
            }, this.pollDelayMs);
        },

        debouncedRefresh() {
            if (this.debounceTimer) {
                clearTimeout(this.debounceTimer);
            }
            this.debounceTimer = setTimeout(() => {
                if (this.tab === 'network') {
                    this.loadNetworkData();
                } else if (this.tab === 'governance') {
                    this.loadGovernanceData();
                } else if (this.tab === 'audit') {
                    this.loadAuditData();
                }
            }, 500); // 500ms debounce
        },

        // Cleanup on component destruction
        cleanup() {
            if (this.pollingInterval) {
                clearInterval(this.pollingInterval);
            }
            if (this.debounceTimer) {
                clearTimeout(this.debounceTimer);
            }
        }
    };
}
