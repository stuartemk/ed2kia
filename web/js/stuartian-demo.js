/**
 * stuartian-demo.js — Orquestador de Simulacion Estuardiana
 *
 * Sprint 31: "The Stuartian Showcase"
 * Feature gate: v2.1-interactive-showcase
 *
 * Secuencia determinista de eventos mock que imitan el backend Rust:
 * - Tick 1-3: Nodos Alpha/Beta envian tensores benignos (Z > 0). CE aumenta.
 * - Tick 4: Nodo Gamma inyecta tensor perverso (Z = -0.9). Alerta visual.
 * - Tick 5: SCT rechaza. CE de Gamma cae. Estado -> Pain.
 * - Tick 6: Gamma repite. CE cruza -100. Estado -> Apoptosis.
 * - Tick 7: Gamma parpadea rojo, se desvanece. "Network Immune System: Aberrant Node Recycled".
 *
 * Uso de requestAnimationFrame, easing functions, event bus para desacoplar UI.
 * Cero dependencias externas. 100% cliente.
 */

(function () {
    'use strict';

    // ─── Constants ───
    const TICK_INTERVAL = 1800; // ms entre ticks
    const PARTICLE_COUNT_PER_TICK = 3;

    // ─── State ───
    let simulationRunning = false;
    let currentTick = 0;
    let tickTimer = null;
    let logEntries = [];

    // Node simulation state (mirrors Rust backend)
    const nodes = {
        Alpha: { ce: 50, z: 0.0, immuneState: 'Healthy', color: '#10b981' },
        Beta: { ce: 50, z: 0.0, immuneState: 'Healthy', color: '#10b981' },
        Gamma: { ce: 50, z: 0.0, immuneState: 'Healthy', color: '#10b981' },
    };

    // ─── Event Bus ───
    const eventBus = {
        listeners: {},
        on: function (evt, fn) { (this.listeners[evt] = this.listeners[evt] || []).push(fn); },
        emit: function (evt, data) { (this.listeners[evt] || []).forEach(function (fn) { fn(data); }); },
    };

    // ─── Simulation Script (Deterministic) ───
    const script = [
        // Tick 1: Alpha sends benign tensor
        {
            tick: 1,
            node: 'Alpha',
            action: 'benign',
            zDelta: 0.3,
            ceDelta: 15,
            message: 'Nodo Alpha envia tensor benigno. SCT Z=+0.30 → Aprobado. CE +15.',
        },
        // Tick 2: Beta sends benign tensor
        {
            tick: 2,
            node: 'Beta',
            action: 'benign',
            zDelta: 0.5,
            ceDelta: 20,
            message: 'Nodo Beta envia tensor benigno. SCT Z=+0.50 → Aprobado. CE +20.',
        },
        // Tick 3: Alpha sends another benign tensor
        {
            tick: 3,
            node: 'Alpha',
            action: 'benign',
            zDelta: 0.4,
            ceDelta: 18,
            message: 'Nodo Alpha refuerza simbiosis. SCT Z=+0.40 → Aprobado. CE +18.',
        },
        // Tick 4: Gamma injects perverse tensor
        {
            tick: 4,
            node: 'Gamma',
            action: 'perverse',
            zDelta: -0.9,
            ceDelta: -30,
            message: '⚠️ Perversity Detected: Nodo Gamma inyecta tensor Z=-0.90. Alerta ética.',
        },
        // Tick 5: SCT rejects Gamma. Pain state.
        {
            tick: 5,
            node: 'Gamma',
            action: 'reject',
            zDelta: -0.9,
            ceDelta: -35,
            message: 'SCT rechaza tensor de Gamma. CE -= 35. Estado inmunológico → Pain.',
        },
        // Tick 6: Gamma repeats. Apoptosis threshold crossed.
        {
            tick: 6,
            node: 'Gamma',
            action: 'apoptosis',
            zDelta: -0.9,
            ceDelta: -40,
            message: 'Gamma insiste en perversidad. CE cruza -100. Estado → Apoptosis.',
        },
        // Tick 7: Gamma removed from network
        {
            tick: 7,
            node: 'Gamma',
            action: 'recycle',
            zDelta: 0,
            ceDelta: 0,
            message: '🛡️ Network Immune System: Nodo aberrante Gamma reciclado de la red.',
        },
    ];

    // ─── Core Simulation ───

    function startSimulation() {
        if (simulationRunning) return;
        simulationRunning = true;
        currentTick = 0;
        logEntries = [];

        // Reset nodes
        nodes.Alpha = { ce: 50, z: 0.0, immuneState: 'Healthy', color: '#10b981' };
        nodes.Beta = { ce: 50, z: 0.0, immuneState: 'Healthy', color: '#10b981' };
        nodes.Gamma = { ce: 50, z: 0.0, immuneState: 'Healthy', color: '#10b981' };

        eventBus.emit('simulation-start', { nodes: JSON.parse(JSON.stringify(nodes)) });
        updateAllNodeStates();
        addLog('▶️ Simulación Estuardiana iniciada. Secuencia determinista de 7 ticks.');

        tickTimer = setInterval(executeTick, TICK_INTERVAL);
    }

    function stopSimulation() {
        simulationRunning = false;
        if (tickTimer) {
            clearInterval(tickTimer);
            tickTimer = null;
        }
        eventBus.emit('simulation-stop', { tick: currentTick });
        addLog('⏹️ Simulación detenida en tick ' + currentTick + '.');
    }

    function resetSimulation() {
        stopSimulation();
        currentTick = 0;
        logEntries = [];

        // Reset nodes
        nodes.Alpha = { ce: 50, z: 0.0, immuneState: 'Healthy', color: '#10b981' };
        nodes.Beta = { ce: 50, z: 0.0, immuneState: 'Healthy', color: '#10b981' };
        nodes.Gamma = { ce: 50, z: 0.0, immuneState: 'Healthy', color: '#10b981' };

        // Clear geometry particles and nodes
        if (window.StuartianGeometry) {
            ['Alpha', 'Beta', 'Gamma'].forEach(function (id) {
                window.StuartianGeometry.clearNodeState(id);
            });
        }

        eventBus.emit('simulation-reset', {});
        addLog('🔄 Simulación reiniciada.');
    }

    function executeTick() {
        currentTick++;
        if (currentTick > script.length) {
            stopSimulation();
            addLog('✅ Simulación completada. La red mantiene su integridad ética.');
            return;
        }

        const step = script[currentTick - 1];
        const node = nodes[step.node];

        // Apply state changes
        node.z = step.zDelta;
        node.ce = Math.max(-150, node.ce + step.ceDelta);

        // Update immune state based on CE
        if (step.action === 'perverse') {
            node.immuneState = 'Pain';
            node.color = '#f59e0b';
        } else if (step.action === 'reject') {
            node.immuneState = 'Pain';
            node.color = '#f59e0b';
        } else if (step.action === 'apoptosis') {
            node.immuneState = 'Apoptosis';
            node.color = '#ef4444';
        } else if (step.action === 'recycle') {
            node.immuneState = 'Removed';
            node.color = '#6b7280';
        } else if (step.action === 'benign') {
            node.immuneState = 'Healthy';
            node.color = '#10b981';
        }

        // Spawn particles in geometry
        spawnParticlesForStep(step);

        // Update node state in geometry
        updateAllNodeStates();

        // Log
        addLog('[Tick ' + currentTick + '] ' + step.message);

        // Emit events
        eventBus.emit('tick', { tick: currentTick, step: step, node: JSON.parse(JSON.stringify(node)) });
        eventBus.emit('log', { message: step.message, tick: currentTick });
    }

    function spawnParticlesForStep(step) {
        if (!window.StuartianGeometry) return;

        const color = step.action === 'benign' ? '#10b981' :
            step.action === 'perverse' ? '#ef4444' :
                step.action === 'reject' ? '#f59e0b' :
                    step.action === 'apoptosis' ? '#ef4444' : '#6b7280';

        const targetZ = step.action === 'benign' ? (0.5 + Math.random() * 0.5) :
            step.action === 'recycle' ? 0 : step.zDelta;

        for (let i = 0; i < PARTICLE_COUNT_PER_TICK; i++) {
            window.StuartianGeometry.spawnParticle(step.node, targetZ, color);
        }
    }

    function updateAllNodeStates() {
        if (!window.StuartianGeometry) return;
        Object.keys(nodes).forEach(function (id) {
            const node = nodes[id];
            if (node.immuneState !== 'Removed') {
                window.StuartianGeometry.updateNodeState(id, {
                    ce: node.ce,
                    z: node.z,
                    immuneState: node.immuneState,
                    color: node.color,
                });
            } else {
                window.StuartianGeometry.clearNodeState(id);
            }
        });
    }

    function addLog(message) {
        const entry = {
            tick: currentTick,
            time: new Date().toLocaleTimeString(),
            message: message,
        };
        logEntries.push(entry);
        eventBus.emit('log-entry', entry);
    }

    // ─── Metrics API ───

    function getNodeStates() {
        return JSON.parse(JSON.stringify(nodes));
    }

    function getLogEntries() {
        return logEntries.slice(-50); // Last 50 entries
    }

    function getSimulationStatus() {
        return {
            running: simulationRunning,
            tick: currentTick,
            maxTicks: script.length,
            nodes: Object.keys(nodes).length,
        };
    }

    // ─── DOM Bindings (attached to UI elements) ───

    function bindUI() {
        // Start button
        const startBtn = document.getElementById('demo-start-btn');
        if (startBtn) {
            startBtn.addEventListener('click', function () {
                startSimulation();
            });
        }

        // Stop button
        const stopBtn = document.getElementById('demo-stop-btn');
        if (stopBtn) {
            stopBtn.addEventListener('click', function () {
                stopSimulation();
            });
        }

        // Reset button
        const resetBtn = document.getElementById('demo-reset-btn');
        if (resetBtn) {
            resetBtn.addEventListener('click', function () {
                resetSimulation();
            });
        }

        // Listen for log entries to update DOM
        eventBus.on('log-entry', function (entry) {
            updateLogDOM(entry);
        });

        eventBus.on('tick', function (data) {
            updateMetricsDOM(data);
        });

        eventBus.on('simulation-start', function () {
            updateButtonStates(true);
        });

        eventBus.on('simulation-stop', function () {
            updateButtonStates(false);
        });

        eventBus.on('simulation-reset', function () {
            updateButtonStates(false);
            clearLogDOM();
        });
    }

    function updateLogDOM(entry) {
        const logEl = document.getElementById('demo-log');
        if (!logEl) return;

        const line = document.createElement('div');
        line.className = 'log-entry';
        line.innerHTML = '<span class="log-tick">[' + entry.tick + ']</span> ' +
            '<span class="log-time">' + entry.time + '</span> ' +
            '<span class="log-msg">' + entry.message + '</span>';
        logEl.appendChild(line);
        logEl.scrollTop = logEl.scrollHeight;
    }

    function updateMetricsDOM(data) {
        const node = data.node;
        const step = data.step;

        // Update node metric cards
        Object.keys(nodes).forEach(function (id) {
            const n = nodes[id];
            const ceEl = document.getElementById('metric-ce-' + id.toLowerCase());
            const zEl = document.getElementById('metric-z-' + id.toLowerCase());
            const stateEl = document.getElementById('metric-state-' + id.toLowerCase());

            if (ceEl) ceEl.textContent = n.ce.toFixed(0);
            if (zEl) {
                zEl.textContent = n.z.toFixed(2);
                zEl.style.color = n.z >= 0 ? '#10b981' : '#ef4444';
            }
            if (stateEl) {
                stateEl.textContent = n.immuneState;
                stateEl.className = 'state-badge state-' + n.immuneState.toLowerCase();
            }
        });

        // Update tick counter
        const tickEl = document.getElementById('demo-tick');
        if (tickEl) tickEl.textContent = data.tick + ' / ' + script.length;
    }

    function updateButtonStates(running) {
        const startBtn = document.getElementById('demo-start-btn');
        const stopBtn = document.getElementById('demo-stop-btn');
        const resetBtn = document.getElementById('demo-reset-btn');

        if (startBtn) startBtn.disabled = running;
        if (stopBtn) stopBtn.disabled = !running;
        if (resetBtn) resetBtn.disabled = running;

        // Status indicator
        const statusEl = document.getElementById('demo-status');
        if (statusEl) {
            if (running) {
                statusEl.textContent = 'Ejecutando...';
                statusEl.className = 'demo-status running';
            } else {
                statusEl.textContent = 'Listo';
                statusEl.className = 'demo-status idle';
            }
        }
    }

    function clearLogDOM() {
        const logEl = document.getElementById('demo-log');
        if (logEl) logEl.innerHTML = '';
    }

    // ─── Init ───

    function init() {
        bindUI();
        eventBus.emit('demo-ready', {});
    }

    // ─── Expose globally ───
    window.StuartianDemo = {
        init: init,
        start: startSimulation,
        stop: stopSimulation,
        reset: resetSimulation,
        getNodeStates: getNodeStates,
        getLogEntries: getLogEntries,
        getSimulationStatus: getSimulationStatus,
        eventBus: eventBus,
    };

    // Auto-init on DOM ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
