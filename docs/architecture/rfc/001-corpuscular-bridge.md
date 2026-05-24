# RFC 001: El Puente Corpuscular (IoT Simbiótico & Economía CE)

| Campo | Valor |
|-------|-------|
| **RFC** | 001 |
| **Título** | El Puente Corpuscular — IoT Simbiótico & Economía CE |
| **Estado** | Propuesta (Sprint 40) |
| **Autor** | ed2kIA Architecture Council |
| **Feature Gate** | `v3.0-corpuscular-bridge` |
| **Dependencias** | `v2.1-proof-of-symbiosis` (CE Ledger), `libp2p` streams |

---

## 1. Resumen Ejecutivo

El Puente Corpuscular conecta la red de información de ed2kIA con el nivel de energía físico, permitiendo que el poder de cómputo distribuido se intercambie por recursos físicos locales (producción 3D, energía solar, agricultura hidropónica) mediante el Crédito de Existencia (CE). Este puente materializa la simbiosis entre lo digital y lo físico, equilibrando la contribución computacional con la producción tangible.

---

## 2. Motivación

ed2kIA ha consolidado una red P2P de interpretabilidad de IA. El siguiente paso evolutivo es integrar esta red con el mundo físico, permitiendo que los nodos que aportan poder de cómputo puedan acceder a recursos físicos de producción local. Esto crea un ecosistema de cooperación simbiótica donde la contribución digital se traduce en bienestar material, sin lógica financiera especulativa.

---

## 3. Arquitectura Técnica

### 3.1 Protocolo de Transporte IoT sobre libp2p

```
┌─────────────────────────────────────────────────────────┐
│              Puente Corpuscular v3.0                     │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌──────────────┐    ┌──────────────┐    ┌───────────┐ │
│  │ MQTT/CoAP    │    │ libp2p       │    │ CE        │ │
│  │ Adapter      │◄──▶│ Stream Relay │◄──▶│ Ledger    │ │
│  │ (IoT HW)     │    │ (P2P Mesh)   │    │ (Ed25519) │ │
│  └──────────────┘    └──────────────┘    └───────────┘ │
│        │                  │                  │           │
│        ▼                  ▼                  ▼           │
│  ┌──────────────────────────────────────────────────┐  │
│  │         Corpuscular Contract Engine              │  │
│  │  CE → Physical Resource Exchange (Zero Fiat)     │  │
│  └──────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

**Protocolos soportados:**
- **MQTT 3.1.1/5.0** — Mensajería ligera para sensores/actuadores (QoS 0-2)
- **CoAP** — Protocolo RESTful para dispositivos IoT de baja potencia (RFC 7252)
- **WebTransport** — Transporte bidireccional sobre HTTP/3 para nodos WASM

**Integración con libp2p:**
- Uso de `libp2p-stream` para túneles encriptados entre nodos IoT y la malla P2P
- Topic-based routing: `/corpuscular/{region}/{device_type}/{action}`
- Payload binario: Protobuf con schema `CorpuscularMessage`

### 3.2 API Local de Hardware (Rust)

```rust
// src/protocol/corpuscular/bridge.rs (scaffold)
pub trait HardwareAdapter {
    fn device_type(&self) -> DeviceType;
    fn connect(&mut self, endpoint: &str) -> Result<(), CorpuscularError>;
    fn execute(&self, command: HardwareCommand) -> Result<HardwareResponse, CorpuscularError>;
    fn status(&self) -> Result<DeviceStatus, CorpuscularError>;
}

pub enum DeviceType {
    Printer3D,
    SolarMicrogrid,
    HydroponicController,
    Custom(String),
}

#[derive(Serialize, Deserialize)]
pub struct HardwareCommand {
    pub device_id: String,
    pub action: String,
    pub parameters: HashMap<String, Value>,
    pub ce_cost: f64,          // CE requerido para esta operación
    pub signed_by: ed25519::Signature,
}
```

### 3.3 Contratos de Circuito Cerrado (CE-Based)

Los contratos corpusculares son acuerdos de intercambio CE ↔ Recurso Físico firmados criptográficamente:

```
┌────────────────────────────────────────────────────┐
│         Corpuscular Contract Lifecycle              │
├────────────────────────────────────────────────────┤
│                                                     │
│  1. Propuesta: Nodo A solicita recurso físico       │
│     → { device_id, action, ce_cost, timestamp }     │
│                                                     │
│  2. Firma Ed25519: Ambas partes firman la propuesta │
│     → signature_A = sign(private_A, hash(proposal)) │
│     → signature_B = sign(private_B, hash(proposal)) │
│                                                     │
│  3. Validación CE: Verificar saldo del solicitante  │
│     → CE_ledger.balance(node_A) >= ce_cost          │
│                                                     │
│  4. Ejecución: HardwareAdapter.execute(command)     │
│     → Éxito: CE transferido (emit → burn)           │
│     → Fallo: CE refundado automáticamente           │
│                                                     │
│  5. Auditoría P2P: Evento replicado vía GossipSub   │
│     → /corpuscular/audit/{contract_id}              │
│                                                     │
└────────────────────────────────────────────────────┘
```

**Invariantes del contrato:**
- Cero fiat: Todos los intercambios usan exclusivamente CE
- Atomicidad: Si el hardware falla, el CE se refunda automáticamente
- Inmutabilidad: Los contratos firmados no pueden modificarse
- Transparencia: Todos los eventos se replican en la malla P2P

---

## 4. Modelo de Datos

### 4.1 CorpuscularMessage (Protobuf Schema)

```protobuf
syntax = "proto3";

message CorpuscularMessage {
  oneof payload {
    HardwareProposal proposal = 1;
    HardwareResponse response = 2;
    ContractExecution execution = 3;
    AuditEvent audit = 4;
  }
}

message HardwareProposal {
  string contract_id = 1;
  string requester_id = 2;
  string device_id = 3;
  string action = 4;
  map<string, string> parameters = 5;
  double ce_cost = 6;
  uint64 timestamp = 7;
  bytes signature = 8;
}

message ContractExecution {
  string contract_id = 1;
  ExecutionStatus status = 2;
  double ce_transferred = 3;
  string result_data = 4;
  uint64 completed_at = 5;
}

enum ExecutionStatus {
  PENDING = 0;
  IN_PROGRESS = 1;
  COMPLETED = 2;
  FAILED_REFUNDED = 3;
}
```

---

## 5. Seguridad & Privacidad

- **Encriptación E2E:** Todos los mensajes corpusculares usan `libp2p` noise protocol (XX/PSK)
- **Firma Ed25519:** Cada propuesta y respuesta requiere firma criptográfica
- **Validación CE:** El CE Ledger verifica saldos antes de ejecutar cualquier contrato
- **Rate Limiting:** Máximo 10 contratos/hora por nodo para prevenir abuso
- **Cero Telemetría:** Los datos de hardware permanecen en el nodo local; solo los eventos de auditoría se replican

---

## 6. Plan de Implementación

| Fase | Sprint | Entregable |
|------|--------|------------|
| Scaffold | S40 | Feature gate `v3.0-corpuscular-bridge`, RFC, tipos Rust |
| Core | S41 | HardwareAdapter trait, CorpuscularMessage protobuf, CE validation |
| IoT | S42 | MQTT/CoAP adapter, libp2p stream relay |
| Contract | S43 | Contract engine, Ed25519 signing, atomic execution |
| Audit | S44 | GossipSub replication, audit trail, dashboard |

---

## 7. Referencias

- RFC 7252: Constrained Application Protocol (CoAP)
- MQTT v5.0 Specification (OASIS Standard)
- libp2p Stream Protocol Documentation
- ed2kIA CE Ledger: `src/economics/existential_credit.rs`
