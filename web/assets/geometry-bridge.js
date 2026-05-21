/**
 * geometry-bridge.js — Puente de Datos para Visualización 3D Estuardiana
 *
 * Consume GET /api/metrics, parsea sct_z_distribution,
 * y actualiza el canvas #stuartian-3d-canvas con requestAnimationFrame.
 *
 * Feature gate: v2.1-3d-viz
 *
 * Diseño:
 * - Proyección 3D→2D manual (sin Three.js)
 * - Rotación por matriz de Euler
 * - Dibujo de octaedro (8 caras, 6 vértices)
 * - Partículas con fricción y aceleración gravitacional
 * - Debounce 500ms, lazy loading vía visibility API
 * - Cero telemetría externa
 */

(function () {
    'use strict';

    // ─── Constants ───
    const CANVAS_ID = 'stuartian-3d-canvas';
    const API_ENDPOINT = '/api/metrics';
    const DEBOUNCE_MS = 500;
    const PARTICLE_COUNT = 40;
    const FRICTION = 0.92;
    const GRAVITY_STRENGTH = 0.003;

    // Colores del octaedro
    const COLOR_FOCO_SUPERIOR = '#00BFFF'; // Azul cielo — Autonomía
    const COLOR_FOCO_INFERIOR = '#8B0000'; // Rojo oscuro — Extracción
    const COLOR_ECUADOR = '#888888';       // Gris — Ilusión binaria
    const COLOR_PARTICLE_APPROVED = '#10b981';
    const COLOR_PARTICLE_REJECTED = '#ef4444';
    const COLOR_PARTICLE_NEUTRAL = '#3b82f6';

    // ─── State ───
    let canvas = null;
    let ctx = null;
    let width = 0;
    let height = 0;
    let rotationX = 0;
    let rotationY = 0;
    let autoRotate = true;
    let particles = [];
    let zDistribution = { positive: 60, neutral: 30, negative: 10 };
    let lastFetch = 0;
    let animFrameId = null;
    let isVisible = true;

    // ─── Octahedron Vertices (3D) ───
    const vertices = [
        { x: 0, y: 0, z: 1 },   // Foco Superior
        { x: 0, y: 0, z: -1 },  // Foco Inferior
        { x: 1, y: 0, z: 0 },   // Ecuador X+
        { x: -1, y: 0, z: 0 },  // Ecuador X-
        { x: 0, y: 1, z: 0 },   // Ecuador Y+
        { x: 0, y: -1, z: 0 },  // Ecuador Y-
    ];

    // 8 caras del octaedro (índices de vértices)
    const faces = [
        [0, 2, 4], [0, 4, 1], [0, 1, 3], [0, 3, 2], // Superior (4 caras)
        [1, 2, 5], [1, 5, 4], [1, 4, 3], [1, 3, 2], // Inferior (4 caras)
    ];

    // ─── 3D Math Utilities ───

    /**
     * Rotación sobre eje X (ángulo en radianes).
     */
    function rotateX(point, angle) {
        const cos = Math.cos(angle);
        const sin = Math.sin(angle);
        return {
            x: point.x,
            y: point.y * cos - point.z * sin,
            z: point.y * sin + point.z * cos,
        };
    }

    /**
     * Rotación sobre eje Y (ángulo en radianes).
     */
    function rotateY(point, angle) {
        const cos = Math.cos(angle);
        const sin = Math.sin(angle);
        return {
            x: point.x * cos + point.z * sin,
            y: point.y,
            z: -point.x * sin + point.z * cos,
        };
    }

    /**
     * Proyección perspectiva 3D → 2D.
     */
    function project(point, scale) {
        const perspective = 4;
        const factor = perspective / (perspective + point.z);
        return {
            x: width / 2 + point.x * scale * factor,
            y: height / 2 - point.y * scale * factor, // Y invertido (canvas)
            factor,
        };
    }

    /**
     * Transforma un punto 3D a coordenadas 2D del canvas.
     */
    function transform3D(point) {
        let p = rotateX(point, rotationX);
        p = rotateY(p, rotationY);
        const scale = Math.min(width, height) * 0.3;
        return project(p, scale);
    }

    // ─── Particle System ───

    /**
     * Crea una partícula que nace en (0,0,0) y se mueve hacia Z
     * según la distribución actual.
     */
    function createParticle() {
        // Determinar destino Z basado en distribución
        const rand = Math.random() * 100;
        let targetZ;
        if (rand < zDistribution.negative) {
            targetZ = -0.3 - Math.random() * 0.7; // Foco Inferior
        } else if (rand < zDistribution.negative + zDistribution.neutral) {
            targetZ = (Math.random() - 0.5) * 0.2; // Ecuador
        } else {
            targetZ = 0.3 + Math.random() * 0.7; // Foco Superior
        }

        return {
            x: (Math.random() - 0.5) * 0.3,
            y: (Math.random() - 0.5) * 0.3,
            z: 0,
            vx: 0,
            vy: 0,
            vz: 0,
            targetZ,
            life: 1.0,
            decay: 0.002 + Math.random() * 0.003,
        };
    }

    /**
     * Actualiza todas las partículas con fricción y gravedad focal.
     */
    function updateParticles() {
        for (let i = particles.length - 1; i >= 0; i--) {
            const p = particles[i];

            // Gravedad hacia targetZ
            const dz = p.targetZ - p.z;
            p.vz += dz * GRAVITY_STRENGTH;

            // Fricción
            p.vx *= FRICTION;
            p.vy *= FRICTION;
            p.vz *= FRICTION;

            // Actualizar posición
            p.x += p.vx;
            p.y += p.vy;
            p.z += p.vz;

            // Clamp Z a [-1, 1]
            p.z = Math.max(-1, Math.min(1, p.z));

            // Decaimiento de vida
            p.life -= p.decay;

            // Eliminar partículas muertas
            if (p.life <= 0) {
                particles.splice(i, 1);
            }
        }

        // Mantener PARTICLE_COUNT partículas
        while (particles.length < PARTICLE_COUNT) {
            particles.push(createParticle());
        }
    }

    // ─── Rendering ───

    /**
     * Dibuja una línea 3D proyectada.
     */
    function drawLine3D(p1, p2, color, lineWidth) {
        const a = transform3D(p1);
        const b = transform3D(p2);
        ctx.beginPath();
        ctx.moveTo(a.x, a.y);
        ctx.lineTo(b.x, b.y);
        ctx.strokeStyle = color;
        ctx.lineWidth = lineWidth || 1.5;
        ctx.stroke();
    }

    /**
     * Dibuja los 6 vértices del octaedro.
     */
    function drawVertices() {
        const colors = [
            COLOR_FOCO_SUPERIOR, // 0: Superior
            COLOR_FOCO_INFERIOR, // 1: Inferior
            COLOR_ECUADOR,       // 2: X+
            COLOR_ECUADOR,       // 3: X-
            COLOR_ECUADOR,       // 4: Y+
            COLOR_ECUADOR,       // 5: Y-
        ];

        for (let i = 0; i < vertices.length; i++) {
            const p = transform3D(vertices[i]);
            const radius = 4 * p.factor;

            ctx.beginPath();
            ctx.arc(p.x, p.y, radius, 0, Math.PI * 2);
            ctx.fillStyle = colors[i];
            ctx.fill();

            // Glow effect para focos principales
            if (i === 0 || i === 1) {
                ctx.beginPath();
                ctx.arc(p.x, p.y, radius * 2.5, 0, Math.PI * 2);
                const gradient = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, radius * 2.5);
                gradient.addColorStop(0, colors[i] + '60');
                gradient.addColorStop(1, colors[i] + '00');
                ctx.fillStyle = gradient;
                ctx.fill();
            }
        }
    }

    /**
     * Dibuja las aristas del octaedro.
     */
    function drawEdges() {
        // Aristas del Foco Superior al Ecuador
        for (let i = 2; i <= 5; i++) {
            drawLine3D(vertices[0], vertices[i], COLOR_FOCO_SUPERIOR + '80', 1.5);
        }

        // Aristas del Foco Inferior al Ecuador
        for (let i = 2; i <= 5; i++) {
            drawLine3D(vertices[1], vertices[i], COLOR_FOCO_INFERIOR + '80', 1.5);
        }

        // Aristas del Ecuador (cuadrado)
        const ecuadorIndices = [2, 4, 3, 5, 2];
        for (let i = 0; i < ecuadorIndices.length - 1; i++) {
            drawLine3D(vertices[ecuadorIndices[i]], vertices[ecuadorIndices[i + 1]], COLOR_ECUADOR + '60', 1);
        }
    }

    /**
     * Dibuja las partículas con colores según región focal.
     */
    function drawParticles() {
        for (const p of particles) {
            const point = { x: p.x * 0.8, y: p.y * 0.8, z: p.z };
            const projected = transform3D(point);
            const radius = 2.5 * projected.factor * p.life;

            let color;
            if (p.z > 0.1) {
                color = COLOR_PARTICLE_APPROVED;
            } else if (p.z < -0.1) {
                color = COLOR_PARTICLE_REJECTED;
            } else {
                color = COLOR_PARTICLE_NEUTRAL;
            }

            ctx.beginPath();
            ctx.arc(projected.x, projected.y, radius, 0, Math.PI * 2);
            ctx.fillStyle = color + Math.floor(p.life * 200).toString(16).padStart(2, '0');
            ctx.fill();
        }
    }

    /**
     * Dibuja etiquetas de los focos.
     */
    function drawLabels() {
        ctx.font = '11px Inter, sans-serif';
        ctx.textAlign = 'center';

        const sup = transform3D(vertices[0]);
        ctx.fillStyle = COLOR_FOCO_SUPERIOR;
        ctx.fillText('Foco Superior (Z=+1)', sup.x, sup.y - 15);

        const inf = transform3D(vertices[1]);
        ctx.fillStyle = COLOR_FOCO_INFERIOR;
        ctx.fillText('Foco Inferior (Z=-1)', inf.x, inf.y + 22);
    }

    /**
     * Dibuja el ecuador como plano semitransparente.
     */
    function drawEcuadorPlane() {
        const ecuadorVerts = [vertices[2], vertices[4], vertices[3], vertices[5]];
        const projected = ecuadorVerts.map(v => transform3D(v));

        ctx.beginPath();
        ctx.moveTo(projected[0].x, projected[0].y);
        for (let i = 1; i < projected.length; i++) {
            ctx.lineTo(projected[i].x, projected[i].y);
        }
        ctx.closePath();
        ctx.fillStyle = 'rgba(136, 136, 136, 0.05)';
        ctx.fill();
        ctx.strokeStyle = 'rgba(136, 136, 136, 0.2)';
        ctx.lineWidth = 0.5;
        ctx.stroke();
    }

    /**
     * Loop principal de renderizado.
     */
    function render() {
        // Limpiar canvas
        ctx.clearRect(0, 0, width, height);

        // Auto-rotación
        if (autoRotate && isVisible) {
            rotationY += 0.005;
        }

        // Actualizar partículas
        if (isVisible) {
            updateParticles();
        }

        // Dibujar en orden: plano ecuador → partículas → aristas → vértices → etiquetas
        drawEcuadorPlane();
        drawParticles();
        drawEdges();
        drawVertices();
        drawLabels();

        animFrameId = requestAnimationFrame(render);
    }

    // ─── Data Fetching ───

    /**
     * Fetch seguro con AbortController y timeout.
     */
    async function safeFetch(url, timeoutMs) {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
        try {
            const response = await fetch(url, { signal: controller.signal });
            clearTimeout(timeoutId);
            if (!response.ok) return null;
            return response.json();
        } catch {
            clearTimeout(timeoutId);
            return null;
        }
    }

    /**
     * Polling de /api/metrics con debounce.
     */
    async function fetchMetrics() {
        const now = Date.now();
        if (now - lastFetch < DEBOUNCE_MS) return;
        lastFetch = now;

        const data = await safeFetch(API_ENDPOINT, 3000);
        if (data && data.sct_z_distribution) {
            const dist = data.sct_z_distribution;
            zDistribution.positive = dist.positive ?? zDistribution.positive;
            zDistribution.neutral = dist.neutral ?? zDistribution.neutral;
            zDistribution.negative = dist.negative ?? zDistribution.negative;
        }
    }

    /**
     * Loop de polling con lazy loading.
     */
    function startPolling() {
        async function poll() {
            if (isVisible) {
                await fetchMetrics();
            }
            setTimeout(poll, DEBOUNCE_MS);
        }
        poll();
    }

    // ─── Initialization ───

    /**
     * Inicializa el canvas y comienza el renderizado.
     */
    function init() {
        canvas = document.getElementById(CANVAS_ID);
        if (!canvas) {
            console.warn('[geometry-bridge] Canvas #stuartian-3d-canvas not found');
            return;
        }

        ctx = canvas.getContext('2d');
        if (!ctx) {
            console.warn('[geometry-bridge] Cannot get 2D context');
            return;
        }

        // Resize observer
        function resize() {
            const rect = canvas.getBoundingClientRect();
            width = rect.width;
            height = rect.height;
            canvas.width = width * window.devicePixelRatio;
            canvas.height = height * window.devicePixelRatio;
            ctx.scale(window.devicePixelRatio, window.devicePixelRatio);
        }
        resize();
        window.addEventListener('resize', resize);

        // Lazy loading vía visibility API
        document.addEventListener('visibilitychange', () => {
            isVisible = document.visibilityState === 'visible';
        });

        // Mouse drag para rotación manual
        let isDragging = false;
        let lastMouseX = 0;
        let lastMouseY = 0;

        canvas.addEventListener('mousedown', (e) => {
            isDragging = true;
            lastMouseX = e.clientX;
            lastMouseY = e.clientY;
            autoRotate = false;
        });

        window.addEventListener('mousemove', (e) => {
            if (!isDragging) return;
            const dx = e.clientX - lastMouseX;
            const dy = e.clientY - lastMouseY;
            rotationY += dx * 0.01;
            rotationX += dy * 0.01;
            lastMouseX = e.clientX;
            lastMouseY = e.clientY;
        });

        window.addEventListener('mouseup', () => {
            isDragging = false;
        });

        // Double click para resetear rotación
        canvas.addEventListener('dblclick', () => {
            rotationX = 0;
            rotationY = 0;
            autoRotate = true;
        });

        // Iniciar polling y render
        startPolling();
        render();

        console.log('[geometry-bridge] Stuartian 3D visualization initialized');
    }

    // Exponer API pública
    window.StuartianGeometryBridge = {
        init,
        setDistribution: (dist) => {
            zDistribution = { ...zDistribution, ...dist };
        },
        getDistribution: () => ({ ...zDistribution }),
        setAutoRotate: (enabled) => { autoRotate = enabled; },
    };

    // Auto-init cuando DOM está listo
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
