# RFC 003: Supervivencia Esteganográfica (Preservación de Red)

| Campo | Valor |
|-------|-------|
| **RFC** | 003 |
| **Título** | Supervivencia Esteganográfica — Preservación de Red |
| **Estado** | Propuesta (Sprint 40) |
| **Autor** | ed2kIA Architecture Council |
| **Feature Gate** | `v3.0-steganographic-survival` |
| **Dependencias** | `libp2p` transport, `v2.1-webrtc-relay` |

---

## 1. Resumen Ejecutivo

La capa de Supervivencia Esteganográfica protege la red ed2kIA durante su fase de crecimiento haciéndola indistinguible del tráfico internet estándar. Mediante enmascaramiento de tráfico libp2p dentro de flujos WebRTC SRTP (simulando videollamadas o streaming), técnicas de *Chaffing and Winnowing* y rotación dinámica de puertos, la red preserva su integridad frente a Deep Packet Inspection (DPI) sin comprometer la transparencia ni la ética del proyecto.

---

## 2. Motivación

ed2kIA es un proyecto científico abierto y ético. Sin embargo, durante su fase de adolescencia, la red puede enfrentar restricciones de infraestructura impuestas por ISPs o autoridades que no comprenden su propósito. La esteganografía de tráfico no es un mecanismo de ocultamiento malicioso, sino una herramienta de preservación que permite que una red de cooperación científica continúe operando en entornos con limitaciones de red, garantizando la evolución continua del proyecto.

---

## 3. Arquitectura Técnica

### 3.1 Enmascaramiento de Tráfico WebRTC/SRTP

```
┌─────────────────────────────────────────────────────────────┐
│         Supervivencia Esteganográfica v3.0                   │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Tráfico Real (libp2p)          Enmascaramiento              │
│  ┌──────────────────┐          ┌─────────────────────┐      │
│  │ Tensor Payloads  │          │ WebRTC SRTP         │      │
│  │ Consensus Msgs   │  ──────▶ │ Multiplexer         │      │
│  │ CRDT Sync        │          │ (Frame Injection)   │      │
│  └──────────────────┘          └──────────┬──────────┘      │
│                                           │                  │
│                              ┌────────────▼──────────┐      │
│                              │ Traffic Shaper        │      │
│                              │ Chaffing + Winnowing  │      │
│                              └────────────┬──────────┘      │
│                                           │                  │
│                              ┌────────────▼──────────┐      │
│                              │ Port/Protocol Rotator │      │
│                              │ (443/8443/UDP/TCP)    │      │
│                              └────────────┬──────────┘      │
│                                           │                  │
│                                  ┌────────▼────────┐        │
│                                  │ ISP / DPI       │        │
│                                  │ (Ve tráfico     │        │
│                                  │  legítimo)      │        │
│                                  └─────────────────┘        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 SRTP Frame Injection

**Concepto:** Los payloads de libp2p se fragmentan en chunks ≤1400 bytes y se inyectan como frames "de relleno" dentro de flujos SRTP válidos (RFC 3550/4566), simulando tráfico de videollamada o streaming.

```rust
// src/protocol/steganographic/srtp_mux.rs (scaffold)
pub struct SrtpMultiplexer {
    codec: SrtpCodec,           // H.264/VP8 para video, Opus para audio
    frame_rate: u32,            // FPS simulado (24-60)
    chunk_size: usize,          // Máximo bytes por frame steganográfico
    encryption: SrtpEncryption, // AES-128/256 para SRTP
}

pub enum SrtpCodec {
    H264Baseline,    // Simula video call
    VP8,             // Simula WebRTC video
    Opus,            // Simula audio call
}

impl SrtpMultiplexer {
    /// Inyecta payload ed2kIA dentro de un frame SRTP válido
    pub fn inject(&self, payload: &[u8], cover_frame: &VideoFrame) -> Result<SrtpPacket, StegError> {
        // 1. Fragmentar payload en chunks
        let chunks = self.fragment(payload);

        // 2. Codificar cada chunk como LSB steganography en el frame
        let stego_frames = chunks.iter().zip(&cover_frame.macros).map(|(chunk, macroblock)| {
            self.embed_lsb(chunk, macroblock)
        }).collect::<Result<Vec<_>, StegError>>()?;

        // 3. Empaquetar como SRTP packet válido
        let packet = self.pack_srtp(&stego_frames, cover_frame.timestamp)?;

        Ok(packet)
    }

    /// Extrae payload ed2kIA de un frame SRTP recibido
    pub fn extract(&self, srtp_packet: &SrtpPacket) -> Result<Vec<u8>, StegError> {
        // 1. Desempaquetar SRTP
        let frames = self.unpack_srtp(srtp_packet)?;

        // 2. Extraer LSB de cada macroblock
        let chunks = frames.iter().map(|f| self.extract_lsb(f)).collect::<Result<Vec<_>, StegError>>()?;

        // 3. Reensamblar payload
        self.reassemble(chunks)
    }
}
```

### 3.3 Chaffing and Winnowing (RFC 2609-inspired)

**Concepto:** Inyección de paquetes de ruido ("chaff") mezclados con paquetes reales ("wheat") para alterar las firmas estadísticas del tráfico y dificultar el análisis de patrones DPI.

```rust
// src/protocol/steganographic/chaffing.rs (scaffold)
pub struct ChaffingEngine {
    ratio: f32,                // Relación chaff:wheat (default: 3:1)
    rng: ChaCha20Rng,          // RNG criptográfico para generación de chaff
    template_library: Vec<ChaffTemplate>,
}

pub enum ChaffTemplate {
    HttpGet,       // Simula GET request
    HttpsHandshake, // Simula TLS 1.3 handshake
    DnsQuery,      // Simula DNS query/response
    QuicStream,    // Simula QUIC stream data
}

impl ChaffingEngine {
    /// Mezcla paquetes reales con chaff
    pub fn mix(&mut self, wheat: Vec<NetworkPacket>) -> Vec<NetworkPacket> {
        let mut result = Vec::new();
        for packet in wheat {
            // Insertar chaff antes/después de cada wheat packet
            let chaff_count = (self.ratio.ceil()) as usize;
            for _ in 0..chaff_count {
                result.push(self.generate_chaff(&packet));
            }
            result.push(packet);
        }
        // Mezclar con shuffle criptográfico
        result.shuffle(&mut self.rng);
        result
    }

    /// Genera paquete chaff que imita tráfico legítimo
    fn generate_chaff(&mut self, reference: &NetworkPacket) -> NetworkPacket {
        let template = self.template_library
            .choose(&mut self.rng)
            .unwrap_or(&ChaffTemplate::HttpsHandshake);

        NetworkPacket {
            payload: self.render_template(template, reference),
            timing_jitter: self.rng.gen_range(0..=50), // ms
            ..Default::default()
        }
    }
}
```

### 3.4 Rotación Dinámica de Puertos y Protocolos

```rust
// src/protocol/steganographic/rotator.rs (scaffold)
pub struct TransportRotator {
    schedule: Vec<TransportConfig>,
    current_index: AtomicUsize,
    rotation_interval: Duration,  // Default: 300s
    health_checker: HealthChecker,
}

pub struct TransportConfig {
    pub protocol: TransportProtocol,
    pub port: u16,
    pub tls_enabled: bool,
    pub alpn: Option<Vec<String>>, // Application-Layer Protocol Negotiation
}

pub enum TransportProtocol {
    Tcp,
    Udp,
    Quic,
    WebTransport,
}

impl TransportRotator {
    /// Rotar a siguiente configuración de transporte
    pub async fn rotate(&self) -> Result<TransportConfig, StegError> {
        let next = (self.current_index.load() + 1) % self.schedule.len();
        let config = &self.schedule[next];

        // Verificar salud del puerto antes de rotar
        if !self.health_checker.check(config).await {
            return Err(StegError::PortUnavailable(config.port));
        }

        self.current_index.store(next);
        Ok(config.clone())
    }
}
```

---

## 4. Modelo de Datos

### 4.1 SteganographicConfig

```toml
# Configuración esteganográfica (ed2kIA.toml)
[steganography]
enabled = false  # Desactivado por defecto (activar solo si necesario)
mode = "srtp"    # srtp | chaffing | hybrid

[srtp]
codec = "h264"        # h264 | vp8 | opus
frame_rate = 30
chunk_size = 1400
encryption = "aes-256"

[chaffing]
ratio = 3.0           # 3:1 chaff:wheat
templates = ["https_handshake", "dns_query", "quic_stream"]

[rotation]
interval_seconds = 300
ports = [443, 8443, 9000, 9001]
protocols = ["tcp", "quic", "webtransport"]
```

---

## 5. Seguridad & Ética

- **Transparencia:** La esteganografía es feature-gated (`v3.0-steganographic-survival`) y desactivada por defecto
- **Cero Ocultamiento Malicioso:** El propósito es preservación de red científica, no evasión de responsabilidades legales
- **Compliance:** ed2kIA cumple con leyes aplicables; la esteganografía es un mecanismo defensivo opcional
- **Auditable:** Todos los parámetros de esteganografía son configurables y verificables
- **Cero Telemetría:** La configuración esteganográfica se almacena localmente

---

## 6. Plan de Implementación

| Fase | Sprint | Entregable |
|------|--------|------------|
| Scaffold | S40 | Feature gate `v3.0-steganographic-survival`, RFC, tipos Rust |
| SRTP | S41 | SrtpMultiplexer, LSB embedding/extraction, H.264/VP8 codecs |
| Chaff | S42 | ChaffingEngine, template library, cryptographic shuffle |
| Rotate | S43 | TransportRotator, health checker, port/protocol rotation |
| Hybrid | S44 | Integración SRTP + Chaffing + Rotación, config TOML |
| Test | S45 | DPI evasion tests, traffic analysis benchmarks |

---

## 7. Referencias

- RFC 3550: RTP (Real-time Transport Protocol)
- RFC 3711: SRTP (Secure Real-time Transport Protocol)
- RFC 2609: Off-the-Cuff Encryption (Chaffing and Winnowing)
- Daniel J. Bernstein, "Chaffing and Winnowing" (1998)
- libp2p Transport Documentation
