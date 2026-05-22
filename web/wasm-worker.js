/**
 * ed2kIA WASM Worker — Non-blocking Background Engine
 *
 * Ley 4 (Simbiosis): Cero bloqueo del Main Thread.
 * Ley 3 (Cero Desperdicio): Loop eficiente con requestAnimationFrame/setTimeout.
 *
 * Feature gate: v2.1-wasm-worker
 *
 * Contrato de Mensajes:
 *   IN:  { type: 'start_node', id?: string, memoryLimitMb?: number }
 *   IN:  { type: 'process_tensor', payload: string }
 *   IN:  { type: 'stop_node' }
 *   OUT: { type: 'node_ready', id, memoryLimitMb }
 *   OUT: { type: 'telemetry', tensors_processed, last_sct: { x, y, z, decision } }
 *   OUT: { type: 'tensor_result', task_id, success, sct: { x, y, z, decision }, latency_ms }
 *   OUT: { type: 'error', message }
 *   OUT: { type: 'node_stopped' }
 *
 * Cero telemetría externa, cero trackers, cero lógica financiera.
 */

'use strict';

// ============================================================================
// State
// ============================================================================

let browserNode = null;
let running = false;
let telemetryInterval = null;
let tensorsProcessed = 0;
let tensorsRejected = 0;
let lastSct = { x: 0, y: 0, z: 0, decision: 'idle' };
let offlineQueue = [];
let nodeId = 'wasm-worker-' + Date.now();
let memoryLimitMb = 64;

// ============================================================================
// SCT Simulation (dummy when WASM not loaded)
// ============================================================================

/**
 * Simula evaluación SCT basada en payload.
 * Devuelve { x, y, z, decision } compatible con StuartianTensor.
 */
function simulateSCTEvaluation(payload) {
    // Deterministic simulation based on payload content
    var seed = 0;
    for (var i = 0; i < payload.length; i++) {
        seed = ((seed << 5) - seed) + payload.charCodeAt(i);
        seed = seed & seed; // Convert to 32bit integer
    }

    // X: Community Benefit [0, 1] — Higher = more beneficial
    var x = Math.abs(Math.sin(seed * 0.01)) * 0.7 + 0.2; // [0.2, 0.9]

    // Y: External Cost [0, 1] — Lower = less extraction
    var y = Math.abs(Math.cos(seed * 0.013)) * 0.5 + 0.1; // [0.1, 0.6]

    // Z: Symbiosis Score [-1, 1] — Positive = approved
    var z = (x - y) * 2 - 0.3; // Bias toward approval for beneficial work
    z = Math.max(-1.0, Math.min(1.0, z));

    var decision = z >= 0.0 ? 'approved' : 'rejected';

    return { x: parseFloat(x.toFixed(4)), y: parseFloat(y.toFixed(4)), z: parseFloat(z.toFixed(4)), decision };
}

// ============================================================================
// WASM Loading (deferred, non-blocking)
// ============================================================================

/**
 * Initializes the BrowserNode from WASM bindings.
 * Falls back to local simulation if WASM fails to load.
 */
async function initWasmNode() {
    try {
        // Try to import WASM bindings (wasm-pack output)
        var wasmModule = await import('./pkg/browser_node.js');
        if (wasmModule && wasmModule.BrowserNode) {
            browserNode = new wasmModule.BrowserNode(nodeId, memoryLimitMb);
            browserNode.init();
            return true;
        }
    } catch (e) {
        // WASM not available — use local simulation
        postMessage({
            type: 'info',
            message: 'WASM not loaded, using local simulation: ' + e.message
        });
    }
    return false;
}

/**
 * Process tensor via WASM or local simulation.
 */
function processTensorLocal(payload) {
    var startMs = Date.now();

    // Use WASM node if available
    if (browserNode && browserNode.processTask) {
        try {
            var result = browserNode.processTask(payload);
            var latency = Date.now() - startMs;
            var sct = simulateSCTEvaluation(payload);
            return { task_id: 'task-' + Date.now(), success: true, sct, latency_ms: latency, result };
        } catch (e) {
            // Fall through to simulation
        }
    }

    // Local simulation fallback
    var sct = simulateSCTEvaluation(payload);
    var latency = Date.now() - startMs;

    if (sct.decision === 'approved') {
        tensorsProcessed++;
    } else {
        tensorsRejected++;
    }

    lastSct = sct;

    return {
        task_id: 'task-' + Date.now(),
        success: sct.decision === 'approved',
        sct: { x: sct.x, y: sct.y, z: sct.z, decision: sct.decision },
        latency_ms: latency
    };
}

// ============================================================================
// Telemetry Loop
// ============================================================================

function startTelemetryLoop() {
    if (telemetryInterval) {
        clearInterval(telemetryInterval);
    }
    telemetryInterval = setInterval(function () {
        if (!running) return;
        postMessage({
            type: 'telemetry',
            tensors_processed: tensorsProcessed,
            tensors_rejected: tensorsRejected,
            last_sct: { x: lastSct.x, y: lastSct.y, z: lastSct.z, decision: lastSct.decision },
            queue_size: offlineQueue.length,
            timestamp: Date.now()
        });
    }, 1000); // 1s interval
}

function stopTelemetryLoop() {
    if (telemetryInterval) {
        clearInterval(telemetryInterval);
        telemetryInterval = null;
    }
}

// ============================================================================
// Flush Offline Queue
// ============================================================================

function flushOfflineQueue() {
    var flushed = 0;
    while (offlineQueue.length > 0 && running) {
        var msg = offlineQueue.shift();
        if (msg && msg.type === 'process_tensor') {
            var result = processTensorLocal(msg.payload);
            postMessage({ type: 'tensor_result', ...result });
            flushed++;
        }
    }
    if (flushed > 0) {
        postMessage({ type: 'queue_flushed', count: flushed });
    }
}

// ============================================================================
// Message Handler
// ============================================================================

self.onmessage = function (e) {
    var msg = e.data;
    if (!msg || !msg.type) return;

    try {
        switch (msg.type) {
            case 'start_node':
                handleStartNode(msg);
                break;
            case 'process_tensor':
                handleProcessTensor(msg);
                break;
            case 'stop_node':
                handleStopNode();
                break;
            default:
                postMessage({ type: 'error', message: 'Unknown message type: ' + msg.type });
        }
    } catch (err) {
        postMessage({ type: 'error', message: err.message || String(err) });
    }
};

function handleStartNode(msg) {
    if (running) {
        postMessage({ type: 'error', message: 'Node already running' });
        return;
    }

    nodeId = msg.id || nodeId;
    memoryLimitMb = Math.max(16, Math.min(512, msg.memoryLimitMb || memoryLimitMb));
    tensorsProcessed = 0;
    tensorsRejected = 0;
    lastSct = { x: 0, y: 0, z: 0, decision: 'starting' };
    running = true;

    // Init WASM (async, non-blocking)
    initWasmNode().then(function (loaded) {
        postMessage({
            type: 'node_ready',
            id: nodeId,
            memoryLimitMb: memoryLimitMb,
            wasm_loaded: loaded
        });
        startTelemetryLoop();
        flushOfflineQueue();
    }).catch(function (err) {
        postMessage({ type: 'error', message: 'Node init failed: ' + err.message });
        running = false;
    });
}

function handleProcessTensor(msg) {
    if (!msg.payload) {
        postMessage({ type: 'error', message: 'Missing payload in process_tensor' });
        return;
    }

    if (!running) {
        // Queue for later
        offlineQueue.push(msg);
        if (offlineQueue.length > 128) {
            offlineQueue.shift(); // Drop oldest
        }
        postMessage({ type: 'queued', queue_size: offlineQueue.length });
        return;
    }

    var result = processTensorLocal(msg.payload);
    postMessage({ type: 'tensor_result', ...result });
}

function handleStopNode() {
    running = false;
    stopTelemetryLoop();

    if (browserNode && browserNode.clearQueue) {
        try { browserNode.clearQueue(); } catch (_) { /* ignore */ }
    }

    offlineQueue = [];
    lastSct = { x: 0, y: 0, z: 0, decision: 'stopped' };

    postMessage({
        type: 'node_stopped',
        id: nodeId,
        final_stats: {
            tensors_processed: tensorsProcessed,
            tensors_rejected: tensorsRejected
        }
    });
}

// ============================================================================
// Error Handler
// ============================================================================

self.onerror = function (event) {
    postMessage({
        type: 'error',
        message: 'Worker error: ' + (event.message || 'unknown'),
        filename: event.filename,
        lineno: event.lineno,
        colno: event.colno
    });
    return true; // Prevent default error handling
};

// Initial handshake
postMessage({ type: 'worker_ready', id: nodeId });
