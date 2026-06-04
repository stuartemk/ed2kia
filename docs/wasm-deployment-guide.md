# WASM Deployment Guide â€” ed2kIA Browser Node (v2.1.0-sprint24)

> **Objetivo:** Habilitar la participaciè´¸n real de voluntarios desde cualquier navegador moderno, cumpliendo la Ley 4 (Simbiosis Existencial) y Ley 1 (Diversidad Comunitaria) sin centralizaciè´¸n ni lè´¸gica financiera.

---

## 1. Requisitos

### 1.1 Herramientas de Compilaciè´¸n

| Herramienta | Versiè´¸n Mé“†nima | Propè´¸sito |
|---|---|---|
| Rust Toolchain | 1.75.0+ | Compilaciè´¸n base |
| `wasm32-unknown-unknown` target | â€” | Compilaciè´¸n WASM |
| `wasm-pack` | 0.12.0+ | Empaquetado WASM + JS bindings |
| Node.js | 18.0.0+ | Servido estè°©tico + pruebas |
| Python 3 | 3.10+ | Scripts de automatizaciè´¸n (opcional) |

### 1.2 Instalaciè´¸n de Dependencias

```bash
# Agregar target WASM
rustup target add wasm32-unknown-unknown

# Instalar wasm-pack (Linux/macOS)
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Instalar wasm-pack (Windows PowerShell)
winget install wasm-pack  # O usar cargo install wasm-pack
```

### 1.3 Verificaciè´¸n

```bash
rustc --version          # >= 1.75.0
wasm-pack --version      # >= 0.12.0
node --version           # >= 18.0.0
rustup target list | grep wasm32  # wasm32-unknown-unknown (installed)
```

---

## 2. Compilaciè´¸n

### 2.1 Compilaciè´¸n WASM (Browser Node)

```bash
# Compilar con feature v2.1-wasm-browser
wasm-pack build --release \
  --target web \
  --features v2.1-wasm-browser \
  --out-dir pkg
```

**Output:**
- `pkg/browser_node_bg.wasm` â€” Binario WASM optimizado
- `pkg/browser_node.js` â€” JS bindings generados por wasm-bindgen
- `pkg/browser_node.d.ts` â€” TypeScript definitions (opcional)

### 2.2 Compilaciè´¸n con Validaciè´¸n

```bash
# Verificar compatibilidad WASM (sin empaquetar)
cargo check --target wasm32-unknown-unknown --features v2.1-wasm-browser

# Verificar libreré“†a completa (non-wasm32)
cargo check --lib

# Ejecutar tests
cargo test --lib --features v2.1-wasm-browser
```

### 2.3 Optimizaciè´¸n de Tamaå¸½o

```bash
# Compilaciè´¸n con optimizaciè´¸n de tamaÃ±o (release profile)
cargo build --release --target wasm32-unknown-unknown --features v2.1-wasm-browser

# Opcional: Reducir con wasm-opt (binaryen)
wasm-opt -Oz pkg/browser_node_bg.wasm -o pkg/browser_node_bg_optimized.wasm
```

**Tamaå¸½o esperado:** ~150-300 KB (gzip: ~50-100 KB)

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

### 3.2 Servido Estè°©tico

```bash
# Opciè´¸n 1: Node.js (recomendado para desarrollo)
npx serve web -p 8080

# Opciè´¸n 2: Python
python -m http.server 8080 --directory web

# Opciè´¸n 3: nginx (producciè´¸n)
# Ver nginx/browser-node.conf
```

### 3.3 Configuraciè´¸n nginx (Producciè´¸n)

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

| Navegador | Versiè´¸n Mé“†nima | WASM | Web Workers | ESM |
|---|---|---|---|---|
| Chrome/Edge | 87+ | Ã¢Å“â€¦ | Ã¢Å“â€¦ | Ã¢Å“â€¦ |
| Firefox | 79+ | Ã¢Å“â€¦ | Ã¢Å“â€¦ | Ã¢Å“â€¦ |
| Safari | 14.1+ | Ã¢Å“â€¦ | Ã¢Å“â€¦ | Ã¢Å“â€¦ |
| Opera | 73+ | Ã¢Å“â€¦ | Ã¢Å“â€¦ | Ã¢Å“â€¦ |
| Brave | 1.30+ | Ã¢Å“â€¦ | Ã¢Å“â€¦ | Ã¢Å“â€¦ |

**Requisitos tèŒ…cnicos:**
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

| Capacidad | Estado | Justificaciè´¸n |
|---|---|---|
| `std::fs` | Ã¢Å’Ó” Bloqueado | WASM no tiene acceso al FS local |
| `std::net` | Ã¢Å’Ó” Bloqueado | WASM no tiene acceso a red directa |
| `Web Workers` | Ã¢Å“â€¦ Permitido | Procesamiento asé“†ncrono sin bloquear UI |
| `postMessage` | Ã¢Å“â€¦ Permitido | Comunicaciè´¸n con main thread |
| `CustomEvent` | Ã¢Å“â€¦ Permitido | Telemetré“†a local (DOM) |
| `localStorage` | Ã¢Å“â€¦ Permitido | Cache de configuraciè´¸n |
| `IndexedDB` | Futuro | Cache de datasets |

### 5.2 Lé“†mites de Memoria y CPU

| Recurso | Lé“†mite | Configuraciè´¸n |
|---|---|---|
| Memoria mè°©xima | 512 MB | `memory_limit_mb` en `BrowserNode::new()` |
| Memoria mé“†nima | 16 MB | Clamp interno |
| Cola de tareas mè°©xima | 64 tareas | `max_queue_size` |
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

**Explicaciè´¸n:**
- `'wasm-unsafe-eval'` â€” Requiere para WASM dinè°©mico (wasm-bindgen)
- `worker-src 'self'` â€” Solo workers del mismo origen
- `frame-ancestors 'none'` â€” Previene clickjacking

---

## 6. Clè°©usula èŒ…tica

### 6.1 Principios del Nodo WASM

1. **Cero Telemetré“†a Externa:** El nodo no envé“†a datos a servidores externos. Toda telemetré“†a es local (DOM CustomEvents).
2. **Cero Trackers:** No hay cookies de seguimiento, fingerprinting ni identificadores persistentes externos.
3. **Cero Lè´¸gica Financiera:** El nodo no procesa pagos, staking ni recompensas monetarias.
4. **Propiedad Comunitaria:** El cè´¸digo es open-source (MIT/Apache-2.0). Los voluntarios poseen su nodo.
5. **Hardware Modesto:** Funciona en dispositivos con 2GB RAM y navegadores modernos.
6. **Conexiones Inestables:** Fallback a cola offline cuando no hay conexiè´¸n.
7. **Fricciè´¸n Cero:** Apertura del HTML = participaciè´¸n. Sin registro, sin configuraciè´¸n compleja.

### 6.2 Transparencia

- **Cè´¸digo fuente:** `src/wasm/browser_node.rs` (Rust/WASM)
- **Bridge JS:** `web/browser-node.js` (Vanilla JS)
- **Interfaz:** `web/browser-node.html` (HTML/CSS/JS)
- **Auditoré“†a pç…¤blica:** Cualquier persona puede auditar el cè´¸digo WASM antes de ejecutarlo.

---

## 7. Monitoreo y MèŒ…tricas

### 7.1 MèŒ…tricas Locales (Browser)

El nodo expone las siguientes mèŒ…tricas a travèŒ…s de `BrowserNodeManager.getHealth()`:

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

El nodo emite `CustomEvent` para integraciè´¸n con el dashboard:

| Evento | Datos |
|---|---|
| `ed2k-node-initialized` | `{ nodeId, memoryLimitMb }` |
| `ed2k-task-complete` | `{ taskId, success, latencyMs }` |
| `ed2k-worker-connected` | `{ workerUrl }` |
| `ed2k-worker-error` | `{ error }` |
| `ed2k-heartbeat` | `{ connected, queueSize }` |

### 7.3 Dashboard Integraciè´¸n

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

### 8.2 Verificaciè´¸n Post-Despliegue

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

### 9.1 Errores Comç…¤nes

| Error | Causa | Soluciè´¸n |
|---|---|---|
| `WasmCompileError` | Navegador antiguo | Actualizar a Chrome 87+/Firefox 79+/Safari 14.1+ |
| `Web Workers not supported` | Contexto inseguro | Usar HTTPS o localhost |
| `SharedArrayBuffer not available` | Headers COOP/COEP faltantes | Aå¸½adir headers en servidor |
| `MemoryLimitExceeded` | memory_limit_mb muy bajo | Aumentar a mé“†nimo 32MB |
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

### 9.3 Logs de Compilaciè´¸n

```bash
# Ver warnings de compilaciè´¸n WASM
RUSTFLAGS="-W warnings" cargo check --target wasm32-unknown-unknown --features v2.1-wasm-browser

# Ver tamaå¸½o del binario WASM
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

| TèŒ…rmino | Definiciè´¸n |
|---|---|
| **BrowserNode** | Nodo WASM compilado que ejecuta tareas SAE en el navegador |
| **BrowserNodeManager** | Clase JS que gestiona el ciclo de vida del BrowserNode con Web Worker bridge |
| **Web Worker Bridge** | Puente postMessage/onmessage entre main thread y Worker |
| **SAE** | Sparse Autoencoder â€” Modelo de inferencia ligera |
| **SCT** | Topological Context Tensor â€” Tensor èŒ…tico de evaluaciè´¸n |
| **Feature Gate** | Compilaciè´¸n condicional en Rust (#[cfg(feature = "...")]) |
| **wasm-bindgen** | Crate que genera bindings Rust Ã¢â€ â€ JavaScript |
| **COOP/COEP** | Headers de seguridad para SharedArrayBuffer |

---

*Documento generado para Sprint24 (v2.1.0-sprint24). ç…¤ltima actualizaciè´¸n: 2026-05-21.*
