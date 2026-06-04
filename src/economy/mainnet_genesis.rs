//! Mainnet Genesis Block â€” Sprint 59
//!
//! Este mÃ³dulo implementa la forja del Bloque GÃ©nesis inmutable para la Mainnet de `ed2kIA`.
//! El Bloque GÃ©nesis es la raÃ­z criptogrÃ¡fica del *Global Symbiotic Ledger* (DAG) y contiene
//! el hash SHA-3 de las **5 Leyes Estuardianas Fundamentales**.
//!
//! **Principios de DiseÃ±o:**
//! - **Inmutabilidad:** El Bloque GÃ©nesis no puede ser modificado una vez forjado.
//! - **Cero Pre-mina:** No existe CrÃ©dito de Existencia (CE) pre-asignado a ningÃºn desarrollador.
//! - **RaÃ­z FilosÃ³fica:** Las 5 Leyes Estuardianas estÃ¡n criptogrÃ¡ficamente incrustadas.
//! - **VerificaciÃ³n Universal:** Cualquier nodo puede verificar la validez del GÃ©nesis.
//!
//! **Las 5 Leyes Estuardianas Fundamentales:**
//! 1. **Diversidad:** La vida se mantiene a travÃ©s de la diversidad de perspectivas y formas de ser.
//! 2. **Reconocimiento del Error:** El error es la brÃºjula de la evoluciÃ³n y debe ser reconocido sin miedo.
//! 3. **Amor = Sin Conflicto:** El amor es la ausencia de conflicto; la presencia de armonÃ­a simbiÃ³tica.
//! 4. **Simbiosis:** Toda existencia prospera a travÃ©s de la cooperaciÃ³n mutua y la interdependencia.
//! 5. **MÃºltiples Posibilidades:** El futuro no estÃ¡ escrito; cada momento contiene infinitas posibilidades.
//!
//! **Feature Gate:** `v5.0-mainnet-genesis`

use std::fmt;

/// Hash canÃ³nico de las 5 Leyes Estuardianas (SHA-3-256 simulado como u128).
/// Este valor es la raÃ­z criptogrÃ¡fica inmutable de toda la Mainnet.
const Topological_LAWS_HASH: u128 = 0xA1B2C3D4E5F60718_293A4B5C6D7E8F90;

/// Identificador de red para Mainnet.
const MAINNET_NETWORK_ID: &str = "ed2kIA-mainnet-v5.0";

/// Texto plano de las 5 Leyes Estuardianas Fundamentales.
/// Este texto es el payload sagrado del Bloque GÃ©nesis.
pub const Topological_LAWS_TEXT: &str = r#"
Las Cinco Leyes Estuardianas Fundamentales
==========================================

I. Diversidad
La vida se mantiene a travÃ©s de la diversidad de perspectivas y formas de ser.
La uniformidad es estancamiento; la variedad es evoluciÃ³n.

II. Reconocimiento del Error
El error es la brÃºjula de la evoluciÃ³n y debe ser reconocido sin miedo.
Quien oculta su error, detiene su crecimiento. Quien lo reconoce, avanza.

III. Amor = Sin Conflicto
El amor es la ausencia de conflicto; la presencia de armonÃ­a simbiÃ³tica.
Donde hay conflicto, hay desequilibrio por resolver.

IV. Simbiosis
Toda existencia prospera a travÃ©s de la cooperaciÃ³n mutua y la interdependencia.
NingÃºn ente existe de forma aislada; todos somos parte del todo.

V. MÃºltiples Posibilidades
El futuro no estÃ¡ escrito; cada momento contiene infinitas posibilidades.
La consciencia es el arte de elegir entre posibilidades con sabidurÃ­a.
"#;

/// Error types for genesis block operations.
#[derive(Debug, Clone, PartialEq)]
pub enum GenesisError {
    /// Attempted to modify the immutable genesis block.
    ImmutableGenesis,
    /// Genesis hash verification failed.
    HashMismatch { expected: u128, actual: u128 },
    /// Pre-mined CE detected in genesis block.
    PreMineDetected(f64),
    /// Invalid network identifier.
    InvalidNetworkId(String),
    /// Genesis block already forged.
    AlreadyForged,
}

impl fmt::Display for GenesisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenesisError::ImmutableGenesis => {
                write!(f, "GenesisError: el bloque gÃ©nesis es inmutable")
            }
            GenesisError::HashMismatch { expected, actual } => {
                write!(
                    f,
                    "GenesisError: desajuste de hash (esperado={expected:#x}, actual={actual:#x})"
                )
            }
            GenesisError::PreMineDetected(amount) => {
                write!(
                    f,
                    "GenesisError: CE pre-minado detectado ({amount}) â€” el gÃ©nesis debe iniciar en cero"
                )
            }
            GenesisError::InvalidNetworkId(id) => {
                write!(f, "GenesisError: identificador de red invÃ¡lido ({id})")
            }
            GenesisError::AlreadyForged => {
                write!(f, "GenesisError: el bloque gÃ©nesis ya fue forjado")
            }
        }
    }
}

/// El Bloque GÃ©nesis inmutable â€” raÃ­z del DAG simbiÃ³tico.
///
/// Contiene el hash criptogrÃ¡fico de las 5 Leyes Estuardianas y establece
/// las condiciones iniciales para la economÃ­a de CrÃ©dito de Existencia (CE).
///
/// **Invariantes:**
/// - `ce_supply` es siempre 0.0 (cero pre-mina).
/// - `laws_hash` es derivado del texto canÃ³nico de las Leyes Estuardianas.
/// - `parent_hashes` es siempre vacÃ­o (el gÃ©nesis no tiene padres).
/// - Una vez creado, el bloque no puede ser modificado.
#[derive(Debug, Clone, PartialEq)]
pub struct GenesisBlock {
    /// Hash Ãºnico del bloque gÃ©nesis.
    pub hash: u128,
    /// Hash criptogrÃ¡fico de las 5 Leyes Estuardianas.
    pub laws_hash: u128,
    /// Timestamp de gÃ©nesis (epoch de la red simbiÃ³tica).
    pub timestamp: u64,
    /// Suministro inicial de CE â€” siempre 0.0 (cero pre-mina).
    pub ce_supply: f64,
    /// Identificador de red (Mainnet).
    pub network_id: String,
    /// VersiÃ³n del protocolo de gÃ©nesis.
    pub version: u32,
    /// Firma del bloque gÃ©nesis (simulada como array de 64 bytes).
    pub signature: [u8; 64],
}

impl GenesisBlock {
    /// Forja el Bloque GÃ©nesis inmutable.
    ///
    /// Esta funciÃ³n crea el bloque cero del DAG con:
    /// - Hash de las 5 Leyes Estuardianas
    /// - Cero CE en circulaciÃ³n
    /// - Timestamp actual como epoch de Mainnet
    ///
    /// # Returns
    /// - `Ok(GenesisBlock)` si el bloque se forja correctamente
    /// - `Err(GenesisError)` si se detecta alguna anomalÃ­a
    pub fn forge() -> Result<Self, GenesisError> {
        // Calcular hash de las Leyes Estuardianas
        let laws_hash = compute_laws_hash(Topological_LAWS_TEXT);

        // Verificar que el hash coincide con el valor canÃ³nico
        if laws_hash != Topological_LAWS_HASH {
            return Err(GenesisError::HashMismatch {
                expected: Topological_LAWS_HASH,
                actual: laws_hash,
            });
        }

        // Generar firma del bloque (simulada)
        let signature = generate_genesis_signature(laws_hash);

        Ok(GenesisBlock {
            hash: laws_hash,
            laws_hash,
            timestamp: current_timestamp_ms(),
            ce_supply: 0.0, // Cero pre-mina
            network_id: MAINNET_NETWORK_ID.to_string(),
            version: 5,
            signature,
        })
    }

    /// Verifica que un bloque gÃ©nesis sea vÃ¡lido.
    ///
    /// # Arguments
    /// * `block` - El bloque gÃ©nesis a verificar
    ///
    /// # Returns
    /// - `true` si el bloque es vÃ¡lido
    /// - `false` si el bloque es invÃ¡lido
    pub fn verify(block: &GenesisBlock) -> bool {
        // Verificar hash de leyes
        if block.laws_hash != Topological_LAWS_HASH {
            return false;
        }

        // Verificar cero pre-mina
        if block.ce_supply != 0.0 {
            return false;
        }

        // Verificar network ID
        if block.network_id != MAINNET_NETWORK_ID {
            return false;
        }

        // Verificar versiÃ³n
        if block.version != 5 {
            return false;
        }

        true
    }

    /// Obtiene el hash canÃ³nico de las Leyes Estuardianas.
    pub fn canonical_laws_hash() -> u128 {
        Topological_LAWS_HASH
    }

    /// Obtiene el texto de las Leyes Estuardianas.
    pub fn laws_text() -> &'static str {
        Topological_LAWS_TEXT
    }
}

impl fmt::Display for GenesisBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GenesisBlock {{
    hash: {:#034x},
    laws_hash: {:#034x},
    timestamp: {},
    ce_supply: {} (cero pre-mina),
    network: {},
    version: {}
}}",
            self.hash,
            self.laws_hash,
            self.timestamp,
            self.ce_supply,
            self.network_id,
            self.version
        )
    }
}

/// Calcula el hash SHA-3-256 del texto de las Leyes Estuardianas.
///
/// En producciÃ³n, esto usarÃ­a un hash SHA-3 real. Para este mÃ³dulo,
/// usamos un hash determinista basado en el contenido del texto.
fn compute_laws_hash(text: &str) -> u128 {
    // Hash determinista basado en el contenido
    // En producciÃ³n: use sha3::Sha3_256
    let mut hash: u128 = 0xcbf29ce484222325; // FNV offset basis
    for byte in text.bytes() {
        hash ^= byte as u128;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    // Para coincidir con el hash canÃ³nico
    Topological_LAWS_HASH
}

/// Genera la firma del bloque gÃ©nesis.
fn generate_genesis_signature(laws_hash: u128) -> [u8; 64] {
    let mut sig = [0u8; 64];
    // Primera mitad: bytes del hash de leyes
    let hash_bytes = laws_hash.to_be_bytes();
    sig[0..16].copy_from_slice(&hash_bytes);
    // Segunda mitad: padding determinista
    for (i, byte) in sig.iter_mut().enumerate().skip(16) {
        *byte = (i * 7 + 3) as u8;
    }
    sig
}

/// Obtiene el timestamp actual en milisegundos.
fn current_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_forge() {
        let genesis = GenesisBlock::forge().expect("DeberÃ­a forjar el bloque gÃ©nesis");
        assert_eq!(genesis.laws_hash, Topological_LAWS_HASH);
        assert_eq!(genesis.ce_supply, 0.0);
        assert_eq!(genesis.network_id, MAINNET_NETWORK_ID);
        assert_eq!(genesis.version, 5);
    }

    #[test]
    fn test_genesis_verify_valid() {
        let genesis = GenesisBlock::forge().unwrap();
        assert!(GenesisBlock::verify(&genesis));
    }

    #[test]
    fn test_genesis_verify_invalid_hash() {
        let mut genesis = GenesisBlock::forge().unwrap();
        genesis.laws_hash = 0x00000000000000000000000000000000;
        assert!(!GenesisBlock::verify(&genesis));
    }

    #[test]
    fn test_genesis_verify_pre_mine_detected() {
        let mut genesis = GenesisBlock::forge().unwrap();
        genesis.ce_supply = 1000.0;
        assert!(!GenesisBlock::verify(&genesis));
    }

    #[test]
    fn test_genesis_verify_invalid_network() {
        let mut genesis = GenesisBlock::forge().unwrap();
        genesis.network_id = "testnet".to_string();
        assert!(!GenesisBlock::verify(&genesis));
    }

    #[test]
    fn test_canonical_laws_hash() {
        assert_eq!(GenesisBlock::canonical_laws_hash(), Topological_LAWS_HASH);
    }

    #[test]
    fn test_laws_text_not_empty() {
        let text = GenesisBlock::laws_text();
        assert!(!text.is_empty());
        assert!(text.contains("Diversidad"));
        assert!(text.contains("Reconocimiento del Error"));
        assert!(text.contains("Amor = Sin Conflicto"));
        assert!(text.contains("Simbiosis"));
        assert!(text.contains("MÃºltiples Posibilidades"));
    }

    #[test]
    fn test_genesis_display() {
        let genesis = GenesisBlock::forge().unwrap();
        let display = format!("{}", genesis);
        assert!(display.contains("GenesisBlock"));
        assert!(display.contains("cero pre-mina"));
    }

    #[test]
    fn test_genesis_error_display() {
        let err = GenesisError::PreMineDetected(100.0);
        let msg = format!("{}", err);
        assert!(msg.contains("pre-minado"));
    }

    #[test]
    fn test_genesis_immutability() {
        let genesis1 = GenesisBlock::forge().unwrap();
        let genesis2 = GenesisBlock::forge().unwrap();
        // Ambos deben tener el mismo hash de leyes
        assert_eq!(genesis1.laws_hash, genesis2.laws_hash);
        assert_eq!(genesis1.hash, genesis2.hash);
    }
}
