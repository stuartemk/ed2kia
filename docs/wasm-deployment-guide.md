# WASM Deployment Guide — ed2kIA Browser Node (v2.1.0-sprint24)

> **Objetivo:** Habilitar la participaci贸n real de voluntarios desde cualquier navegador moderno, cumpliendo la Ley 4 (Simbiosis Existencial) y Ley 1 (Diversidad Comunitaria) sin centralizaci贸n ni l贸gica financiera.

---

## 1. Requisitos

### 1.1 Herramientas de Compilaci贸n

| Herramienta | Versi贸n M铆nima | Prop贸sito |
|---|---|---|
| Rust Toolchain | 1.75.0+ | Compilaci贸n base |
| `wasm32-unknown-unknown` target | — | Compilaci贸n WASM |
| `wasm-pack` | 0.12.0+ | Empaquetado WASM + JS bindings |
| Node.js | 18.0.0+ | Servido est谩tico + pruebas |
| Python 3 | 3.10+ | Scripts de automatizaci贸n (opcional) |

### 1.2 Instalaci贸n de Dependencias

```bash
# Agregar target WASM
rustup target add wasm32-unknown-unknown

# Instalar wasm-pack (Linux/macOS)
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Instalar wasm-pack (Windows PowerShell)
winget install wasm-pack  # O usar cargo install wasm-pack
```

### 1.3 Verificaci贸n

```bash
rustc --version          # >= 1.75.0
wasm-pack --version      # >= 0.12.0
node --version           # >= 18.0.0
rustup target list | grep wasm32  # wasm32-unknown-unknown (installed)
```

---

## 2. Compilaci贸n

### 2.1 Compilaci贸n WASM (Browser Node)

```bash
# Compilar con feature v2.1-wasm-browser
wasm-pack build --release \
  --target web \
  --features v2.1-wasm-browser \
  --out-dir pkg
```

**Output:**
- `pkg/browser_node_bg.wasm` — Binario WASM optimizado
- `pkg/browser_node.js` — JS bindings generados por wasm-bindgen
- `pkg/browser_node.d.ts` — TypeScript definitions (opcional)

### 2.2 Compilaci贸n con Validaci贸n

```bash
# Verificar compatibilidad WASM (sin empaquetar)
cargo check --target wasm32-unknown-unknown --features v2.1-wasm-browser

# Verificar librer铆a completa (non-wasm32)
cargo check --lib

# Ejecutar tests
cargo test --lib --features v2.1-wasm-browser
```

### 2.3 Optimizaci贸n de Tama帽o

```bash
# Compilaci贸n con optimizaci贸n de tamaño (release profile)
cargo build --release --target wasm32-unknown-unknown --features v2.1-wasm-browser

# Opcional: Reducir con wasm-opt (binaryen)
wasm-opt -Oz pkg/browser_node_bg.wasm -o pkg/browser_node_bg_optimized.wasm
```

**Tama帽o esperado:** ~150-300 KB (gzip: ~50-100 KB)

---

## 3. Despliegue en Navegador

### 3.1 Estructura de Archivos

```
web/
  browser-node.html      # Interfaz del nodo browser
  browser-node.js        # BrowserNodeManager (Web Worker bridge)
  pkg/
    browser_node_bg.wasm # Binario WASM
    browser_node.js      # JS bindings
```

### 3.2 Servido Est谩tico

```bash
# Opci贸n 1: Node.js (recomendado para desarrollo)
npx serve web -p 8080

# Opci贸n 2: Python
python -m http.server 8080 --directory web

# Opci贸n 3: nginx (producci贸n)
# Ver nginx/browser-node.conf
```

### 3.3 Configuraci贸n nginx (Producci贸n)

```nginx
server {
    listen 80;
    server_name node.ed2k.ai;

    # WASM requires correct MIME type
    location /pkg/ {
        types {
            application/wasm wasm;
        }
        root /var/www/ed2kIA/web;
        expires 7d;
        add_header Cache-Control "public, immutable";
    }

    location / {
        root /var/www/ed2kIA/web;
        try_files $uri $uri/ /browser-node.html;
    }

    # Security headers
    add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; worker-src 'self';";
    add_header X-Content-Type-Options "nosniff";
    add_header Referrer-Policy "strict-origin-when-cross-origin";
}
```

---

## 4. Compatibilidad de Navegadores

| Navegador | Versi贸n M铆nima | WASM | Web Workers | ESM |
|---|---|---|---|---|
| Chrome/Edge | 87+ | âœ… | âœ… | âœ… |
| Firefox | 79+ | âœ… | âœ… | âœ… |
| Safari | 14.1+ | âœ… | âœ… | âœ… |
| Opera | 73+ | âœ… | âœ… | âœ… |
| Brave | 1.30+ | âœ… | âœ… | âœ… |

**Requisitos t茅cnicos:**
- WebAssembly threads (opcional, para fututo paralelismo)
- SharedArrayBuffer (requiere COOP/COEP headers si se usa)
- Service Workers (para offline support, futuro)

### 4.1 Headers COOP/COEP (para SharedArrayBuffer)

```nginx
add_header Cross-Origin-Opener-Policy "same-origin";
add_header Cross-Origin-Embedder-Policy "require-corp";
```

---

## 5. Seguridad y Sandboxing

### 5.1 Capacidades del Nodo WASM

| Capacidad | Estado | Justificaci贸n |
|---|---|---|
| `std::fs` | âŒӔ Bloqueado | WASM no tiene acceso al FS local |
| `std::net` | âŒӔ Bloqueado | WASM no tiene acceso a red directa |
| `Web Workers` | âœ… Permitido | Procesamiento as铆ncrono sin bloquear UI |
| `postMessage` | âœ… Permitido | Comunicaci贸n con main thread |
| `CustomEvent` | âœ… Permitido | Telemetr铆a local (DOM) |
| `localStorage` | âœ… Permitido | Cache de configuraci贸n |
| `IndexedDB` | Futuro | Cache de datasets |

### 5.2 L铆mites de Memoria y CPU

| Recurso | L铆mite | Configuraci贸n |
|---|---|---|
| Memoria m谩xima | 512 MB | `memory_limit_mb` en `BrowserNode::new()` |
| Memoria m铆nima | 16 MB | Clamp interno |
| Cola de tareas m谩xima | 64 tareas | `max_queue_size` |
| Timeout por tarea | 10s | `WORKER_TIMEOUT_MS` en JS |
| Heartbeat interval | 5s | `HEARTBEAT_INTERVAL_MS` en JS |

### 5.3 Content Security Policy (CSP)

```
default-src 'self';
script-src 'self' 'wasm-unsafe-eval';
worker-src 'self';
connect-src 'self';
img-src 'self' data:;
style-src 'self' 'unsafe-inline';
frame-ancestors 'none';
base-uri 'self';
form-action 'self';
```

**Explicaci贸n:**
- `'wasm-unsafe-eval'` — Requiere para WASM din谩mico (wasm-bindgen)
- `worker-src 'self'` — Solo workers del mismo origen
- `frame-ancestors 'none'` — Previene clickjacking

---

## 6. Cl谩usula 茅tica

### 6.1 Principios del Nodo WASM

1. **Cero Telemetr铆a Externa:** El nodo no env铆a datos a servidores externos. Toda telemetr铆a es local (DOM CustomEvents).
2. **Cero Trackers:** No hay cookies de seguimiento, fingerprinting ni identificadores persistentes externos.
3. **Cero L贸gica Financiera:** El nodo no procesa pagos, staking ni recompensas monetarias.
4. **Propiedad Comunitaria:** El c贸digo es open-source (MIT/Apache-2.0). Los voluntarios poseen su nodo.
5. **Hardware Modesto:** Funciona en dispositivos con 2GB RAM y navegadores modernos.
6. **Conexiones Inestables:** Fallback a cola offline cuando no hay conexi贸n.
7. **Fricci贸n Cero:** Apertura del HTML = participaci贸n. Sin registro, sin configuraci贸n compleja.

### 6.2 Transparencia

- **C贸digo fuente:** `src/wasm/browser_node.rs` (Rust/WASM)
- **Bridge JS:** `web/browser-node.js` (Vanilla JS)
- **Interfaz:** `web/browser-node.html` (HTML/CSS/JS)
- **Auditor铆a p煤blica:** Cualquier persona puede auditar el c贸digo WASM antes de ejecutarlo.

---

## 7. Monitoreo y M茅tricas

### 7.1 M茅tricas Locales (Browser)

El nodo expone las siguientes m茅tricas a trav茅s de `BrowserNodeManager.getHealth()`:

```javascript
{
  id: "node-abc123",
  initialized: true,
  queueSize: 3,
  tasksProcessed: 42,
  tasksFailed: 0,
  memoryLimitMb: 64,
  uptimeMs: 120000,
  connected: true,
  offlineQueue: 0
}
```

### 7.2 Eventos del DOM

El nodo emite `CustomEvent` para integraci贸n con el dashboard:

| Evento | Datos |
|---|---|
| `ed2k-node-initialized` | `{ nodeId, memoryLimitMb }` |
| `ed2k-task-complete` | `{ taskId, success, latencyMs }` |
| `ed2k-worker-connected` | `{ workerUrl }` |
| `ed2k-worker-error` | `{ error }` |
| `ed2k-heartbeat` | `{ connected, queueSize }` |

### 7.3 Dashboard Integraci贸n

```html
<script>
  window.addEventListener('ed2k-task-complete', (e) => {
    console.log(`Task ${e.detail.taskId} completed in ${e.detail.latencyMs}ms`);
  });
</script>
```

---

## 8. Flujo de Despliegue

### 8.1 Pipeline de CI/CD

```yaml
# .github/workflows/wasm-deploy.yml
name: WASM Deploy
on:
  push:
    branches: [main]
    paths:
      - "src/wasm/**"
      - "web/**"

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - uses: jetli/wasm-pack-action@v0.4.0
      - run: wasm-pack build --release --target web --features v2.1-wasm-browser
      - uses: actions/upload-artifact@v4
        with:
          name: wasm-pkg
          path: pkg/

  deploy:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          name: wasm-pkg
          path: web/pkg/
      - uses: peaceiris/actions-gh-pages@v4
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./web
```

### 8.2 Verificaci贸n Post-Despliegue

```bash
# Verificar que WASM se sirve con MIME type correcto
curl -I https://node.ed2k.ai/pkg/browser_node_bg.wasm | grep Content-Type
# Expected: application/wasm

# Verificar CSP headers
curl -I https://node.ed2k.ai/browser-node.html | grep Content-Security-Policy

# Test de funcionalidad (headless Chrome)
npx puppeteer-test web/browser-node.html
```

---

## 9. Troubleshooting

### 9.1 Errores Com煤nes

| Error | Causa | Soluci贸n |
|---|---|---|
| `WasmCompileError` | Navegador antiguo | Actualizar a Chrome 87+/Firefox 79+/Safari 14.1+ |
| `Web Workers not supported` | Contexto inseguro | Usar HTTPS o localhost |
| `SharedArrayBuffer not available` | Headers COOP/COEP faltantes | A帽adir headers en servidor |
| `MemoryLimitExceeded` | memory_limit_mb muy bajo | Aumentar a m铆nimo 32MB |
| `QueueFull` | 64 tareas en cola | Esperar a que se procesen o aumentar timeout |
| `Worker timeout` | Tarea >10s | Verificar payload o aumentar `WORKER_TIMEOUT_MS` |

### 9.2 Debug en Navegador

```javascript
// Consola del navegador
const node = new BrowserNodeManager({ id: "debug-node", useWorker: false });
await node.init();
console.log(node.getHealth());

// Ver eventos en tiempo real
window.addEventListener('ed2k-task-complete', console.log);
window.addEventListener('ed2k-worker-error', console.error);
```

### 9.3 Logs de Compilaci贸n

```bash
# Ver warnings de compilaci贸n WASM
RUSTFLAGS="-W warnings" cargo check --target wasm32-unknown-unknown --features v2.1-wasm-browser

# Ver tama帽o del binario WASM
wasm-objdump -h pkg/browser_node_bg.wasm

# Analizar funciones exportadas
wasm-objdump -x pkg/browser_node_bg.wasm | grep -A5 "Export"
```

---

## 10. Referencias

| Recurso | URL |
|---|---|
| wasm-bindgen docs | https://rustwasm.github.io/wasm-bindgen/ |
| Web Workers Spec | https://html.spec.whatwg.org/multipage/workers.html |
| WASM Memory Model | https://webassembly.org/docs/memory/ |
| COOP/COEP Guides | https://web.dev/coop-coep/ |
| ed2kIA README | ../README.md |
| CHANGELOG | ../CHANGELOG.md |
| Browser Node Source | ../src/wasm/browser_node.rs |
| BrowserNodeManager JS | ../web/browser-node.js |

---

## 11. Glosario

| T茅rmino | Definici贸n |
|---|---|
| **BrowserNode** | Nodo WASM compilado que ejecuta tareas SAE en el navegador |
| **BrowserNodeManager** | Clase JS que gestiona el ciclo de vida del BrowserNode con Web Worker bridge |
| **Web Worker Bridge** | Puente postMessage/onmessage entre main thread y Worker |
| **SAE** | Sparse Autoencoder — Modelo de inferencia ligera |
| **SCT** | Stuartian Context Tensor — Tensor 茅tico de evaluaci贸n |
| **Feature Gate** | Compilaci贸n condicional en Rust (#[cfg(feature = "...")]) |
| **wasm-bindgen** | Crate que genera bindings Rust â†” JavaScript |
| **COOP/COEP** | Headers de seguridad para SharedArrayBuffer |

---

*Documento generado para Sprint24 (v2.1.0-sprint24). 煤ltima actualizaci贸n: 2026-05-21.*
