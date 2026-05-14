// ed2kIA Dashboard - Alpine.js Application

function dashboard() {
    return {
        // State
        tab: 'status',
        loaded: false,
        healthStatus: 'healthy',
        healthText: 'Saludable',
        uptime: 0,
        uptimeInterval: null,

        // Data
        status: null,
        network: null,
        feedback: null,
        metrics: null,

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
            this.startAutoRefresh();
            this.startUptimeCounter();
        },

        // API Calls
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

        // Auto-refresh
        startAutoRefresh() {
            this.refreshInterval = setInterval(() => {
                this.loadHealth();
                if (this.tab === 'status') {
                    this.loadStatus();
                } else if (this.tab === 'network') {
                    this.loadNetwork();
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

        // Uptime Counter
        startUptimeCounter() {
            this.uptimeInterval = setInterval(() => {
                this.uptime += 5; // Add 5 seconds every interval
            }, 5000);
        },

        // Formatters
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
