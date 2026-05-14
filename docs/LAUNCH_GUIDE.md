# ed2kIA Launch Guide

> **Guía paso a paso para operadores de seed nodes, validación de red y procedimientos de rollback.**

---

## 📋 Tabla de Contenidos

1. [Requisitos Previos](#-requisitos-previos)
2. [Arquitectura de Lanzamiento](#-arquitectura-de-lanzamiento)
3. [Fase 1: Preparación de Seed Nodes](#-fase-1-preparación-de-seed-nodes)
4. [Fase 2: Inicialización Genesis](#-fase-2-inicialización-genesis)
5. [Fase 3: Validación de Red](#-fase-3-validación-de-red)
6. [Fase 4: Lanzamiento Público](#-fase-4-lanzamiento-público)
7. [Monitoreo y Mantenimiento](#-monitoreo-y-mantenimiento)
8. [Procedimientos de Rollback](#-procedimientos-de-rollback)
9. [Troubleshooting](#-troubleshooting)

---

## 🔧 Requisitos Previos

### Hardware Mínimo (Seed Node)

| Recurso | Mínimo | Recomendado |
|---------|--------|-------------|
| CPU | 4 cores | 8+ cores |
| RAM | 8 GB | 16+ GB |
| Disco | 50 GB SSD | 100+ GB NVMe |
| Red | 100 Mbps | 1 Gbps |
| SO | Ubuntu 22.04 / Debian 12 / Windows 11 | Linux preferred |

### Software

- **Rust 1.75+** (edición 2021)
- **Docker** (opcional, para despliegue en contenedores)
- **Git** (para versionado y actualizaciones)
- **OpenSSL** (para operaciones de certificado)

### Puertos

| Puerto | Protocolo | Descripción |
|--------|-----------|-------------|
| 9000 | TCP/UDP | P2P (libp2p) |
| 3000 | TCP | HTTP/Web UI |
| 9090 | TCP | Prometheus metrics (opcional) |

---

## 🏗️ Arquitectura de Lanzamiento

```
┌─────────────────────────────────────────────────────────────┐
│                    ed2kIA Network Architecture               │
└─────────────────────────────────────────────────────────────┘

                    ┌──────────────────┐
                    │   Seed Node 1    │  ← Bootstrap node (genesis)
                    │  (Genesis Node)  │
                    └────────┬─────────┘
                             │ GossipSub
              ┌──────────────┼──────────────┐
              │              │              │
        ┌─────▼─────┐  ┌────▼─────┐  ┌────▼─────┐
        │ Seed 2    │  │ Seed 3   │  │ Seed N   │
        │ (Validator)│  │(Validator)│  │(Validator)│
        └─────┬─────┘  └────┬─────┘  └────┬─────┘
              │              │              │
              └──────────────┼──────────────┘
                             │
                    ┌────────▼────────┐
                    │  Peer Network   │  ← Community nodes
                    │  (Volunteers)   │
                    └─────────────────┘
```

### Roles de Nodo

| Rol | Descripción | Cantidad Mínima |
|-----|-------------|-----------------|
| **Genesis** | Inicializa la red, crea leases iniciales | 1 |
| **Seed** | Descubrimiento de peers, propagación de mensajes | 3 |
| **Validator** | Participa en consenso, verifica ZKP | 3+ |
| **Peer** | Contribuye con SAE forward, recibe steering | N |

---

## 🚀 Fase 1: Preparación de Seed Nodes

### Paso 1.1: Clonar y Compilar

```bash
# Clonar repositorio
git clone https://github.com/ed2kia/ed2kIA.git
cd ed2kIA

# Verificar versión
git checkout v0.5.0

# Compilar en modo release
cargo build --release

# Verificar binario
./target/release/ed2kia --version
```

### Paso 1.2: Ejecutar Launch Checklist

```bash
chmod +x scripts/launch_checklist.sh
./scripts/launch_checklist.sh
```

Resuelve todos los `[FAIL]` antes de continuar. Los `[WARN]` deben ser revisados pero no bloquean el lanzamiento.

### Paso 1.3: Configurar Variables de Entorno

```bash
# Crear archivo de configuración
cat > .env << 'EOF'
# Node Identity
ED2KIA_NODE_ID=seed-node-1
ED2KIA_IS_BOOTSTRAP=true

# Network
ED2KIA_P2P_PORT=9000
ED2KIA_HTTP_PORT=3000
ED2KIA_BOOTSTRAP_PEERS=""

# Data
ED2KIA_DATA_DIR=./data

# Logging
ED2KIA_LOG_LEVEL=info

# SAE (Optional)
ED2KIA_SAE_PATH=./models/qwen2-7b-sae.safetensors
EOF
```

### Paso 1.4: Configurar Firewall

```bash
# Ubuntu/Debian (ufw)
sudo ufw allow 9000/tcp
sudo ufw allow 9000/udp
sudo ufw allow 3000/tcp

# RHEL/CentOS (firewalld)
sudo firewall-cmd --permanent --add-port=9000/tcp
sudo firewall-cmd --permanent --add-port=9000/udp
sudo firewall-cmd --permanent --add-port=3000/tcp
sudo firewall-cmd --reload

# Windows (PowerShell)
New-NetFirewallRule -DisplayName "ed2kIA P2P" -Direction Inbound -Protocol TCP -LocalPort 9000 -Action Allow
New-NetFirewallRule -DisplayName "ed2kIA P2P UDP" -Direction Inbound -Protocol UDP -LocalPort 9000 -Action Allow
New-NetFirewallRule -DisplayName "ed2kIA HTTP" -Direction Inbound -Protocol TCP -LocalPort 3000 -Action Allow
```

---

## 🌅 Fase 2: Inicialización Genesis

### Paso 2.1: Ejecutar Modo Genesis (Solo Nodo Bootstrap)

```bash
# En el nodo bootstrap (genesis)
./target/release/ed2kia bootstrap genesis \
    --data-dir ./data \
    --p2p-port 9000 \
    --http-port 3000 \
    --is-bootstrap
```

**Salida esperada:**
```
[INFO] Running genesis initialization...
[INFO] Creating initial SAE leases...
[INFO] Distributing layers across genesis node...
[INFO] Activating GossipSub topics...
[INFO] Genesis complete. Network is ready.
[INFO] Node ID: <PEER_ID>
[INFO] Multiaddress: /ip4/0.0.0.0/tcp/9000/p2p/<PEER_ID>
```

### Paso 2.2: Registrar Multiaddress del Genesis

Captura el multiaddress del nodo genesis y compártelo con los operadores de seed nodes:

```bash
# Obtener multiaddress
./target/release/ed2kia status | grep multiaddress
```

### Paso 2.3: Iniciar Seed Nodes Secundarios

En cada seed node adicional:

```bash
# Configurar bootstrap peer
export ED2KIA_BOOTSTRAP_PEERS="/ip4/<GENESIS_IP>/tcp/9000/p2p/<GENESIS_PEER_ID>"

# Unirse a la red
./target/release/ed2kia bootstrap join \
    --data-dir ./data \
    --bootstrap "$ED2KIA_BOOTSTRAP_PEERS"
```

### Paso 2.4: Verificar Conexión entre Seeds

```bash
# En cada seed, verificar peers conectados
curl http://localhost:3000/api/network | jq '.data.total_peers'

# Debe mostrar al menos 1 peer conectado
```

---

## ✅ Fase 3: Validación de Red

### Paso 3.1: Validar Health Checks

```bash
# Verificar health de cada nodo
for PORT in 3000 3001 3002; do
    echo "Node on port $PORT:"
    curl -s http://localhost:$PORT/api/health | jq '.'
done
```

**Respuesta esperada:**
```json
{
    "status": "healthy",
    "checks": {
        "p2p": "ok",
        "sae": "ok",
        "database": "ok"
    }
}
```

### Paso 3.2: Validar Consenso

```bash
# Verificar estado de consenso
curl http://localhost:3000/api/status | jq '.data.node'
```

### Paso 3.3: Validar Gobernanza

```bash
# Verificar que el sistema de gobernanza está activo
./target/release/ed2kia govern list
```

### Paso 3.4: Ejecutar Simulación de Red

```bash
# Ejecutar simulación completa (requiere Docker)
chmod +x scripts/simulate_network.sh
./scripts/simulate_network.sh
```

---

## 🌍 Fase 4: Lanzamiento Público

### Paso 4.1: Publicar Seed Addresses

Crear y publicar `seeds.json`:

```json
[
    {
        "multiaddress": "/ip4/<IP1>/tcp/9000/p2p/<PEER_ID1>",
        "http_addr": "<IP1>:3000",
        "health_url": "http://<IP1>:3000/api/health",
        "operator": "Operator 1",
        "location": "Region"
    },
    {
        "multiaddress": "/ip4/<IP2>/tcp/9000/p2p/<PEER_ID2>",
        "http_addr": "<IP2>:3000",
        "health_url": "http://<IP2>:3000/api/health",
        "operator": "Operator 2",
        "location": "Region"
    }
]
```

### Paso 4.2: Publicar en GitHub Release

```bash
# Preparar release
chmod +x scripts/tag_release.sh
./scripts/tag_release.sh 0.5.0

# Push tag
git push origin v0.5.0

# Crear GitHub Release
gh release create v0.5.0 \
    release/v0.5.0/* \
    --title "v0.5.0 - Bootstrap, Governance, Reputation & Ecosystem" \
    --notes-file release/v0.5.0/RELEASE_NOTES.md
```

### Paso 4.3: Anunciar Lanzamiento

- Publicar en GitHub Discussions
- Notificar a la lista de correo
- Actualizar estado en README.md
- Publicar en canales comunitarios

---

## 📊 Monitoreo y Mantenimiento

### Métricas Clave

| Métrica | Umbral Alerta | Umbral Crítico |
|---------|---------------|----------------|
| Peers conectados | < 3 | < 1 |
| Latencia P2P | > 500ms | > 2000ms |
| SAE forward/s | < 10 | < 1 |
| Consenso approval | < 60% | < 40% |
| Uptime | < 99% | < 95% |
| Memoria usada | > 80% | > 95% |

### Logs

```bash
# Systemd
sudo journalctl -u ed2kia -f

# Docker
docker logs ed2kia-node1 -f

# Directo
./target/release/ed2kia join 2>&1 | tee ed2kia.log
```

---

## 🔄 Procedimientos de Rollback

### Rollback de Modelo SAE

```bash
# Ver versiones disponibles
./target/release/ed2kia sync list

# Rollback a versión anterior
./target/release/ed2kia sync rollback <repo_id>
```

### Rollback de Red (Coordinado)

1. **Notificar** a todos los operadores de seed
2. **Detener** nuevos joins (`--readonly` mode)
3. **Sincronizar** estado de ledger entre seeds
4. **Revertir** a versión anterior:
   ```bash
   git checkout v0.4.0
   cargo build --release
   ```
5. **Reiniciar** nodos en orden: genesis → seeds → peers
6. **Validar** integridad de ledger

### Rollback de Propuesta de Gobernanza

```bash
# Ver estado de propuesta
./target/release/ed2kia govern list

# Si una propuesta fue ejecutada erróneamente:
# 1. Crear propuesta de reversión
./target/release/ed2kia govern propose \
    --type governance \
    --title "Revert Proposal <ID>" \
    --payload "Revert changes from proposal <ID>"

# 2. Votar y ejecutar reversión
```

---

## 🔧 Troubleshooting

### Nodo no se conecta a peers

**Síntoma:** `total_peers: 0`

**Solución:**
```bash
# 1. Verificar firewall
sudo ufw status

# 2. Verificar bootstrap peers
./target/release/ed2kia status

# 3. Forzar reconexión
./target/release/ed2kia exit
./target/release/ed2kia join --bootstrap "/ip4/<SEED_IP>/tcp/9000/p2p/<SEER_ID>"
```

### Health check falla

**Síntoma:** `/api/health` retorna 500

**Solución:**
```bash
# 1. Verificar logs
sudo journalctl -u ed2kia -n 100

# 2. Verificar disco
df -h

# 3. Verificar memoria
free -h

# 4. Reiniciar servicio
sudo systemctl restart ed2kia
```

### Consenso no alcanza quórum

**Síntoma:** Propuestas pendientes sin resolución

**Solución:**
```bash
# 1. Verificar reputación de nodos votantes
./target/release/ed2kia reputation leaderboard

# 2. Verificar cantidad de nodos activos
curl http://localhost:3000/api/network | jq '.data.total_peers'

# 3. Extender período de votación (requiere propuesta)
```

### Ledger corrupto

**Síntoma:** `verify_chain_integrity` retorna false

**Solución:**
```bash
# 1. Backup del ledger actual
cp data/ledger.redb data/ledger.redb.backup

# 2. Intentar reparación
# (La integridad de cadena es inmutable; si está corrupta,
#  se necesita restaurar desde backup o re-sync desde genesis)

# 3. Restaurar desde backup
cp data/ledger.redb.backup data/ledger.redb

# 4. Reiniciar nodo
sudo systemctl restart ed2kia
```

---

## 📞 Soporte

- **Issue Tracker:** [GitHub Issues](https://github.com/ed2kia/ed2kIA/issues)
- **Discusión:** [GitHub Discussions](https://github.com/ed2kia/ed2kIA/discussions)
- **Documentación:** [docs/](../docs/)

---

**ed2kIA** - Descentralizando la interpretabilidad de IA para el beneficio humano.
