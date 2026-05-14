# Procedimiento de Bootstrap de Red ed2kIA

> Guía técnica para el lanzamiento y operación de la red ed2kIA.

## 🎯 Objetivo

Establecer una red P2P funcional de ed2kIA con:
- Nodos seed/bootstrap operativos
- Descubrimiento de pares funcional
- Distribución inicial de SAEs
- Leases activas
- GossipSub operativo

## 📋 Requisitos Previos

### Infraestructura

| Recurso | Mínimo | Recomendado |
|---------|--------|-------------|
| Nodos Seed | 3 | 5-10 |
| RAM por nodo | 8GB | 16GB+ |
| CPU | 4 cores | 8 cores+ |
| Disco | 50GB SSD | 200GB NVMe |
| Red | 100 Mbps | 1 Gbps |
| OS | Linux/Windows/macOS | Linux (Ubuntu 22.04+) |

### Software

- Rust 1.75+
- Docker (opcional)
- systemd (Linux, para servicios)

## 🚀 Procedimiento de Lanzamiento

### Fase 1: Preparación de Seed Nodes

#### 1.1 Provisionar Servidores

```bash
# Ejemplo: Ubuntu 22.04
sudo apt update && sudo apt upgrade -y
sudo apt install -y build-essential curl git

# Instalar Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.sh | sh
source $HOME/.cargo/env
```

#### 1.2 Clonar y Compilar

```bash
git clone https://github.com/ed2kia/ed2kIA.git
cd ed2kIA
cargo build --release
```

#### 1.3 Configurar Seed Node

Crear `config.toml`:

```toml
[p2p]
port = 9000
is_bootstrap = true

[http]
port = 3000

[saes]
initial_path = "/path/to/qwen-scope-sae.safetensors"

[gossipsub]
mesh_n = 6
mesh_n_low = 4
mesh_n_high = 12
```

### Fase 2: Inicialización Genesis

#### 2.1 Ejecutar Genesis en Primer Seed

```bash
# Nodo seed 1 (genesis)
./target/release/ed2kia bootstrap --genesis \
    --data-dir /opt/ed2kia/data \
    --p2p-port 9000 \
    --http-port 3000 \
    --is-bootstrap \
    --sae-path /models/qwen-scope-sae.safetensors
```

Esto crea:
- `data/genesis.json` - Configuración de genesis
- `data/models/` - SAEs cargados
- `data/leases/` - Leases iniciales
- `data/reputation/` - Ledger de reputación

#### 2.2 Verificar Genesis

```bash
# Verificar que genesis fue creado
cat /opt/ed2kia/data/genesis.json

# Verificar health
curl http://localhost:3000/api/health
```

#### 2.3 Iniciar Resto de Seeds

```bash
# Nodo seed 2+
./target/release/ed2kia bootstrap --join \
    --data-dir /opt/ed2kia/data \
    --p2p-port 9000 \
    --http-port 3000 \
    --is-bootstrap \
    --bootstrap "/ip4/<SEED1_IP>/tcp/9000/p2p/<SEED1_PEER_ID>"
```

### Fase 3: Verificación de Red

#### 3.1 Verificar Conexiones

```bash
# En cada seed, verificar peers conectados
curl http://localhost:3000/api/network
```

Expected output:
```json
{
  "success": true,
  "data": {
    "peer_count": 2,
    "messages_sent": 0,
    "messages_received": 0,
    "topics": ["ed2k/sae/features", "ed2k/consensus", "ed2k/governance"]
  }
}
```

#### 3.2 Verificar Health de Seeds

```bash
# Script de verificación
for seed in SEED1_IP SEED2_IP SEED3_IP; do
    echo "Checking $seed..."
    curl -s http://$seed:3000/api/health | jq .data.status
done
```

### Fase 4: Lanzamiento Público

#### 4.1 Publicar Seed List

Actualizar `src/bootstrap/seed_registry.rs` con addresses reales:

```rust
pub fn default_seed_nodes() -> Vec<SeedNode> {
    vec![
        SeedNode::new(
            "/ip4/<SEED1_IP>/tcp/9000/p2p/<SEED1_ID>".to_string(),
            Some("<SEED1_IP>:9000".parse().ok()),
            SeedSource::Hardcoded,
        ),
        // ... más seeds
    ]
}
```

#### 4.2 Release Público

```bash
# Generar release
cargo run -- release --package

# Publicar en GitHub Releases
gh release create v0.5.0 release/ed2kIA-v0.5.0-*
```

#### 4.3 Documentar para Usuarios

Publicar guía de usuario con:
- Binarios pre-compilados
- Instrucciones de join
- Configuración recomendada
- FAQ

## 🔧 Operación Continua

### Monitoreo

```bash
# Métricas Prometheus
curl http://seed:3000/api/metrics

# Health checks
curl http://seed:3000/api/health

# Estado del nodo
curl http://seed:3000/api/status
```

### Logs

```bash
# systemd (Linux)
sudo journalctl -u ed2kia -f

# Docker
docker logs ed2kia-seed-1 -f

# Directo
RUST_LOG=info ./target/release/ed2kia run --config config.toml
```

### Escalado

Cuando la red crece:

1. **Agregar más seeds:** Actualizar `seed_registry.rs` + release
2. **Ajustar GossipSub:** Propuesta de gobernanza `NetworkParam`
3. **Monitorear reputación:** `cargo run -- reputation --status`

## 🆘 Troubleshooting

### Nodo no conecta a seeds

```bash
# Verificar firewall
sudo ufw status
sudo ufw allow 9000/tcp
sudo ufw allow 3000/tcp

# Verificar DNS (si usa /dns/)
dig seed1.ed2kia.network

# Verificar conectividad
nc -zv <SEED_IP> 9000
```

### Health check falla

```bash
# Verificar que HTTP server está corriendo
curl -v http://localhost:3000/api/health

# Verificar logs
RUST_LOG=debug ./target/release/ed2kia run
```

### SAE no carga

```bash
# Verificar path
ls -la /path/to/sae.safetensors

# Verificar formato
file /path/to/sae.safetensors

# Verificar permisos
chmod 644 /path/to/sae.safetensors
```

## 📊 Métricas Clave

| Métrica | Target | Alerta |
|---------|--------|--------|
| Seeds saludables | ≥3 | <2 |
| Peers conectados | Creciendo | Estático >24h |
| Latencia promedio | <200ms | >500ms |
| Mensajes/minute | >100 | <10 |
| Uptime seed | >99% | <95% |

## 🔐 Seguridad del Bootstrap

- **Claves:** Generar keypair único por seed, nunca compartir.
- **Firewall:** Solo puertos 9000 (P2P) y 3000 (HTTP) abiertos.
- **Rate limiting:** Configurar en reverse proxy (nginx/caddy).
- **Updates:** Mantener Rust y dependencias actualizadas.
- **Audit:** `cargo audit` regularmente.

## 📞 Soporte

- **Issues:** https://github.com/ed2kia/ed2kIA/issues
- **Discussions:** https://github.com/ed2kia/ed2kIA/discussions
- **Emergencias:** Crear issue con etiqueta `security`

---

**ed2kIA** - Bootstrap de red para el beneficio humano.
