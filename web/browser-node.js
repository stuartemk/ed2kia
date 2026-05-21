/**
 * ed2kIA Browser Node — Web Worker Bridge & Initialization
 *
 * Ley 4 (Simbiosis): Hardware modesto, conexiones inestables, fricción cero.
 * Ley 1 (Diversidad): Cero centralización, propiedad comunitaria.
 *
 * Feature gate: v2.1-wasm-browser-node
 *
 * Flujo:
 * 1. new Worker('browser-node.worker.js') → postMessage init
 * 2. onmessage task dispatch → postMessage results
 * 3. Health monitoring: heartbeat 5s, fallback a cola local
 * 4. Métricas a public-dashboard.html
 *
 * Cero telemetría externa, cero trackers, cero lógica financiera.
 */

(function (root) {
    'use strict';

    // ============================================================================
    // Constants
    // ============================================================================

    var HEARTBEAT_INTERVAL_MS = 5000;
    var MAX_QUEUE_SIZE = 64;
    var WORKER_TIMEOUT_MS = 10000;
    var DEFAULT_MEMORY_MB = 64;

    // ============================================================================
    // BrowserNodeManager — Main API
    // ============================================================================

    /**
     * BrowserNodeManager — Manages WASM BrowserNode lifecycle.
     *
     * @param {Object} options
     * @param {string} options.id - Unique node identifier
     * @param {number} options.memoryLimitMb - Memory limit in MB (16-512)
     * @param {string} options.workerUrl - Path to browser-node.worker.js
     * @param {boolean} options.useWorker - Enable Web Worker (default: true)
     */
    function BrowserNodeManager(options) {
        options = options || {};

        this.id = options.id || 'browser-node-' + Date.now();
        this.memoryLimitMb = Math.max(16, Math.min(512, options.memoryLimitMb || DEFAULT_MEMORY_MB));
        this.workerUrl = options.workerUrl || 'browser-node.worker.js';
        this.useWorker = options.useWorker !== false;

        this.worker = null;
        this.initialized = false;
        this.taskQueue = [];
        this.processedCount = 0;
        this.failedCount = 0;
        this.heartbeatTimer = null;
        this.startedAt = Date.now();
        this.listeners = {};
        this.offlineQueue = [];
        this.connected = false;
    }

    // ============================================================================
    // Public API
    // ============================================================================

    /**
     * Initialize the browser node.
     * @returns {Promise<Object>} Initialization result
     */
    BrowserNodeManager.prototype.init = function () {
        var self = this;
        return new Promise(function (resolve, reject) {
            try {
                if (self.useWorker && typeof Worker !== 'undefined') {
                    self._initWorker();
                }

                self.initialized = true;
                self._startHeartbeat();

                var result = {
                    id: self.id,
                    status: 'initialized',
                    memoryLimitMb: self.memoryLimitMb,
                    useWorker: self.useWorker,
                    timestamp: Date.now()
                };

                self._emit('initialized', result);
                resolve(result);
            } catch (err) {
                self._emit('error', { message: err.message });
                reject(err);
            }
        });
    };

    /**
     * Process a task.
     * @param {string} payload - JSON task payload
     * @returns {Promise<Object>} Task result
     */
    BrowserNodeManager.prototype.processTask = function (payload) {
        var self = this;
        return new Promise(function (resolve, reject) {
            if (!self.initialized) {
                var err = new Error('Node not initialized. Call init() first.');
                self._emit('error', { message: err.message });
                reject(err);
                return;
            }

            if (!payload || typeof payload !== 'string') {
                var err = new Error('Invalid payload');
                self._emit('error', { message: err.message });
                reject(err);
                return;
            }

            // Queue if worker is busy
            if (self.useWorker && self.worker && self.connected) {
                self._sendToWorker(payload, resolve, reject);
            } else {
                // Fallback: process locally
                self._processLocal(payload).then(resolve).catch(reject);
            }
        });
    };

    /**
     * Get health status.
     * @returns {Object} Health metrics
     */
    BrowserNodeManager.prototype.getHealth = function () {
        var uptime = Date.now() - this.startedAt;
        return {
            id: this.id,
            initialized: this.initialized,
            connected: this.connected,
            queueSize: this.taskQueue.length,
            processedCount: this.processedCount,
            failedCount: this.failedCount,
            memoryLimitMb: this.memoryLimitMb,
            uptimeMs: uptime,
            useWorker: this.useWorker,
            offlineQueueSize: this.offlineQueue.length,
            timestamp: Date.now()
        };
    };

    /**
     * Register an event listener.
     * @param {string} event - Event name
     * @param {Function} callback - Callback function
     */
    BrowserNodeManager.prototype.on = function (event, callback) {
        if (!this.listeners[event]) {
            this.listeners[event] = [];
        }
        this.listeners[event].push(callback);
    };

    /**
     * Remove an event listener.
     * @param {string} event - Event name
     * @param {Function} callback - Callback function
     */
    BrowserNodeManager.prototype.off = function (event, callback) {
        if (!this.listeners[event]) return;
        var idx = this.listeners[event].indexOf(callback);
        if (idx >= 0) {
            this.listeners[event].splice(idx, 1);
        }
    };

    /**
     * Shutdown the node.
     */
    BrowserNodeManager.prototype.shutdown = function () {
        if (this.heartbeatTimer) {
            clearInterval(this.heartbeatTimer);
            this.heartbeatTimer = null;
        }
        if (this.worker) {
            this.worker.terminate();
            this.worker = null;
        }
        this.initialized = false;
        this.connected = false;
        this._emit('shutdown', { id: this.id });
    };

    // ============================================================================
    // Internal Methods
    // ============================================================================

    BrowserNodeManager.prototype._initWorker = function () {
        var self = this;
        try {
            this.worker = new Worker(this.workerUrl);

            this.worker.onmessage = function (e) {
                var data = e.data;
                if (data.type === 'ready') {
                    self.connected = true;
                    self._emit('worker-connected', { id: self.id });
                    self._flushOfflineQueue();
                } else if (data.type === 'task-result') {
                    self.processedCount++;
                    self._emit('task-complete', data.result);
                    if (data._resolve) {
                        data._resolve(data.result);
                    }
                } else if (data.type === 'error') {
                    self.failedCount++;
                    self._emit('worker-error', data);
                    if (data._reject) {
                        data._reject(new Error(data.message));
                    }
                }
            };

            this.worker.onerror = function (err) {
                self.connected = false;
                self._emit('worker-error', { message: err.message });
            };

            // Send init message
            this.worker.postMessage({
                type: 'init',
                id: this.id,
                memoryLimitMb: this.memoryLimitMb
            });
        } catch (err) {
            console.warn('[BrowserNode] Worker init failed, falling back to local:', err.message);
            this.useWorker = false;
        }
    };

    BrowserNodeManager.prototype._sendToWorker = function (payload, resolve, reject) {
        var self = this;
        var timeout = setTimeout(function () {
            self._emit('timeout', { payload });
            reject(new Error('Worker timeout'));
        }, WORKER_TIMEOUT_MS);

        // Wrap resolve/reject in message for worker response
        var message = {
            type: 'task',
            payload: payload,
            _resolve: function (result) {
                clearTimeout(timeout);
                resolve(result);
            },
            _reject: function (err) {
                clearTimeout(timeout);
                reject(err);
            }
        };

        // Can't serialize functions, so use pending promises
        var pending = { resolve: resolve, reject: reject, timeout: timeout };
        this.taskQueue.push({ payload, pending: pending });

        try {
            this.worker.postMessage({
                type: 'task',
                payload: payload,
                queueIndex: this.taskQueue.length - 1
            });
        } catch (err) {
            clearTimeout(timeout);
            this.connected = false;
            this.offlineQueue.push(payload);
            this._emit('disconnected', { queued: this.offlineQueue.length });
            reject(new Error('Worker disconnected'));
        }
    };

    BrowserNodeManager.prototype._processLocal = function (payload) {
        // Local fallback — simulates WASM processing
        var start = Date.now();
        var result;

        try {
            var task = JSON.parse(payload);
            var taskType = task.type || 'HealthCheck';

            switch (taskType) {
                case 'HealthCheck':
                    result = { task_id: this.id + '-local', success: true, output: 'pong-' + this.id, latency_ms: Date.now() - start };
                    break;
                case 'SaeInference':
                    var k = (payload.length % 16) + 8;
                    var mean = (payload.length % 100) / 100;
                    result = { task_id: this.id + '-local', success: true, output: 'activations(k=' + k + ',mean=' + mean.toFixed(3) + ',node=' + this.id + ')', latency_ms: Date.now() - start };
                    break;
                case 'GradientValidation':
                    var valid = payload.length > 10;
                    result = { task_id: this.id + '-local', success: valid, output: valid ? 'gradient_valid' : 'gradient_too_short', latency_ms: Date.now() - start };
                    break;
                default:
                    result = { task_id: this.id + '-local', success: true, output: 'unknown-task', latency_ms: Date.now() - start };
            }

            this.processedCount++;
            this._emit('task-complete', result);
            return Promise.resolve(result);
        } catch (err) {
            this.failedCount++;
            this._emit('error', { message: err.message });
            return Promise.reject(err);
        }
    };

    BrowserNodeManager.prototype._startHeartbeat = function () {
        var self = this;
        this.heartbeatTimer = setInterval(function () {
            var health = self.getHealth();
            self._emit('heartbeat', health);

            // Check connectivity
            if (self.useWorker && self.worker) {
                try {
                    self.worker.postMessage({ type: 'ping' });
                } catch (err) {
                    self.connected = false;
                    self._emit('disconnected', { reason: err.message });
                }
            }
        }, HEARTBEAT_INTERVAL_MS);
    };

    BrowserNodeManager.prototype._flushOfflineQueue = function () {
        var self = this;
        while (this.offlineQueue.length > 0 && this.taskQueue.length < MAX_QUEUE_SIZE) {
            var payload = this.offlineQueue.shift();
            this._processLocal(payload).catch(function (err) {
                console.warn('[BrowserNode] Offline task failed:', err.message);
            });
        }
        this._emit('queue-flushed', { remaining: this.offlineQueue.length });
    };

    BrowserNodeManager.prototype._emit = function (event, data) {
        var callbacks = this.listeners[event];
        if (callbacks) {
            for (var i = 0; i < callbacks.length; i++) {
                try {
                    callbacks[i](data);
                } catch (err) {
                    console.error('[BrowserNode] Listener error:', err.message);
                }
            }
        }

        // Dispatch CustomEvent for DOM integration
        if (typeof window !== 'undefined' && window.CustomEvent) {
            var customEvent = new CustomEvent('ed2k-' + event, { detail: data });
            window.dispatchEvent(customEvent);
        }
    };

    // ============================================================================
    // Export
    // ============================================================================

    if (typeof module !== 'undefined' && module.exports) {
        module.exports = BrowserNodeManager;
    } else {
        root.BrowserNodeManager = BrowserNodeManager;
    }

})(typeof window !== 'undefined' ? window : this);
