/**
 * ed2kIA Browser Node — Web Worker Bridge (Sprint25)
 *
 * Ley 4 (Simbiosis): Hardware modesto, conexiones inestables, fricción cero.
 * Ley 1 (Diversidad): Cero centralización, propiedad comunitaria.
 *
 * Feature gates: v2.1-wasm-worker, v2.1-ui-symbiosis
 *
 * Flujo Sprint25:
 * 1. new Worker('wasm-worker.js') → postMessage { type: 'start_node' }
 * 2. Worker emite { type: 'telemetry', tensors_processed, last_sct: { x, y, z, decision } } cada 1s
 * 3. Main Thread escucha y actualiza UI + Octaedro 3D
 * 4. Health monitoring: heartbeat 5s, fallback a cola local (max 128)
 * 5. Métricas a public-dashboard.html vía CustomEvent ed2k-telemetry
 *
 * Cero telemetría externa, cero trackers, cero lógica financiera.
 * WASM NUNCA bloquea el Main Thread.
 */

(function (root) {
    'use strict';

    // ============================================================================
    // Constants
    // ============================================================================

    var HEARTBEAT_INTERVAL_MS = 5000;
    var MAX_QUEUE_SIZE = 128;  // Increased for Sprint25 offline queue
    var WORKER_TIMEOUT_MS = 10000;
    var DEFAULT_MEMORY_MB = 64;
    var DEFAULT_WORKER_URL = 'wasm-worker.js';  // Sprint25: wasm-worker.js

    // ============================================================================
    // BrowserNodeManager — Main API (Sprint25 Refactored)
    // ============================================================================

    /**
     * BrowserNodeManager — Manages WASM BrowserNode lifecycle via Web Worker.
     *
     * @param {Object} options
     * @param {string} options.id - Unique node identifier
     * @param {number} options.memoryLimitMb - Memory limit in MB (16-512)
     * @param {string} options.workerUrl - Path to wasm-worker.js (default: 'wasm-worker.js')
     * @param {boolean} options.useWorker - Enable Web Worker (default: true)
     */
    function BrowserNodeManager(options) {
        options = options || {};

        this.id = options.id || 'browser-node-' + Date.now();
        this.memoryLimitMb = Math.max(16, Math.min(512, options.memoryLimitMb || DEFAULT_MEMORY_MB));
        this.workerUrl = options.workerUrl || DEFAULT_WORKER_URL;
        this.useWorker = options.useWorker !== false;

        this.worker = null;
        this.initialized = false;
        this.nodeStarted = false;  // Sprint25: tracks startNode() state
        this.taskQueue = [];
        this.processedCount = 0;
        this.failedCount = 0;
        this.tensorsProcessed = 0;  // Sprint25: SCT tensors processed
        this.tensorsRejected = 0;   // Sprint25: SCT tensors rejected
        this.lastSct = null;        // Sprint25: last SCT vector { x, y, z, decision }
        this.heartbeatTimer = null;
        this.startedAt = Date.now();
        this.listeners = {};
        this.offlineQueue = [];
        this.connected = false;
        this.pendingPromises = {};  // Sprint25: task_id -> { resolve, reject, timeout }
    }

    // ============================================================================
    // Public API — Core (Backward Compatible)
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
     * Process a task (backward compatible).
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

            // Sprint25: Send to wasm-worker if connected
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
            nodeStarted: this.nodeStarted,
            queueSize: this.taskQueue.length,
            processedCount: this.processedCount,
            failedCount: this.failedCount,
            tensorsProcessed: this.tensorsProcessed,
            tensorsRejected: this.tensorsRejected,
            lastSct: this.lastSct,
            memoryLimitMb: this.memoryLimitMb,
            uptimeMs: uptime,
            useWorker: this.useWorker,
            offlineQueueSize: this.offlineQueue.length,
            timestamp: Date.now()
        };
    };

    // ============================================================================
    // Public API — Sprint25 Symbiosis (New)
    // ============================================================================

    /**
     * Start the symbiotic node (Sprint25).
     * Sends { type: 'start_node' } to wasm-worker.
     * @returns {Promise<Object>} Start result
     */
    BrowserNodeManager.prototype.startNode = function () {
        var self = this;
        return new Promise(function (resolve, reject) {
            if (!self.initialized) {
                var err = new Error('Node not initialized. Call init() first.');
                reject(err);
                return;
            }

            if (!self.useWorker || !self.worker) {
                self.nodeStarted = true;
                resolve({ id: self.id, status: 'started', wasm_loaded: false });
                return;
            }

            try {
                self.worker.postMessage({
                    type: 'start_node',
                    id: self.id,
                    memoryLimitMb: self.memoryLimitMb
                });

                // Wait for node_ready response
                var timeout = setTimeout(function () {
                    self._removePending('start_node');
                    self._emit('timeout', { action: 'start_node' });
                    reject(new Error('Start node timeout'));
                }, WORKER_TIMEOUT_MS);

                self.pendingPromises['start_node'] = { resolve: resolve, reject: reject, timeout: timeout };
            } catch (err) {
                reject(err);
            }
        });
    };

    /**
     * Stop the symbiotic node (Sprint25).
     * Sends { type: 'stop_node' } to wasm-worker.
     * @returns {Promise<Object>} Stop result
     */
    BrowserNodeManager.prototype.stopNode = function () {
        var self = this;
        return new Promise(function (resolve, reject) {
            if (!self.nodeStarted) {
                resolve({ id: self.id, status: 'already_stopped' });
                return;
            }

            if (!self.useWorker || !self.worker) {
                self.nodeStarted = false;
                resolve({ id: self.id, status: 'stopped' });
                return;
            }

            try {
                self.worker.postMessage({ type: 'stop_node' });

                var timeout = setTimeout(function () {
                    self._removePending('stop_node');
                    reject(new Error('Stop node timeout'));
                }, WORKER_TIMEOUT_MS);

                self.pendingPromises['stop_node'] = { resolve: resolve, reject: reject, timeout: timeout };
            } catch (err) {
                reject(err);
            }
        });
    };

    /**
     * Process a tensor via SCT evaluation (Sprint25).
     * @param {string} payload - Tensor payload string
     * @returns {Promise<Object>} SCT result { x, y, z, decision }
     */
    BrowserNodeManager.prototype.processTensor = function (payload) {
        var self = this;
        return new Promise(function (resolve, reject) {
            if (!self.initialized) {
                reject(new Error('Node not initialized. Call init() first.'));
                return;
            }

            if (!payload || typeof payload !== 'string') {
                reject(new Error('Invalid payload'));
                return;
            }

            if (self.useWorker && self.worker && self.connected) {
                var task_id = 'tensor-' + Date.now() + '-' + Math.random().toString(36).substr(2, 6);
                self.worker.postMessage({
                    type: 'process_tensor',
                    payload: payload
                });

                var timeout = setTimeout(function () {
                    self._removePending(task_id);
                    reject(new Error('Process tensor timeout'));
                }, WORKER_TIMEOUT_MS);

                self.pendingPromises[task_id] = { resolve: resolve, reject: reject, timeout: timeout };
            } else {
                // Local fallback
                self._simulateSCT(payload).then(resolve).catch(reject);
            }
        });
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
     * Register a telemetry listener (Sprint25 shorthand).
     * @param {Function} callback - Receives { tensors_processed, tensors_rejected, last_sct: { x, y, z, decision } }
     */
    BrowserNodeManager.prototype.onTelemetry = function (callback) {
        this.on('telemetry', callback);
    };

    /**
     * Register an error listener (Sprint25 shorthand).
     * @param {Function} callback - Receives { message }
     */
    BrowserNodeManager.prototype.onError = function (callback) {
        this.on('error', callback);
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
            try {
                this.worker.postMessage({ type: 'stop_node' });
            } catch (e) { /* ignore */ }
            this.worker.terminate();
            this.worker = null;
        }
        this.initialized = false;
        this.nodeStarted = false;
        this.connected = false;

        // Clear pending promises
        var keys = Object.keys(this.pendingPromises);
        for (var i = 0; i < keys.length; i++) {
            var entry = this.pendingPromises[keys[i]];
            if (entry.timeout) clearTimeout(entry.timeout);
            if (entry.reject) entry.reject(new Error('Node shutdown'));
        }
        this.pendingPromises = {};

        this._emit('shutdown', { id: this.id });
    };

    // ============================================================================
    // Internal Methods — Worker Bridge (Sprint25)
    // ============================================================================

    BrowserNodeManager.prototype._initWorker = function () {
        var self = this;
        try {
            this.worker = new Worker(this.workerUrl);

            this.worker.onmessage = function (e) {
                var data = e.data;
                if (!data || !data.type) return;

                switch (data.type) {
                    case 'node_ready':
                        self.connected = true;
                        self._emit('worker-connected', { id: self.id, wasm_loaded: data.wasm_loaded });
                        self._flushOfflineQueue();

                        // Resolve start_node promise if pending
                        var startPromise = self.pendingPromises['start_node'];
                        if (startPromise) {
                            clearTimeout(startPromise.timeout);
                            startPromise.resolve({
                                id: data.id || self.id,
                                status: 'ready',
                                wasm_loaded: data.wasm_loaded || false,
                                memoryLimitMb: data.memoryLimitMb
                            });
                            self._removePending('start_node');
                        }
                        break;

                    case 'telemetry':
                        // Sprint25: SCT telemetry from worker
                        self.tensorsProcessed = data.tensors_processed || 0;
                        self.tensorsRejected = data.tensors_rejected || 0;
                        self.lastSct = data.last_sct || null;
                        self._emit('telemetry', {
                            tensors_processed: self.tensorsProcessed,
                            tensors_rejected: self.tensorsRejected,
                            last_sct: self.lastSct,
                            queue_size: data.queue_size || 0,
                            timestamp: data.timestamp || Date.now()
                        });
                        break;

                    case 'tensor_result':
                        // Sprint25: SCT tensor result
                        var tensorPromise = self.pendingPromises[data.task_id];
                        if (tensorPromise) {
                            clearTimeout(tensorPromise.timeout);
                            if (data.success) {
                                tensorPromise.resolve({
                                    task_id: data.task_id,
                                    success: true,
                                    sct: data.sct,
                                    latency_ms: data.latency_ms
                                });
                            } else {
                                tensorPromise.reject(new Error('Tensor processing failed'));
                            }
                            self._removePending(data.task_id);
                        }
                        self.processedCount++;
                        self._emit('task-complete', { type: 'tensor', data: data });
                        break;

                    case 'node_stopped':
                        self.nodeStarted = false;
                        self.connected = false;

                        var stopPromise = self.pendingPromises['stop_node'];
                        if (stopPromise) {
                            clearTimeout(stopPromise.timeout);
                            stopPromise.resolve({
                                id: self.id,
                                status: 'stopped',
                                final_stats: data.final_stats
                            });
                            self._removePending('stop_node');
                        }
                        self._emit('node-stopped', data);
                        break;

                    case 'error':
                        self.failedCount++;
                        self._emit('worker-error', { message: data.message });
                        self._emit('error', { message: data.message });
                        break;

                    // Backward compatibility with old worker messages
                    case 'ready':
                        self.connected = true;
                        self._emit('worker-connected', { id: self.id });
                        self._flushOfflineQueue();
                        break;

                    case 'task-result':
                        self.processedCount++;
                        self._emit('task-complete', data.result);
                        var oldPromise = self.pendingPromises['task-' + (self.processedCount - 1)];
                        if (oldPromise) {
                            clearTimeout(oldPromise.timeout);
                            oldPromise.resolve(data.result);
                        }
                        break;
                }
            };

            this.worker.onerror = function (err) {
                self.connected = false;
                self._emit('worker-error', { message: err ? err.message : 'Unknown worker error' });
                self._emit('error', { message: 'Worker error: ' + (err ? err.message : 'Unknown') });
            };

        } catch (err) {
            console.warn('[BrowserNode] Worker init failed, falling back to local:', err.message);
            this.useWorker = false;
        }
    };

    BrowserNodeManager.prototype._sendToWorker = function (payload, resolve, reject) {
        var self = this;
        var task_id = 'task-' + Date.now() + '-' + Math.random().toString(36).substr(2, 6);
        var timeout = setTimeout(function () {
            self._removePending(task_id);
            self._emit('timeout', { payload });
            reject(new Error('Worker timeout'));
        }, WORKER_TIMEOUT_MS);

        this.taskQueue.push({ payload: payload, task_id: task_id });

        try {
            this.worker.postMessage({
                type: 'process_tensor',
                payload: payload
            });

            self.pendingPromises[task_id] = { resolve: resolve, reject: reject, timeout: timeout };
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

    /**
     * Simulate SCT evaluation locally (Sprint25 fallback).
     * @param {string} payload
     * @returns {Promise<Object>} SCT result { x, y, z, decision }
     */
    BrowserNodeManager.prototype._simulateSCT = function (payload) {
        var start = Date.now();
        try {
            var seed = 0;
            for (var i = 0; i < payload.length; i++) {
                seed = ((seed << 5) - seed) + payload.charCodeAt(i);
                seed = seed & seed;
            }
            var x = Math.abs(Math.sin(seed * 0.01)) * 0.7 + 0.2;
            var y = Math.abs(Math.cos(seed * 0.013)) * 0.5 + 0.1;
            var z = (x - y) * 2 - 0.3;
            z = Math.max(-1.0, Math.min(1.0, z));
            var decision = z >= 0.0 ? 'approved' : 'rejected';

            var sct = {
                x: parseFloat(x.toFixed(4)),
                y: parseFloat(y.toFixed(4)),
                z: parseFloat(z.toFixed(4)),
                decision: decision
            };

            this.tensorsProcessed++;
            this.lastSct = sct;
            this._emit('telemetry', {
                tensors_processed: this.tensorsProcessed,
                tensors_rejected: this.tensorsRejected,
                last_sct: sct,
                queue_size: 0,
                timestamp: Date.now()
            });

            return Promise.resolve({
                task_id: 'local-' + Date.now(),
                success: true,
                sct: sct,
                latency_ms: Date.now() - start
            });
        } catch (err) {
            this.failedCount++;
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

    BrowserNodeManager.prototype._removePending = function (task_id) {
        if (this.pendingPromises[task_id]) {
            delete this.pendingPromises[task_id];
        }
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
