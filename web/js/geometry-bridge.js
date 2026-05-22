/**
 * geometry-bridge.js — Motor de Renderizado 3D del Octaedro Estuardiano
 *
 * Sprint 31: "The Stuartian Showcase"
 * Feature gate: v2.1-interactive-showcase
 *
 * Renderizado Canvas 2D/3D ligero (sin dependencias externas).
 * - Proyección 3D->2D manual con matriz de rotacion Euler
 * - Octaedro completo (8 caras, 6 vertices, 12 aristas)
 * - Sistema de particulas con "gravedad etica" en eje Z
 * - Gradientes dinamicos: Foco Superior (#00ffcc) / Foco Inferior (#ff3333)
 * - Interpolacion suave (easeInOutCubic) para transiciones de estado
 * - Tooltips interactivos sobre ejes X/Y/Z
 *
 * Cero telemetria externa. 100% cliente.
 */

(function () {
    'use strict';

    // ─── Constants ───
    const CANVAS_ID = 'stuartian-octahedron';
    const COLOR_FOCO_SUPERIOR = '#00ffcc';
    const COLOR_FOCO_INFERIOR = '#ff3333';
    const COLOR_EQUATOR = '#4a5568';
    const COLOR_PARTICLE_APPROVED = '#10b981';
    const COLOR_PARTICLE_REJECTED = '#ef4444';
    const COLOR_PARTICLE_NEUTRAL = '#3b82f6';
    const COLOR_EDGE = 'rgba(148, 163, 184, 0.3)';
    const COLOR_VERTEX_GLOW = 'rgba(0, 255, 204, 0.6)';
    const PARTICLE_FRICTION = 0.93;
    const GRAVITY_STRENGTH = 0.004;
    const AUTO_ROTATE_SPEED = 0.003;

    // ─── State ───
    let canvas = null;
    let ctx = null;
    let width = 0;
    let height = 0;
    let rotationX = 0;
    let rotationY = 0;
    let autoRotate = true;
    let particles = [];
    let animFrameId = null;
    let isVisible = true;
    let mouseDown = false;
    let lastMouseX = 0;
    let lastMouseY = 0;
    let hoveredAxis = null;
    let nodeStates = {}; // { nodeId: { ce, z, immuneState, x, y } }
    let eventBus = { listeners: {}, on: function (evt, fn) { (this.listeners[evt] = this.listeners[evt] || []).push(fn); }, emit: function (evt, data) { (this.listeners[evt] || []).forEach(fn => fn(data)); } };

    // ─── Octahedron Vertices (3D unit) ───
    const vertices = [
        { x: 0, y: 1, z: 0, label: 'Foco Superior', color: COLOR_FOCO_SUPERIOR },   // 0: Top
        { x: 0, y: -1, z: 0, label: 'Foco Inferior', color: COLOR_FOCO_INFERIOR },  // 1: Bottom
        { x: 1, y: 0, z: 0, label: 'X+ (Autonomia)', color: COLOR_EQUATOR },        // 2: X+
        { x: -1, y: 0, z: 0, label: 'X- (Autonomia)', color: COLOR_EQUATOR },       // 3: X-
        { x: 0, y: 0, z: 1, label: 'Y+ (Extraccion)', color: COLOR_EQUATOR },       // 4: Y+
        { x: 0, y: 0, z: -1, label: 'Y- (Extraccion)', color: COLOR_EQUATOR },      // 5: Y-
    ];

    // 12 edges connecting vertices
    const edges = [
        [0, 2], [0, 3], [0, 4], [0, 5], // Top to equator
        [1, 2], [1, 3], [1, 4], [1, 5], // Bottom to equator
        [2, 4], [4, 3], [3, 5], [5, 2], // Equator ring
    ];

    // 8 faces (for potential fill)
    const faces = [
        [0, 2, 4], [0, 4, 3], [0, 3, 5], [0, 5, 2], // Top pyramid
        [1, 4, 2], [1, 3, 4], [1, 5, 3], [1, 2, 5], // Bottom pyramid
    ];

    // ─── 3D Math ───

    function rotateX(point, angle) {
        const cos = Math.cos(angle);
        const sin = Math.sin(angle);
        return { x: point.x, y: point.y * cos - point.z * sin, z: point.y * sin + point.z * cos };
    }

    function rotateY(point, angle) {
        const cos = Math.cos(angle);
        const sin = Math.sin(angle);
        return { x: point.x * cos + point.z * sin, y: point.y, z: -point.x * sin + point.z * cos };
    }

    function project(point, scale) {
        const perspective = 4;
        const factor = perspective / (perspective + point.z);
        return {
            x: width / 2 + point.x * scale * factor,
            y: height / 2 - point.y * scale * factor,
            z: point.z,
            factor,
        };
    }

    function transform3D(point) {
        let p = rotateX(point, rotationX);
        p = rotateY(p, rotationY);
        const scale = Math.min(width, height) * 0.28;
        return project(p, scale);
    }

    // ─── Easing ───

    function easeInOutCubic(t) {
        return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
    }

    function lerp(a, b, t) {
        return a + (b - a) * t;
    }

    // ─── Particle System ───

    function createParticle(nodeId, targetZ, color) {
        return {
            id: nodeId || 'anon',
            x: (Math.random() - 0.5) * 0.4,
            y: 0,
            z: 0,
            vx: 0,
            vy: 0,
            vz: 0,
            targetZ: targetZ,
            color: color || COLOR_PARTICLE_NEUTRAL,
            life: 1.0,
            decay: 0.003 + Math.random() * 0.002,
            birth: performance.now(),
        };
    }

    function spawnParticle(nodeId, z, color) {
        particles.push(createParticle(nodeId, z, color));
    }

    function updateParticles() {
        for (let i = particles.length - 1; i >= 0; i--) {
            const p = particles[i];
            const dz = p.targetZ - p.z;
            p.vz += dz * GRAVITY_STRENGTH;
            p.vx *= PARTICLE_FRICTION;
            p.vy *= PARTICLE_FRICTION;
            p.vz *= PARTICLE_FRICTION;
            p.x += p.vx;
            p.y += p.vy;
            p.z += p.vz;
            p.z = Math.max(-1, Math.min(1, p.z));
            p.life -= p.decay;
            if (p.life <= 0) {
                particles.splice(i, 1);
            }
        }
    }

    // ─── Rendering ───

    function drawGlow(x, y, radius, color) {
        const gradient = ctx.createRadialGradient(x, y, 0, x, y, radius);
        gradient.addColorStop(0, color);
        gradient.addColorStop(1, 'transparent');
        ctx.beginPath();
        ctx.arc(x, y, radius, 0, Math.PI * 2);
        ctx.fillStyle = gradient;
        ctx.fill();
    }

    function drawEdge(p1, p2, color, lineWidth) {
        const a = transform3D(p1);
        const b = transform3D(p2);
        ctx.beginPath();
        ctx.moveTo(a.x, a.y);
        ctx.lineTo(b.x, b.y);
        ctx.strokeStyle = color || COLOR_EDGE;
        ctx.lineWidth = lineWidth || 1.5;
        ctx.stroke();
    }

    function drawVertex(point, label, color) {
        const p = transform3D(point);
        const radius = 4 + p.factor * 2;

        // Glow
        drawGlow(p.x, p.y, radius * 3, (color || COLOR_VERTEX_GLOW) + '40');

        // Dot
        ctx.beginPath();
        ctx.arc(p.x, p.y, radius, 0, Math.PI * 2);
        ctx.fillStyle = color || COLOR_VERTEX_GLOW;
        ctx.fill();

        // Label
        if (label) {
            ctx.font = '11px Inter, sans-serif';
            ctx.fillStyle = 'rgba(226, 232, 240, 0.8)';
            ctx.textAlign = 'center';
            ctx.fillText(label, p.x, p.y - radius - 6);
        }
    }

    function drawParticle(p) {
        const pt = transform3D({ x: p.x, y: p.y, z: p.z });
        const radius = 2.5 * pt.factor * p.life;
        const alpha = Math.floor(p.life * 255).toString(16).padStart(2, '0');

        ctx.beginPath();
        ctx.arc(pt.x, pt.y, radius, 0, Math.PI * 2);
        ctx.fillStyle = p.color + alpha;
        ctx.fill();

        // Node ID label for particles
        if (p.life > 0.6 && radius > 1.5) {
            ctx.font = '9px monospace';
            ctx.fillStyle = 'rgba(226, 232, 240, ' + p.life * 0.7 + ')';
            ctx.textAlign = 'left';
            ctx.fillText(p.id, pt.x + radius + 3, pt.y + 3);
        }
    }

    function drawAxisLabels() {
        const labels = [
            { point: { x: 0, y: 1.4, z: 0 }, text: 'Z+ Alineacion Etica', color: COLOR_FOCO_SUPERIOR },
            { point: { x: 0, y: -1.4, z: 0 }, text: 'Z- Perversidad', color: COLOR_FOCO_INFERIOR },
            { point: { x: 1.4, y: 0, z: 0 }, text: 'X+ Autonomia', color: '#10b981' },
            { point: { x: -1.4, y: 0, z: 0 }, text: 'X- Dependencia', color: '#f59e0b' },
            { point: { x: 0, y: 0, z: 1.4 }, text: 'Y+ Extraccion', color: '#8b5cf6' },
            { point: { x: 0, y: 0, z: -1.4 }, text: 'Y- Contribucion', color: '#06b6d4' },
        ];

        labels.forEach(function (l) {
            const p = transform3D(l.point);
            ctx.font = 'bold 11px Inter, sans-serif';
            ctx.fillStyle = l.color + 'cc';
            ctx.textAlign = 'center';
            ctx.fillText(l.text, p.x, p.y);
        });
    }

    function drawNodeStates() {
        const ids = Object.keys(nodeStates);
        const cols = Math.min(3, ids.length);
        const startX = width - 280;
        const startY = 10;

        if (startX < width / 2 + 200) return; // Not enough space

        ids.forEach(function (id, i) {
            const state = nodeStates[id];
            const col = i % cols;
            const row = Math.floor(i / cols);
            const x = startX + col * 90;
            const y = startY + row * 60;

            // Node box
            ctx.fillStyle = 'rgba(17, 24, 39, 0.85)';
            ctx.strokeStyle = stateColor(state.immuneState);
            ctx.lineWidth = 1.5;
            roundRect(ctx, x, y, 82, 52, 6);
            ctx.fill();
            ctx.stroke();

            // Node ID
            ctx.font = 'bold 10px monospace';
            ctx.fillStyle = '#e2e8f0';
            ctx.textAlign = 'left';
            ctx.fillText(id, x + 5, y + 14);

            // CE
            ctx.font = '9px monospace';
            ctx.fillStyle = '#94a3b8';
            ctx.fillText('CE:' + state.ce.toFixed(0), x + 5, y + 27);

            // Z
            ctx.fillStyle = state.z >= 0 ? '#10b981' : '#ef4444';
            ctx.fillText('Z:' + state.z.toFixed(2), x + 5, y + 38);

            // State
            ctx.fillStyle = stateColor(state.immuneState);
            ctx.fillText(state.immuneState, x + 45, y + 27);
        });
    }

    function stateColor(state) {
        switch (state) {
            case 'Healthy': return '#10b981';
            case 'Pain': return '#f59e0b';
            case 'Apoptosis': return '#ef4444';
            default: return '#94a3b8';
        }
    }

    function roundRect(ctx, x, y, w, h, r) {
        ctx.beginPath();
        ctx.moveTo(x + r, y);
        ctx.lineTo(x + w - r, y);
        ctx.quadraticCurveTo(x + w, y, x + w, y + r);
        ctx.lineTo(x + w, y + h - r);
        ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
        ctx.lineTo(x + r, y + h);
        ctx.quadraticCurveTo(x, y + h, x, y + h - r);
        ctx.lineTo(x, y + r);
        ctx.quadraticCurveTo(x, y, x + r, y);
        ctx.closePath();
    }

    function drawTooltip() {
        if (!hoveredAxis) return;
        const info = {
            'Z+': { title: 'Eje Z+: Alineacion Etica', desc: 'Tensores con Z>0 son atraidos al Foco Superior. Representan compute etico, beneficio comunitario y simbiosis verificable.' },
            'Z-': { title: 'Eje Z-: Perversidad', desc: 'Tensores con Z<0 caen al Foco Inferior. CE se quema, estado inmunologico degrada: Healthy -> Pain -> Apoptosis.' },
            'X': { title: 'Eje X: Autonomia vs Dependencia', desc: 'Medida de independencia computacional. X+ = nodo autonomo, X- = dependencia extractiva.' },
            'Y': { title: 'Eje Y: Extraccion vs Contribucion', desc: 'Balance entre lo tomado y lo dado a la red. Y+ = extraccion, Y- = contribucion simbiotica.' },
        };
        const data = info[hoveredAxis];
        if (!data) return;

        const tx = 10;
        const ty = height - 80;
        ctx.fillStyle = 'rgba(17, 24, 39, 0.95)';
        ctx.strokeStyle = '#2d3a52';
        ctx.lineWidth = 1;
        roundRect(ctx, tx, ty, 300, 70, 8);
        ctx.fill();
        ctx.stroke();

        ctx.font = 'bold 11px Inter, sans-serif';
        ctx.fillStyle = '#e2e8f0';
        ctx.textAlign = 'left';
        ctx.fillText(data.title, tx + 10, ty + 20);

        ctx.font = '10px Inter, sans-serif';
        ctx.fillStyle = '#94a3b8';
        const words = data.desc.split(' ');
        let line = '';
        let ly = ty + 38;
        words.forEach(function (w) {
            const test = line + w + ' ';
            if (ctx.measureText(test).width > 280) {
                ctx.fillText(line.trim(), tx + 10, ly);
                line = w + ' ';
                ly += 14;
            } else {
                line = test;
            }
        });
        ctx.fillText(line.trim(), tx + 10, ly);
    }

    function render() {
        if (!ctx || !isVisible) return;

        // Clear with subtle gradient
        const bg = ctx.createLinearGradient(0, 0, 0, height);
        bg.addColorStop(0, '#0a0e17');
        bg.addColorStop(1, '#111827');
        ctx.fillStyle = bg;
        ctx.fillRect(0, 0, width, height);

        // Draw edges
        edges.forEach(function (e) {
            drawEdge(vertices[e[0]], vertices[e[1]]);
        });

        // Draw vertices with labels
        vertices.forEach(function (v, i) {
            const label = (i === 0 || i === 1) ? v.label : null;
            drawVertex(v, label, v.color);
        });

        // Draw axis labels
        drawAxisLabels();

        // Draw particles
        particles.forEach(function (p) {
            drawParticle(p);
        });

        // Draw node states panel
        drawNodeStates();

        // Draw tooltip
        drawTooltip();
    }

    // ─── Animation Loop ───

    function animate() {
        if (autoRotate) {
            rotationY += AUTO_ROTATE_SPEED;
        }
        updateParticles();
        render();
        animFrameId = requestAnimationFrame(animate);
    }

    // ─── Mouse Interaction ───

    function setupInteraction() {
        canvas.addEventListener('mousedown', function (e) {
            mouseDown = true;
            lastMouseX = e.clientX;
            lastMouseY = e.clientY;
            autoRotate = false;
        });

        window.addEventListener('mouseup', function () {
            mouseDown = false;
        });

        window.addEventListener('mousemove', function (e) {
            if (mouseDown) {
                const dx = e.clientX - lastMouseX;
                const dy = e.clientY - lastMouseY;
                rotationY += dx * 0.01;
                rotationX += dy * 0.01;
                lastMouseX = e.clientX;
                lastMouseY = e.clientY;
            }

            // Hover detection for axis tooltips
            const rect = canvas.getBoundingClientRect();
            const mx = e.clientX - rect.left;
            const my = e.clientY - rect.top;
            hoveredAxis = detectAxisHover(mx, my);
        });

        canvas.addEventListener('dblclick', function () {
            autoRotate = true;
            rotationX = 0;
            rotationY = 0;
        });
    }

    function detectAxisHover(mx, my) {
        // Simple zone detection based on canvas quadrants
        const cx = width / 2;
        const cy = height / 2;
        if (my < cy * 0.5) return 'Z+';
        if (my > cy * 1.5) return 'Z-';
        if (mx < cx * 0.5) return 'X';
        if (mx > cx * 1.5) return 'X';
        return null;
    }

    // ─── Visibility API ───

    function setupVisibility() {
        document.addEventListener('visibilitychange', function () {
            isVisible = !document.hidden;
        });
    }

    // ─── Resize ───

    function resize() {
        if (!canvas) return;
        const rect = canvas.parentElement.getBoundingClientRect();
        width = rect.width;
        height = Math.max(400, rect.height);
        canvas.width = width * window.devicePixelRatio;
        canvas.height = height * window.devicePixelRatio;
        canvas.style.width = width + 'px';
        canvas.style.height = height + 'px';
        ctx = canvas.getContext('2d');
        ctx.scale(window.devicePixelRatio, window.devicePixelRatio);
    }

    // ─── Public API ───

    function init(canvasId) {
        canvas = document.getElementById(canvasId || CANVAS_ID);
        if (!canvas) {
            console.warn('[geometry-bridge] Canvas #' + (canvasId || CANVAS_ID) + ' not found');
            return;
        }
        ctx = canvas.getContext('2d');
        resize();
        setupInteraction();
        setupVisibility();
        window.addEventListener('resize', resize);
        animate();
        eventBus.emit('ready', { canvas: canvas });
    }

    function updateNodeState(nodeId, state) {
        nodeStates[nodeId] = state;
        eventBus.emit('node-update', { id: nodeId, state: state });
    }

    function clearNodeState(nodeId) {
        delete nodeStates[nodeId];
    }

    function setAutoRotate(enabled) {
        autoRotate = enabled;
    }

    // ─── Expose globally ───
    window.StuartianGeometry = {
        init: init,
        spawnParticle: spawnParticle,
        updateNodeState: updateNodeState,
        clearNodeState: clearNodeState,
        setAutoRotate: setAutoRotate,
        eventBus: eventBus,
        easeInOutCubic: easeInOutCubic,
    };
})();
