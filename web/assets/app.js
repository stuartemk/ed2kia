// ed2kIA Dashboard - Alpine.js Application
// Sprint8: Production Portal + Merit System Integration

function dashboard() {
    return {
        // State
        tab: 'status',
        loaded: false,
        healthStatus: 'healthy',
        healthText: 'Saludable',
        uptime: 0,
        uptimeInterval: null,

        // Browser Node (Sprint8)
        connected: false,
        connecting: false,
        connectError: '',

        // Data
        status: null,
        network: null,
        feedback: null,
        metrics: null,
        atlasStats: null,

        // Merit System (Sprint8)
        meritData: null,
        proofs: [],
        claiming: false,
        claimSuccess: '',
        claimError: '',

        // Form
        newFeedback: {
            layer_id: '',
            feature_idx: 0,
            decision: 'approved',
            concept: ''
        },
        submitting: false,
        submitSuccess: '',
        submitError: '',

        // Auto-refresh
        refreshInterval: null,

        init() {
            this.loadStatus();
            this.loadHealth();
            this.loadMeritFromStorage();
            this.startAutoRefresh();
            this.startUptimeCounter();
        },

        // ─── Browser Node Connection (Sprint8) ───

        async connectBrowserNode() {
            this.connecting = true;
            this.connectError = '';

            try {
                // Initialize WASM Worker + WebRTC via API
                const response = await fetch('/api/node/connect', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ type: 'browser' })
                });

                const data = await response.json();
                if (data.success) {
                    this.connected = true;
                    console.log('[ed2kIA] Browser node connected');
                } else {
                    this.connectError = data.error || 'Error al conectar';
                }
            } catch (error) {
                // Fallback: simulate connection for demo
                console.warn('[ed2kIA] /api/node/connect unavailable, using fallback');
                this.connected = true;
                this.startAtlasPolling();
            } finally {
                this.connecting = false;
            }
        },

        disconnectBrowserNode() {
            this.connected = false;
            fetch('/api/node/disconnect', { method: 'POST' }).catch(() => { });
            this.stopAtlasPolling();
        },

        // ─── Atlas Stats (Sprint8) ───

        async loadAtlasStats() {
            try {
                const response = await fetch('/api/atlas/stats');
                const data = await response.json();
                if (data.success) {
                    this.atlasStats = data.data;
                }
            } catch (error) {
                console.warn('[ed2kIA] /api/atlas/stats unavailable');
                // Fallback: use network data if available
                if (this.network) {
                    this.atlasStats = {
                        voluntarios_activos: this.network.peer_count || 0,
                        neuronas_auditadas: (this.network.messages_sent || 0) + (this.network.messages_received || 0),
                        ataques_bloqueados: 0,
                        consenso_global: 0.95
                    };
                }
            }
        },

        atlasPollInterval: null,

        startAtlasPolling() {
            if (this.connected && !this.atlasPollInterval) {
                this.loadAtlasStats();
                this.atlasPollInterval = setInterval(() => {
                    if (this.tab === 'atlas') {
                        this.loadAtlasStats();
                    }
                }, 5000); // Poll every 5s when on Atlas tab
            }
        },

        stopAtlasPolling() {
            if (this.atlasPollInterval) {
                clearInterval(this.atlasPollInterval);
                this.atlasPollInterval = null;
            }
        },

        // ─── Merit System (Sprint8) ───

        loadMeritFromStorage() {
            try {
                const stored = localStorage.getItem('ed2kIA_merit_proofs');
                if (stored) {
                    this.proofs = JSON.parse(stored);
                }
                const current = localStorage.getItem('ed2kIA_merit_current');
                if (current) {
                    this.meritData = JSON.parse(current);
                }
            } catch (e) {
                console.error('[ed2kIA] Error loading merit from storage:', e);
            }
        },

        saveMeritToStorage() {
            try {
                localStorage.setItem('ed2kIA_merit_proofs', JSON.stringify(this.proofs));
                if (this.meritData) {
                    localStorage.setItem('ed2kIA_merit_current', JSON.stringify(this.meritData));
                }
            } catch (e) {
                console.error('[ed2kIA] Error saving merit to storage:', e);
            }
        },

        async loadMerit() {
            this.loadMeritFromStorage();
            try {
                const response = await fetch('/api/merit/status');
                const data = await response.json();
                if (data.success && data.data) {
                    this.meritData = data.data;
                    this.saveMeritToStorage();
                }
            } catch (error) {
                console.warn('[ed2kIA] /api/merit/status unavailable');
                // Use stored data if API unavailable
                if (!this.meritData && this.proofs.length > 0) {
                    const latest = this.proofs[this.proofs.length - 1];
                    this.meritData = {
                        node_id: latest.node_id,
                        audit_count: latest.audit_count,
                        tier: latest.tier,
                    };
                }
            }
        },

        async claimMerit() {
            this.claiming = true;
            this.claimSuccess = '';
            this.claimError = '';

            try {
                const response = await fetch('/api/merit/claim', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        node_id: this.getNodeId(),
                        audit_count: this.meritData?.audit_count || 0
                    })
                });

                const data = await response.json();
                if (data.success && data.data) {
                    const proof = data.data;
                    this.proofs.push(proof);
                    this.meritData = {
                        node_id: proof.node_id,
                        audit_count: proof.audit_count,
                        tier: proof.tier,
                    };
                    this.saveMeritToStorage();
                    this.claimSuccess = 'Proof de mérito firmado y almacenado';
                } else {
                    this.claimError = data.error || 'Error al solicitar proof';
                }
            } catch (error) {
                // Generate local proof for demo when API unavailable
                console.warn('[ed2kIA] /api/merit/claim unavailable, generating local proof');
                const localProof = this.generateLocalProof();
                this.proofs.push(localProof);
                this.meritData = {
                    node_id: localProof.node_id,
                    audit_count: localProof.audit_count,
                    tier: localProof.tier,
                };
                this.saveMeritToStorage();
                this.claimSuccess = 'Proof local generado (modo demo)';
            } finally {
                this.claiming = false;
            }
        },

        generateLocalProof() {
            const auditCount = this.meritData?.audit_count || 1;
            const tier = this.calculateTier(auditCount);
            const timestamp = Math.floor(Date.now() / 1000);
            const hash = btoa(`${this.getNodeId()}:${auditCount}:${timestamp}`).substring(0, 32);

            return {
                node_id: this.getNodeId(),
                audit_count: auditCount,
                timestamp: timestamp,
                tier: tier,
                hash: hash,
                signature: 'local-demo-' + Math.random().toString(36).substring(2, 18)
            };
        },

        calculateTier(auditCount) {
            if (auditCount >= 1000) return 'Steward';
            if (auditCount >= 100) return 'Guardian';
            if (auditCount >= 10) return 'Contributor';
            return 'Novice';
        },

        getTierLabel(tier) {
            const labels = {
                'Novice': 'Novato — Explorando la Red',
                'Contributor': 'Contributor — Auditor Activo',
                'Guardian': 'Guardián — Protector de la Verdad',
                'Steward': 'Mayordomo — Líder de la Comunidad'
            };
            return labels[tier] || tier;
        },

        getTierBadge(tier) {
            const badges = {
                'Novice': '🌱',
                'Contributor': '⚡',
                'Guardian': '🛡️',
                'Steward': '👑'
            };
            return badges[tier] || '❓';
        },

        getNextTierInfo() {
            if (!this.meritData) return 'Conecta para ver tu progreso';
            const counts = { 'Novice': 10, 'Contributor': 100, 'Guardian': 1000, 'Steward': null };
            const current = this.meritData.tier;
            const next = counts[current];

            if (!next) return 'Rango máximo alcanzado';
            const remaining = next - (this.meritData.audit_count || 0);
            return `${remaining} auditorías para el siguiente rango`;
        },

        getNodeId() {
            let nodeId = localStorage.getItem('ed2kIA_node_id');
            if (!nodeId) {
                nodeId = 'browser-' + Math.random().toString(36).substring(2, 10);
                localStorage.setItem('ed2kIA_node_id', nodeId);
            }
            return nodeId;
        },

        // ─── API Calls ───

        async loadStatus() {
            try {
                const response = await fetch('/api/status');
                const data = await response.json();
                if (data.success) {
                    this.status = data.data;
                }
            } catch (error) {
                console.error('Error loading status:', error);
            }
            this.loaded = true;
        },

        async loadNetwork() {
            try {
                const response = await fetch('/api/network');
                const data = await response.json();
                if (data.success) {
                    this.network = data.data;
                }
            } catch (error) {
                console.error('Error loading network:', error);
            }
            this.loaded = true;
        },

        async loadFeedback() {
            try {
                const response = await fetch('/api/feedback');
                const data = await response.json();
                if (data.success) {
                    this.feedback = data.data;
                }
            } catch (error) {
                console.error('Error loading feedback:', error);
            }
            this.loaded = true;
        },

        async loadMetrics() {
            try {
                const response = await fetch('/api/metrics');
                this.metrics = await response.text();
            } catch (error) {
                console.error('Error loading metrics:', error);
                this.metrics = 'Error cargando métricas';
            }
        },

        async loadHealth() {
            try {
                const response = await fetch('/api/health');
                const data = await response.json();
                if (data.success) {
                    this.healthStatus = data.data.status || 'healthy';
                    this.healthText = data.data.message || 'Saludable';
                    if (data.data.uptime_seconds) {
                        this.uptime = data.data.uptime_seconds;
                    }
                }
            } catch (error) {
                console.error('Error loading health:', error);
                this.healthStatus = 'unhealthy';
                this.healthText = 'Error de conexión';
            }
        },

        async submitFeedback() {
            this.submitting = true;
            this.submitSuccess = '';
            this.submitError = '';

            try {
                const payload = {
                    layer_id: this.newFeedback.layer_id,
                    feature_idx: parseInt(this.newFeedback.feature_idx),
                    decision: this.newFeedback.decision,
                    concept: this.newFeedback.concept || null
                };

                const response = await fetch('/api/feedback', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(payload)
                });

                const data = await response.json();
                if (data.success) {
                    this.submitSuccess = 'Feedback enviado correctamente';
                    this.resetForm();
                    this.loadFeedback();
                } else {
                    this.submitError = data.error || 'Error al enviar feedback';
                }
            } catch (error) {
                this.submitError = 'Error de conexión: ' + error.message;
            } finally {
                this.submitting = false;
            }
        },

        resetForm() {
            this.newFeedback = {
                layer_id: '',
                feature_idx: 0,
                decision: 'approved',
                concept: ''
            };
        },

        // ─── Auto-refresh ───

        startAutoRefresh() {
            this.refreshInterval = setInterval(() => {
                this.loadHealth();
                if (this.tab === 'status') {
                    this.loadStatus();
                } else if (this.tab === 'network') {
                    this.loadNetwork();
                } else if (this.tab === 'atlas') {
                    this.loadAtlasStats();
                } else if (this.tab === 'feedback') {
                    this.loadFeedback();
                }
            }, 10000); // Every 10 seconds
        },

        stopAutoRefresh() {
            if (this.refreshInterval) {
                clearInterval(this.refreshInterval);
            }
            if (this.uptimeInterval) {
                clearInterval(this.uptimeInterval);
            }
        },

        // ─── Uptime Counter ───

        startUptimeCounter() {
            this.uptimeInterval = setInterval(() => {
                this.uptime += 5; // Add 5 seconds every interval
            }, 5000);
        },

        // ─── Formatters ───

        formatUptime(seconds) {
            const days = Math.floor(seconds / 86400);
            const hours = Math.floor((seconds % 86400) / 3600);
            const minutes = Math.floor((seconds % 3600) / 60);
            const secs = Math.floor(seconds % 60);

            if (days > 0) {
                return `${days}d ${hours}h ${minutes}m`;
            }
            if (hours > 0) {
                return `${hours}h ${minutes}m ${secs}s`;
            }
            if (minutes > 0) {
                return `${minutes}m ${secs}s`;
            }
            return `${secs}s`;
        },

        cleanup() {
            this.stopAutoRefresh();
            this.stopAtlasPolling();
        }
    };
}

// Service Worker Registration (Optional for PWA)
if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
        // Uncomment to enable PWA
        // navigator.serviceWorker.register('/sw.js');
    });
}

// Performance monitoring
window.addEventListener('load', () => {
    const timing = performance.getEntriesByType('navigation')[0];
    if (timing) {
        console.log(`[ed2kIA] Page loaded in ${Math.round(timing.loadEventEnd - timing.startTime)}ms`);
    }
});
