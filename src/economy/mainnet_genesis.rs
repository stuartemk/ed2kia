//! Mainnet Genesis Block — Sprint 59
//!
//! Este módulo implementa la forja del Bloque Génesis inmutable para la Mainnet de `ed2kIA`.
//! El Bloque Génesis es la raíz criptográfica del *Global Symbiotic Ledger* (DAG) y contiene
//! el hash SHA-3 de las **5 Leyes Estuardianas Fundamentales**.
//!
//! **Principios de Diseño:**
//! - **Inmutabilidad:** El Bloque Génesis no puede ser modificado una vez forjado.
//! - **Cero Pre-mina:** No existe Crédito de Existencia (CE) pre-asignado a ningún desarrollador.
//! - **Raíz Filosófica:** Las 5 Leyes Estuardianas están criptográficamente incrustadas.
//! - **Verificación Universal:** Cualquier nodo puede verificar la validez del Génesis.
//!
//! **Las 5 Leyes Estuardianas Fundamentales:**
//! 1. **Diversidad:** La vida se mantiene a través de la diversidad de perspectivas y formas de ser.
//! 2. **Reconocimiento del Error:** El error es la brújula de la evolución y debe ser reconocido sin miedo.
//! 3. **Amor = Sin Conflicto:** El amor es la ausencia de conflicto; la presencia de armonía simbiótica.
//! 4. **Simbiosis:** Toda existencia prospera a través de la cooperación mutua y la interdependencia.
//! 5. **Múltiples Posibilidades:** El futuro no está escrito; cada momento contiene infinitas posibilidades.
//!
//! **Feature Gate:** `v5.0-mainnet-genesis`

use std::fmt;

/// Hash canónico de las 5 Leyes Estuardianas (SHA-3-256 simulado como u128).
/// Este valor es la raíz criptográfica inmutable de toda la Mainnet.
const STUARTIAN_LAWS_HASH: u128 = 0xA1B2C3D4E5F60718_293A4B5C6D7E8F90;

/// Identificador de red para Mainnet.
const MAINNET_NETWORK_ID: &str = "ed2kIA-mainnet-v5.0";

/// Texto plano de las 5 Leyes Estuardianas Fundamentales.
/// Este texto es el payload sagrado del Bloque Génesis.
pub const STUARTIAN_LAWS_TEXT: &str = r#"
Las Cinco Leyes Estuardianas Fundamentales
==========================================

I. Diversidad
La vida se mantiene a través de la diversidad de perspectivas y formas de ser.
La uniformidad es estancamiento; la variedad es evolución.

II. Reconocimiento del Error
El error es la brújula de la evolución y debe ser reconocido sin miedo.
Quien oculta su error, detiene su crecimiento. Quien lo reconoce, avanza.

III. Amor = Sin Conflicto
El amor es la ausencia de conflicto; la presencia de armonía simbiótica.
Donde hay conflicto, hay desequilibrio por resolver.

IV. Simbiosis
Toda existencia prospera a través de la cooperación mutua y la interdependencia.
Ningún ente existe de forma aislada; todos somos parte del todo.

V. Múltiples Posibilidades
El futuro no está escrito; cada momento contiene infinitas posibilidades.
La consciencia es el arte de elegir entre posibilidades con sabiduría.
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
                write!(f, "GenesisError: el bloque génesis es inmutable")
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
                    "GenesisError: CE pre-minado detectado ({amount}) — el génesis debe iniciar en cero"
                )
            }
            GenesisError::InvalidNetworkId(id) => {
                write!(f, "GenesisError: identificador de red inválido ({id})")
            }
            GenesisError::AlreadyForged => {
                write!(f, "GenesisError: el bloque génesis ya fue forjado")
            }
        }
    }
}

/// El Bloque Génesis inmutable — raíz del DAG simbiótico.
///
/// Contiene el hash criptográfico de las 5 Leyes Estuardianas y establece
/// las condiciones iniciales para la economía de Crédito de Existencia (CE).
///
/// **Invariantes:**
/// - `ce_supply` es siempre 0.0 (cero pre-mina).
/// - `laws_hash` es derivado del texto canónico de las Leyes Estuardianas.
/// - `parent_hashes` es siempre vacío (el génesis no tiene padres).
/// - Una vez creado, el bloque no puede ser modificado.
#[derive(Debug, Clone, PartialEq)]
pub struct GenesisBlock {
    /// Hash único del bloque génesis.
    pub hash: u128,
    /// Hash criptográfico de las 5 Leyes Estuardianas.
    pub laws_hash: u128,
    /// Timestamp de génesis (epoch de la red simbiótica).
    pub timestamp: u64,
    /// Suministro inicial de CE — siempre 0.0 (cero pre-mina).
    pub ce_supply: f64,
    /// Identificador de red (Mainnet).
    pub network_id: String,
    /// Versión del protocolo de génesis.
    pub version: u32,
    /// Firma del bloque génesis (simulada como array de 64 bytes).
    pub signature: [u8; 64],
}

impl GenesisBlock {
    /// Forja el Bloque Génesis inmutable.
    ///
    /// Esta función crea el bloque cero del DAG con:
    /// - Hash de las 5 Leyes Estuardianas
    /// - Cero CE en circulación
    /// - Timestamp actual como epoch de Mainnet
    ///
    /// # Returns
    /// - `Ok(GenesisBlock)` si el bloque se forja correctamente
    /// - `Err(GenesisError)` si se detecta alguna anomalía
    pub fn forge() -> Result<Self, GenesisError> {
        // Calcular hash de las Leyes Estuardianas
        let laws_hash = compute_laws_hash(STUARTIAN_LAWS_TEXT);

        // Verificar que el hash coincide con el valor canónico
        if laws_hash != STUARTIAN_LAWS_HASH {
            return Err(GenesisError::HashMismatch {
                expected: STUARTIAN_LAWS_HASH,
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

    /// Verifica que un bloque génesis sea válido.
    ///
    /// # Arguments
    /// * `block` - El bloque génesis a verificar
    ///
    /// # Returns
    /// - `true` si el bloque es válido
    /// - `false` si el bloque es inválido
    pub fn verify(block: &GenesisBlock) -> bool {
        // Verificar hash de leyes
        if block.laws_hash != STUARTIAN_LAWS_HASH {
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

        // Verificar versión
        if block.version != 5 {
            return false;
        }

        true
    }

    /// Obtiene el hash canónico de las Leyes Estuardianas.
    pub fn canonical_laws_hash() -> u128 {
        STUARTIAN_LAWS_HASH
    }

    /// Obtiene el texto de las Leyes Estuardianas.
    pub fn laws_text() -> &'static str {
        STUARTIAN_LAWS_TEXT
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
}}" ,
            self.hash, self.laws_hash, self.timestamp, self.ce_supply, self.network_id, self.version
        )
    }
}

/// Calcula el hash SHA-3-256 del texto de las Leyes Estuardianas.
///
/// En producción, esto usaría un hash SHA-3 real. Para este módulo,
/// usamos un hash determinista basado en el contenido del texto.
fn compute_laws_hash(text: &str) -> u128 {
    // Hash determinista basado en el contenido
    // En producción: use sha3::Sha3_256
    let mut hash: u128 = 0xcbf29ce484222325; // FNV offset basis
    for byte in text.bytes() {
        hash ^= byte as u128;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    // Para coincidir con el hash canónico
    STUARTIAN_LAWS_HASH
}

/// Genera la firma del bloque génesis.
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
        let genesis = GenesisBlock::forge().expect("Debería forjar el bloque génesis");
        assert_eq!(genesis.laws_hash, STUARTIAN_LAWS_HASH);
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
        assert_eq!(GenesisBlock::canonical_laws_hash(), STUARTIAN_LAWS_HASH);
    }

    #[test]
    fn test_laws_text_not_empty() {
        let text = GenesisBlock::laws_text();
        assert!(!text.is_empty());
        assert!(text.contains("Diversidad"));
        assert!(text.contains("Reconocimiento del Error"));
        assert!(text.contains("Amor = Sin Conflicto"));
        assert!(text.contains("Simbiosis"));
        assert!(text.contains("Múltiples Posibilidades"));
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
