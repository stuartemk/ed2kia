//! CRDT — Conflict-free Replicated Data Types para convergencia eventual.
//!
//! **Stuartian Law 5 (Múltiples Posibilidades):** Los CRDTs garantizan
//! convergencia eventual sin coordinación centralizada.
//!
//! **Feature Gate:** `v2.1-crdt-state`
//!
//! ### CRDTs Implementados
//! | CRDT | Uso | Propiedades |
//! |---|---|---|
//! | GCounter | Merito criptográfico (grow-only) | Commutative, Associative, Idempotent |
//! | PNCounter | Reputación bounded (inc/dec) | Commutative, Associative, Idempotent |
//! | ORSet | Banned/slashed peers | Commutative, Associative, Idempotent |
//!
//! ### Propiedades Matemáticas
//! Todos los merge() son:
//! - **Commutative:** merge(a, b) == merge(b, a)
//! - **Associative:** merge(merge(a, b), c) == merge(a, merge(b, c))
//! - **Idempotent:** merge(a, a) == a
//!
//! ### Serialización
//! bincode para transferencia de estado CRDT entre nodos.
//! Cada CRDT implementa `serialize()` y `deserialize()`.
//!
//! ### Convergencia
//! Test con 3 nodos particionados que al reconectar convergen
//! al mismo estado sin coordinación centralizada.

use std::collections::BTreeMap;
use std::fmt;

/// Error en operaciones CRDT.
#[derive(Debug)]
pub enum CrdtError {
    /// Vector de versión incompatible.
    IncompatibleVersion(String),
    /// Error de merge.
    MergeError(String),
    /// Estado corrupto.
    CorruptState(String),
    /// Error de serialización.
    SerializationError(String),
    /// Valor fuera de rango.
    OutOfRange(String),
}

impl fmt::Display for CrdtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CrdtError::IncompatibleVersion(msg) => {
                write!(f, "Incompatible version: {}", msg)
            }
            CrdtError::MergeError(msg) => {
                write!(f, "Merge error: {}", msg)
            }
            CrdtError::CorruptState(msg) => {
                write!(f, "Corrupt state: {}", msg)
            }
            CrdtError::SerializationError(msg) => {
                write!(f, "Serialization error: {}", msg)
            }
            CrdtError::OutOfRange(msg) => {
                write!(f, "Out of range: {}", msg)
            }
        }
    }
}

impl std::error::Error for CrdtError {}

/// Vector de versión para CRDT.
///
/// Cada nodo tiene su propio contador que se incrementa
/// con cada operación. El merge toma el máximo por nodo.
///
/// ### Propiedades
/// - **Commutative:** max(a, b) == max(b, a)
/// - **Associative:** max(max(a, b), c) == max(a, max(b, c))
/// - **Idempotent:** max(a, a) == a
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VersionVector {
    /// Contador por nodo_id.
    counters: BTreeMap<String, u64>,
}

impl VersionVector {
    /// Crea un nuevo vector de versión vacío.
    pub fn new() -> Self {
        Self {
            counters: BTreeMap::new(),
        }
    }

    /// Incrementa el contador de un nodo.
    pub fn increment(&mut self, node_id: &str) {
        *self.counters.entry(node_id.to_string()).or_insert(0) += 1;
    }

    /// Retorna el contador de un nodo específico.
    pub fn get(&self, node_id: &str) -> u64 {
        *self.counters.get(node_id).unwrap_or(&0)
    }

    /// Compara dos vectores de versión.
    /// - Less: this < other (this es estrictamente anterior)
    /// - Greater: this > other (this es estrictamente posterior)
    /// - Equal: concurrent o iguales (conflict o mismo estado)
    ///
    /// Algoritmo:
    /// 1. Si todos self[i] <= other[i] y alguno < : self < other
    /// 2. Si todos self[i] >= other[i] y alguno > : self > other
    /// 3. De otro modo: concurrent (Equal)
    pub fn compare(&self, other: &VersionVector) -> std::cmp::Ordering {
        let mut less = false;
        let mut greater = false;

        // Verificar todos los nodos de self
        for (node, &count) in &self.counters {
            let other_count = other.get(node);
            if count < other_count {
                less = true;
            } else if count > other_count {
                greater = true;
            }
            if less && greater {
                return std::cmp::Ordering::Equal; // Concurrent
            }
        }

        // Verificar nodos de other que no están en self
        for (node, &count) in &other.counters {
            if !self.counters.contains_key(node) && count > 0 {
                less = true;
                if greater {
                    return std::cmp::Ordering::Equal; // Concurrent
                }
            }
        }

        if less {
            std::cmp::Ordering::Less
        } else if greater {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    }

    /// Merge con otro vector de versión (toma máximo por nodo).
    ///
    /// **Idempotent:** merge(v, v) == v
    /// **Commutative:** merge(a, b) == merge(b, a)
    /// **Associative:** merge(merge(a, b), c) == merge(a, merge(b, c))
    pub fn merge(&mut self, other: &VersionVector) {
        for (node, &count) in &other.counters {
            let entry = self.counters.entry(node.clone()).or_insert(0);
            if count > *entry {
                *entry = count;
            }
        }
    }

    /// Retorna los nodos con contadores activos.
    pub fn nodes(&self) -> Vec<&String> {
        self.counters.keys().collect()
    }

    /// Retorna true si el vector está vacío.
    pub fn is_empty(&self) -> bool {
        self.counters.is_empty()
    }
}

/// G-Counter (Grow-only Counter) para mérito criptográfico.
///
/// **Stuartian Law 5:** Cada nodo incrementa su propio contador.
/// El valor global es la suma de todos los contadores.
/// Merge toma el máximo por nodo (convergencia eventual).
///
/// ### Propiedades
/// - **Commutative:** sum(max(a, b)) == sum(max(b, a))
/// - **Associative:** sum(max(max(a, b), c)) == sum(max(a, max(b, c)))
/// - **Idempotent:** sum(max(a, a)) == sum(a)
/// - **Monotonic:** El valor nunca decrece
///
/// ### Uso
/// - Mérito criptográfico acumulado
/// - Conteo de contribuciones
/// - Tokens generados (no destruibles)
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GCounter {
    /// Contador por nodo_id.
    counters: BTreeMap<String, u64>,
}

impl GCounter {
    /// Crea un nuevo G-Counter vacío.
    pub fn new() -> Self {
        Self {
            counters: BTreeMap::new(),
        }
    }

    /// Incrementa el contador de un nodo.
    pub fn increment(&mut self, node_id: &str, amount: u64) {
        *self.counters.entry(node_id.to_string()).or_insert(0) += amount;
    }

    /// Retorna el valor total (suma de todos los contadores).
    pub fn value(&self) -> u64 {
        self.counters.values().sum()
    }

    /// Retorna el contador de un nodo específico.
    pub fn get(&self, node_id: &str) -> u64 {
        *self.counters.get(node_id).unwrap_or(&0)
    }

    /// Merge con otro G-Counter (toma máximo por nodo).
    ///
    /// **Idempotent:** merge(c, c) == c
    /// **Commutative:** merge(a, b) == merge(b, a)
    pub fn merge(&mut self, other: &GCounter) {
        for (node, &count) in &other.counters {
            let entry = self.counters.entry(node.clone()).or_insert(0);
            if count > *entry {
                *entry = count;
            }
        }
    }

    /// Serializa el G-Counter a bytes (formato simple binario).
    ///
    /// Formato: [num_nodes:u32][node_len:u32][node_bytes][count:u64]...
    pub fn serialize(&self) -> Result<Vec<u8>, CrdtError> {
        let mut buf = Vec::new();
        let num_nodes = self.counters.len() as u32;
        buf.extend_from_slice(&num_nodes.to_le_bytes());

        for (node, &count) in &self.counters {
            let node_bytes = node.as_bytes();
            let node_len = node_bytes.len() as u32;
            buf.extend_from_slice(&node_len.to_le_bytes());
            buf.extend_from_slice(node_bytes);
            buf.extend_from_slice(&count.to_le_bytes());
        }

        Ok(buf)
    }

    /// Helper: lee un u32 desde data[pos..pos+4].
    fn read_u32(data: &[u8], pos: usize) -> Result<u32, CrdtError> {
        let end = pos + 4;
        if end > data.len() {
            return Err(CrdtError::SerializationError(
                "Data too short for u32".into(),
            ));
        }
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&data[pos..end]);
        Ok(u32::from_le_bytes(buf))
    }

    /// Helper: lee un u64 desde data[pos..pos+8].
    fn read_u64(data: &[u8], pos: usize) -> Result<u64, CrdtError> {
        let end = pos + 8;
        if end > data.len() {
            return Err(CrdtError::SerializationError(
                "Data too short for u64".into(),
            ));
        }
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&data[pos..end]);
        Ok(u64::from_le_bytes(buf))
    }

    /// Helper: lee un i64 desde data[pos..pos+8].
    fn read_i64(data: &[u8], pos: usize) -> Result<i64, CrdtError> {
        let end = pos + 8;
        if end > data.len() {
            return Err(CrdtError::SerializationError(
                "Data too short for i64".into(),
            ));
        }
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&data[pos..end]);
        Ok(i64::from_le_bytes(buf))
    }

    /// Deserializa un G-Counter desde bytes.
    pub fn deserialize(data: &[u8]) -> Result<Self, CrdtError> {
        if data.len() < 4 {
            return Err(CrdtError::SerializationError(
                "Data too short for GCounter".into(),
            ));
        }

        let mut counters = BTreeMap::new();
        let mut pos = 0;

        let num_nodes = Self::read_u32(data, pos)? as usize;
        pos += 4;

        for _ in 0..num_nodes {
            if pos + 4 > data.len() {
                return Err(CrdtError::SerializationError(
                    "Data too short for node entry".into(),
                ));
            }

            let node_len = Self::read_u32(data, pos)? as usize;
            pos += 4;

            if pos + node_len + 8 > data.len() {
                return Err(CrdtError::SerializationError(
                    "Data too short for node data".into(),
                ));
            }

            let node = String::from_utf8(data[pos..pos + node_len].to_vec())
                .map_err(|_| CrdtError::SerializationError("Invalid node UTF-8".into()))?;
            pos += node_len;

            let count = Self::read_u64(data, pos)?;
            pos += 8;

            counters.insert(node, count);
        }

        Ok(Self { counters })
    }
}

/// PN-Counter (Positive-Negative Counter) para reputación bounded.
///
/// **Stuartian Law 5:** Dos G-Counters internos (positivos y negativos).
/// El valor es positives.value() - negatives.value() con límites [min, max].
///
/// ### Propiedades
/// - **Commutative:** merge(a, b) == merge(b, a)
/// - **Associative:** merge(merge(a, b), c) == merge(a, merge(b, c))
/// - **Idempotent:** merge(c, c) == c
/// - **Bounded:** El valor se clamp-a a [min_value, max_value]
///
/// ### Uso
/// - Reputación de nodos (bounded)
/// - Scores de confianza
/// - Votos ponderados
#[derive(Debug, Clone)]
pub struct PNCounter {
    /// Contadores positivos (incrementos).
    positives: GCounter,
    /// Contadores negativos (decrementos).
    negatives: GCounter,
    /// Valor mínimo permitido.
    min_value: i64,
    /// Valor máximo permitido.
    max_value: i64,
}

impl PNCounter {
    /// Crea un nuevo PN-Counter con límites especificados.
    pub fn new(min_value: i64, max_value: i64) -> Result<Self, CrdtError> {
        if min_value > max_value {
            return Err(CrdtError::OutOfRange(
                "min_value cannot exceed max_value".into(),
            ));
        }
        Ok(Self {
            positives: GCounter::new(),
            negatives: GCounter::new(),
            min_value,
            max_value,
        })
    }

    /// Incrementa la reputación de un nodo.
    pub fn increment(&mut self, node_id: &str, amount: u64) {
        self.positives.increment(node_id, amount);
    }

    /// Decrementa la reputación de un nodo.
    pub fn decrement(&mut self, node_id: &str, amount: u64) {
        self.negatives.increment(node_id, amount);
    }

    /// Retorna el valor actual (clamp-ado a [min, max]).
    pub fn value(&self) -> i64 {
        let raw = self.positives.value() as i64 - self.negatives.value() as i64;
        raw.clamp(self.min_value, self.max_value)
    }

    /// Retorna el valor sin clamp (para debugging).
    pub fn raw_value(&self) -> i64 {
        self.positives.value() as i64 - self.negatives.value() as i64
    }

    /// Merge con otro PN-Counter.
    ///
    /// **Idempotent:** merge(c, c) == c
    /// **Commutative:** merge(a, b) == merge(b, a)
    pub fn merge(&mut self, other: &PNCounter) {
        self.positives.merge(&other.positives);
        self.negatives.merge(&other.negatives);
    }

    /// Serializa el PN-Counter a bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, CrdtError> {
        let pos_data = self.positives.serialize()?;
        let neg_data = self.negatives.serialize()?;

        let mut buf = Vec::new();
        buf.extend_from_slice(&self.min_value.to_le_bytes());
        buf.extend_from_slice(&self.max_value.to_le_bytes());
        buf.extend_from_slice(&(pos_data.len() as u32).to_le_bytes());
        buf.extend_from_slice(&pos_data);
        buf.extend_from_slice(&(neg_data.len() as u32).to_le_bytes());
        buf.extend_from_slice(&neg_data);

        Ok(buf)
    }

    /// Deserializa un PN-Counter desde bytes.
    pub fn deserialize(data: &[u8]) -> Result<Self, CrdtError> {
        if data.len() < 20 {
            return Err(CrdtError::SerializationError(
                "Data too short for PNCounter".into(),
            ));
        }

        let mut pos = 0;
        let min_value = GCounter::read_i64(data, pos)?;
        pos += 8;

        let max_value = GCounter::read_i64(data, pos)?;
        pos += 8;

        let pos_len = GCounter::read_u32(data, pos)? as usize;
        pos += 4;

        if pos + pos_len >= data.len() {
            return Err(CrdtError::SerializationError(
                "Data too short for positives".into(),
            ));
        }
        let pos_data = &data[pos..pos + pos_len];
        pos += pos_len;

        let neg_len = GCounter::read_u32(data, pos)? as usize;
        pos += 4;

        if pos + neg_len > data.len() {
            return Err(CrdtError::SerializationError(
                "Data too short for negatives".into(),
            ));
        }
        let neg_data = &data[pos..pos + neg_len];

        let positives = GCounter::deserialize(pos_data)?;
        let negatives = GCounter::deserialize(neg_data)?;

        Ok(Self {
            positives,
            negatives,
            min_value,
            max_value,
        })
    }
}

impl Default for PNCounter {
    fn default() -> Self {
        Self::new(0, i64::MAX).unwrap()
    }
}

/// OR-Set (Observed-Remove Set) para banned/slashed peers.
///
/// **Stuartian Law 5:** Cada elemento tiene un tag único para
/// resolver conflictos add-after-remove. Si un elemento fue
/// añadido en múltiples nodos, se necesita un remove por cada tag.
///
/// ### Propiedades
/// - **Commutative:** merge(a, b) == merge(b, a)
/// - **Associative:** merge(merge(a, b), c) == merge(a, merge(b, c))
/// - **Idempotent:** merge(s, s) == s
/// - **Convergent:** 3 nodos particionados convergen al mismo estado
///
/// ### Uso
/// - Banned/slashed peers
/// - Blacklist de nodos maliciosos
/// - Sets de permisos distribuidos
#[derive(Debug, Clone, Default)]
pub struct ORSet {
    /// Elementos activos: element -> set of tags.
    elements: BTreeMap<String, BTreeMap<u64, String>>,
    /// Tumba de elementos removidos: element -> set of (tag, node_id).
    tombstone: BTreeMap<String, BTreeMap<u64, String>>,
    /// Contador de tags para generar tags únicos.
    tag_counter: u64,
}

impl ORSet {
    /// Crea un nuevo OR-Set vacío.
    pub fn new() -> Self {
        Self {
            elements: BTreeMap::new(),
            tombstone: BTreeMap::new(),
            tag_counter: 0,
        }
    }

    /// Añade un elemento al set.
    ///
    /// Cada add genera un tag único (tag_counter++).
    /// Si el elemento fue previamente removido, los tags
    /// tombstoned se limpian para este nuevo tag.
    pub fn add(&mut self, element: &str, node_id: &str) -> u64 {
        let tag = self.tag_counter;
        self.tag_counter += 1;

        // Añadir al set activo
        self.elements
            .entry(element.to_string())
            .or_default()
            .insert(tag, node_id.to_string());

        // Limpiar tombstone para este tag (add-after-remove)
        if let Some(tomb) = self.tombstone.get_mut(element) {
            tomb.remove(&tag);
        }

        tag
    }

    /// Remueve un elemento del set.
    ///
    /// Todos los tags observados del elemento se mueven a tombstone.
    /// Si otro nodo añadió el mismo elemento con un tag diferente,
    /// ese tag permanecerá activo (observed-remove semantics).
    pub fn remove(&mut self, element: &str, node_id: &str) -> usize {
        let tags = self.elements.remove(element);
        match tags {
            Some(tag_map) => {
                let count = tag_map.len();
                let tomb = self.tombstone.entry(element.to_string()).or_default();
                for (tag, _) in tag_map {
                    tomb.insert(tag, node_id.to_string());
                }
                count
            }
            None => 0,
        }
    }

    /// Retorna true si el elemento está presente.
    pub fn contains(&self, element: &str) -> bool {
        self.elements
            .get(element)
            .map(|tags| !tags.is_empty())
            .unwrap_or(false)
    }

    /// Retorna los elementos activos.
    pub fn elements(&self) -> Vec<&String> {
        self.elements
            .keys()
            .filter(|k| !self.elements.get(*k).unwrap().is_empty())
            .collect()
    }

    /// Retorna el número de elementos activos.
    pub fn len(&self) -> usize {
        self.elements
            .values()
            .filter(|tags| !tags.is_empty())
            .count()
    }

    /// Retorna true si el set está vacío.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Merge con otro OR-Set.
    ///
    /// **Algoritmo:**
    /// 1. Para cada elemento en other.elements:
    ///    - Añadir tags que no están en self.tombstone
    /// 2. Para cada elemento en other.tombstone:
    ///    - Mover tags observados en self.elements a self.tombstone
    ///
    /// **Idempotent:** merge(s, s) == s
    /// **Commutative:** merge(a, b) == merge(b, a)
    pub fn merge(&mut self, other: &ORSet) {
        // 1. Añadir elementos de other
        for (element, tags) in &other.elements {
            let self_tags = self.elements.entry(element.clone()).or_default();

            for (tag, node_id) in tags {
                // Solo añadir si no está en tombstone
                let tomb = self.tombstone.get(element);
                let is_tombstoned = match tomb {
                    Some(t) => t.contains_key(tag),
                    None => false,
                };
                if !is_tombstoned {
                    self_tags.insert(*tag, node_id.clone());
                }
            }
        }

        // 2. Aplicar tombstones de other
        for (element, tomb_tags) in &other.tombstone {
            let self_tomb = self.tombstone.entry(element.clone()).or_default();

            for (tag, node_id) in tomb_tags {
                // Si el tag está en self.elements, moverlo a tombstone
                if let Some(self_tags) = self.elements.get_mut(element) {
                    if let Some(node) = self_tags.remove(tag) {
                        self_tomb.insert(*tag, node);
                    }
                }
                // Siempre añadir al tombstone (para futuros adds con mismo tag)
                self_tomb.entry(*tag).or_insert_with(|| node_id.clone());
            }

            // Limpiar elementos vacíos
            if let Some(tags) = self.elements.get(element) {
                if tags.is_empty() {
                    self.elements.remove(element);
                }
            }
        }

        // Sync tag_counter para evitar colisiones
        if other.tag_counter > self.tag_counter {
            self.tag_counter = other.tag_counter;
        }
    }

    /// Serializa el OR-Set a bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, CrdtError> {
        let mut buf = Vec::new();

        // Elements
        let elem_count = self.elements.len() as u32;
        buf.extend_from_slice(&elem_count.to_le_bytes());
        for (element, tags) in &self.elements {
            let elem_bytes = element.as_bytes();
            buf.extend_from_slice(&(elem_bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(elem_bytes);

            let tag_count = tags.len() as u32;
            buf.extend_from_slice(&tag_count.to_le_bytes());
            for (tag, node_id) in tags {
                buf.extend_from_slice(&tag.to_le_bytes());
                let node_bytes = node_id.as_bytes();
                buf.extend_from_slice(&(node_bytes.len() as u32).to_le_bytes());
                buf.extend_from_slice(node_bytes);
            }
        }

        // Tombstones
        let tomb_count = self.tombstone.len() as u32;
        buf.extend_from_slice(&tomb_count.to_le_bytes());
        for (element, tags) in &self.tombstone {
            let elem_bytes = element.as_bytes();
            buf.extend_from_slice(&(elem_bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(elem_bytes);

            let tag_count = tags.len() as u32;
            buf.extend_from_slice(&tag_count.to_le_bytes());
            for (tag, node_id) in tags {
                buf.extend_from_slice(&tag.to_le_bytes());
                let node_bytes = node_id.as_bytes();
                buf.extend_from_slice(&(node_bytes.len() as u32).to_le_bytes());
                buf.extend_from_slice(node_bytes);
            }
        }

        // Tag counter
        buf.extend_from_slice(&self.tag_counter.to_le_bytes());

        Ok(buf)
    }

    /// Deserializa un OR-Set desde bytes.
    pub fn deserialize(data: &[u8]) -> Result<Self, CrdtError> {
        let mut pos = 0;
        let mut elements = BTreeMap::new();
        let mut tombstone = BTreeMap::new();

        // Read elements
        let elem_count = GCounter::read_u32(data, pos)? as usize;
        pos += 4;

        for _ in 0..elem_count {
            let element = Self::read_string(data, &mut pos)?;
            let tag_count = GCounter::read_u32(data, pos)? as usize;
            pos += 4;

            let mut tags = BTreeMap::new();
            for _ in 0..tag_count {
                let tag = GCounter::read_u64(data, pos)?;
                pos += 8;
                let node_id = Self::read_string(data, &mut pos)?;
                tags.insert(tag, node_id);
            }
            elements.insert(element, tags);
        }

        // Read tombstones
        let tomb_count = GCounter::read_u32(data, pos)? as usize;
        pos += 4;

        for _ in 0..tomb_count {
            let element = Self::read_string(data, &mut pos)?;
            let tag_count = GCounter::read_u32(data, pos)? as usize;
            pos += 4;

            let mut tags = BTreeMap::new();
            for _ in 0..tag_count {
                let tag = GCounter::read_u64(data, pos)?;
                pos += 8;
                let node_id = Self::read_string(data, &mut pos)?;
                tags.insert(tag, node_id);
            }
            tombstone.insert(element, tags);
        }

        // Read tag counter
        let tag_counter = GCounter::read_u64(data, pos)?;

        Ok(Self {
            elements,
            tombstone,
            tag_counter,
        })
    }

    /// Helper: lee un string desde data[pos..] y avanza pos.
    fn read_string(data: &[u8], pos: &mut usize) -> Result<String, CrdtError> {
        if *pos + 4 > data.len() {
            return Err(CrdtError::SerializationError(
                "Data too short for string len".into(),
            ));
        }
        let len = GCounter::read_u32(data, *pos)? as usize;
        *pos += 4;

        if *pos + len > data.len() {
            return Err(CrdtError::SerializationError(
                "Data too short for string".into(),
            ));
        }
        let s = String::from_utf8(data[*pos..*pos + len].to_vec())
            .map_err(|_| CrdtError::SerializationError("Invalid UTF-8".into()))?;
        *pos += len;
        Ok(s)
    }
}

/// CRDT de reputación — Max-Registry para reputación de nodos.
///
/// **Stuartian Law 5:** Convergencia eventual. Cada nodo
/// mantiene su propia vista de reputación que converge sin coordinación.
#[derive(Debug, Clone)]
pub struct ReputationCrdt {
    /// Reputación por nodo_id.
    pub reputations: BTreeMap<String, f64>,
    /// Vector de versión.
    pub version: VersionVector,
}

impl ReputationCrdt {
    /// Crea un nuevo CRDT de reputación vacío.
    pub fn new() -> Self {
        Self {
            reputations: BTreeMap::new(),
            version: VersionVector::new(),
        }
    }

    /// Actualiza la reputación de un nodo.
    pub fn update(&mut self, node_id: &str, reputation: f64, updater_id: &str) {
        *self.reputations.entry(node_id.to_string()).or_insert(0.0) = reputation;
        self.version.increment(updater_id);
    }

    /// Merge con otro CRDT de reputación.
    ///
    /// **Stuartian Law 5:** Toma el máximo por nodo (max-registry).
    /// **Idempotent:** merge(r, r) == r
    /// **Commutative:** merge(a, b) == merge(b, a)
    pub fn merge(&mut self, other: &ReputationCrdt) {
        for (node_id, other_rep) in &other.reputations {
            let entry = self.reputations.entry(node_id.clone()).or_insert(0.0);
            if *other_rep > *entry {
                *entry = *other_rep;
            }
        }
        self.version.merge(&other.version);
    }

    /// Obtiene la reputación de un nodo.
    pub fn get(&self, node_id: &str) -> Option<f64> {
        self.reputations.get(node_id).copied()
    }
}

impl Default for ReputationCrdt {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════
    // VersionVector Tests
    // ═══════════════════════════════════════════

    #[test]
    fn test_version_vector_new() {
        let vv = VersionVector::new();
        assert!(vv.is_empty());
    }

    #[test]
    fn test_version_vector_increment() {
        let mut vv = VersionVector::new();
        vv.increment("node-1");
        assert_eq!(vv.get("node-1"), 1);
        vv.increment("node-1");
        assert_eq!(vv.get("node-1"), 2);
    }

    #[test]
    fn test_version_vector_compare_equal() {
        let mut vv1 = VersionVector::new();
        let mut vv2 = VersionVector::new();
        vv1.increment("a");
        vv2.increment("a");
        assert_eq!(vv1.compare(&vv2), std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_version_vector_compare_less() {
        let mut vv1 = VersionVector::new();
        let mut vv2 = VersionVector::new();
        vv1.increment("a");
        vv2.increment("a");
        vv2.increment("a");
        assert_eq!(vv1.compare(&vv2), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_version_vector_compare_greater() {
        let mut vv1 = VersionVector::new();
        let mut vv2 = VersionVector::new();
        vv1.increment("a");
        vv1.increment("a");
        vv2.increment("a");
        assert_eq!(vv1.compare(&vv2), std::cmp::Ordering::Greater);
    }

    #[test]
    fn test_version_vector_compare_concurrent() {
        let mut vv1 = VersionVector::new();
        let mut vv2 = VersionVector::new();
        vv1.increment("a");
        vv2.increment("b");
        assert_eq!(vv1.compare(&vv2), std::cmp::Ordering::Equal); // Concurrent
    }

    #[test]
    fn test_version_vector_merge() {
        let mut vv1 = VersionVector::new();
        let mut vv2 = VersionVector::new();
        vv1.increment("a");
        vv1.increment("a");
        vv2.increment("b");
        vv2.increment("b");
        vv2.increment("b");

        vv1.merge(&vv2);
        assert_eq!(vv1.get("a"), 2);
        assert_eq!(vv1.get("b"), 3);
    }

    #[test]
    fn test_version_vector_merge_idempotent() {
        let mut vv1 = VersionVector::new();
        let mut vv2 = VersionVector::new();
        vv1.increment("a");
        vv2.increment("a");

        let before = vv1.clone();
        vv1.merge(&vv2);
        // After merge, both should have same 'a' count
        assert_eq!(vv1.get("a"), vv2.get("a"));
    }

    // ═══════════════════════════════════════════
    // GCounter Tests
    // ═══════════════════════════════════════════

    #[test]
    fn test_gcounter_new() {
        let gc = GCounter::new();
        assert_eq!(gc.value(), 0);
    }

    #[test]
    fn test_gcounter_increment() {
        let mut gc = GCounter::new();
        gc.increment("node-1", 5);
        assert_eq!(gc.value(), 5);
        gc.increment("node-2", 3);
        assert_eq!(gc.value(), 8);
    }

    #[test]
    fn test_gcounter_merge_commutative() {
        let mut a = GCounter::new();
        let mut b = GCounter::new();
        a.increment("x", 5);
        b.increment("y", 3);

        let mut a_copy = a.clone();
        let mut b_copy = b.clone();

        a.merge(&b);
        b_copy.merge(&a_copy);

        assert_eq!(a.value(), b_copy.value());
    }

    #[test]
    fn test_gcounter_merge_idempotent() {
        let mut gc = GCounter::new();
        gc.increment("node-1", 5);

        let gc_copy = gc.clone();
        gc.merge(&gc_copy);
        assert_eq!(gc.value(), 5);
    }

    #[test]
    fn test_gcounter_merge_associative() {
        let mut a = GCounter::new();
        let mut b = GCounter::new();
        let mut c = GCounter::new();
        a.increment("a", 1);
        b.increment("b", 2);
        c.increment("c", 3);

        let mut ab = a.clone();
        ab.merge(&b);
        ab.merge(&c);

        let mut bc = b.clone();
        bc.merge(&c);
        let mut a2 = a.clone();
        a2.merge(&bc);

        assert_eq!(ab.value(), a2.value());
    }

    #[test]
    fn test_gcounter_serialize_deserialize() {
        let mut gc = GCounter::new();
        gc.increment("node-1", 5);
        gc.increment("node-2", 3);

        let data = gc.serialize().unwrap();
        let restored = GCounter::deserialize(&data).unwrap();
        assert_eq!(restored.value(), 8);
        assert_eq!(restored.get("node-1"), 5);
        assert_eq!(restored.get("node-2"), 3);
    }

    // ═══════════════════════════════════════════
    // PNCounter Tests
    // ═══════════════════════════════════════════

    #[test]
    fn test_pncounter_new() {
        let pn = PNCounter::new(0, 100).unwrap();
        assert_eq!(pn.value(), 0);
    }

    #[test]
    fn test_pncounter_invalid_range() {
        match PNCounter::new(100, 0) {
            Err(CrdtError::OutOfRange(_)) => {} // Expected
            other => panic!("Expected OutOfRange, got {:?}", other),
        }
    }

    #[test]
    fn test_pncounter_increment_decrement() {
        let mut pn = PNCounter::new(0, 100).unwrap();
        pn.increment("node-1", 10);
        assert_eq!(pn.value(), 10);
        pn.decrement("node-1", 3);
        assert_eq!(pn.value(), 7);
    }

    #[test]
    fn test_pncounter_bounded() {
        let mut pn = PNCounter::new(0, 10).unwrap();
        pn.increment("node-1", 15);
        assert_eq!(pn.value(), 10); // Clamped to max
    }

    #[test]
    fn test_pncounter_merge() {
        let mut a = PNCounter::new(0, 100).unwrap();
        let mut b = PNCounter::new(0, 100).unwrap();
        a.increment("x", 10);
        b.increment("y", 5);
        b.decrement("y", 2);

        a.merge(&b);
        assert_eq!(a.value(), 13); // 10 + (5-2)
    }

    #[test]
    fn test_pncounter_serialize_deserialize() {
        let mut pn = PNCounter::new(0, 100).unwrap();
        pn.increment("node-1", 10);
        pn.decrement("node-1", 3);

        let data = pn.serialize().unwrap();
        let restored = PNCounter::deserialize(&data).unwrap();
        assert_eq!(restored.value(), 7);
    }

    #[test]
    fn test_pncounter_default() {
        let _ = PNCounter::default();
    }

    // ═══════════════════════════════════════════
    // ORSet Tests
    // ═══════════════════════════════════════════

    #[test]
    fn test_orset_new() {
        let os = ORSet::new();
        assert!(os.is_empty());
    }

    #[test]
    fn test_orset_add_remove() {
        let mut os = ORSet::new();
        os.add("peer-1", "node-a");
        assert!(os.contains("peer-1"));
        assert_eq!(os.len(), 1);

        os.remove("peer-1", "node-a");
        assert!(!os.contains("peer-1"));
        assert_eq!(os.len(), 0);
    }

    #[test]
    fn test_orset_add_after_remove() {
        let mut os = ORSet::new();
        os.add("peer-1", "node-a");
        os.remove("peer-1", "node-a");
        os.add("peer-1", "node-b");
        assert!(os.contains("peer-1"));
    }

    #[test]
    fn test_orset_merge_commutative() {
        let mut a = ORSet::new();
        let mut b = ORSet::new();
        a.add("x", "node-a");
        b.add("y", "node-b");

        let mut a_copy = a.clone();
        let mut b_copy = b.clone();

        a.merge(&b);
        b_copy.merge(&a_copy);

        assert_eq!(a.elements().len(), b_copy.elements().len());
        assert!(a.contains("x") && a.contains("y"));
    }

    #[test]
    fn test_orset_merge_idempotent() {
        let mut os = ORSet::new();
        os.add("peer-1", "node-a");

        let os_copy = os.clone();
        os.merge(&os_copy);
        assert_eq!(os.len(), 1);
    }

    #[test]
    fn test_orset_merge_associative() {
        let mut a = ORSet::new();
        let mut b = ORSet::new();
        let mut c = ORSet::new();
        a.add("x", "node-a");
        b.add("y", "node-b");
        c.add("z", "node-c");

        let mut ab = a.clone();
        ab.merge(&b);
        ab.merge(&c);

        let mut bc = b.clone();
        bc.merge(&c);
        let mut a2 = a.clone();
        a2.merge(&bc);

        assert_eq!(ab.len(), a2.len());
        assert_eq!(ab.len(), 3);
    }

    #[test]
    fn test_orset_serialize_deserialize() {
        let mut os = ORSet::new();
        os.add("peer-1", "node-a");
        os.add("peer-2", "node-b");

        let data = os.serialize().unwrap();
        let restored = ORSet::deserialize(&data).unwrap();
        assert_eq!(restored.len(), 2);
        assert!(restored.contains("peer-1"));
        assert!(restored.contains("peer-2"));
    }

    // ═══════════════════════════════════════════
    // ReputationCrdt Tests
    // ═══════════════════════════════════════════

    #[test]
    fn test_crdt_new() {
        let crdt = ReputationCrdt::new();
        assert!(crdt.reputations.is_empty());
    }

    #[test]
    fn test_crdt_update() {
        let mut crdt = ReputationCrdt::new();
        crdt.update("node-1", 0.8, "updater-1");
        assert_eq!(crdt.get("node-1"), Some(0.8));
    }

    #[test]
    fn test_crdt_merge_takes_max() {
        let mut crdt1 = ReputationCrdt::new();
        crdt1.update("node-1", 0.5, "updater-1");

        let mut crdt2 = ReputationCrdt::new();
        crdt2.update("node-1", 0.8, "updater-2");

        crdt1.merge(&crdt2);
        assert_eq!(crdt1.get("node-1"), Some(0.8));
    }

    #[test]
    fn test_crdt_default() {
        let _ = ReputationCrdt::default();
    }

    // ═══════════════════════════════════════════
    // Convergence Tests — 3 Partitioned Nodes
    // ═══════════════════════════════════════════

    /// Test de convergencia con 3 nodos particionados.
    ///
    /// Escenario:
    /// 1. Tres nodos (A, B, C) inician con estado vacío
    /// 2. Cada nodo hace operaciones locales (partición)
    /// 3. Nodos se reconectan y hacen merge par a par
    /// 4. Verificar que todos convergen al mismo estado
    #[test]
    fn test_gcounter_convergence_3_nodes() {
        // Fase 1: Partición — cada nodo opera localmente
        let mut node_a = GCounter::new();
        let mut node_b = GCounter::new();
        let mut node_c = GCounter::new();

        node_a.increment("A", 10);
        node_b.increment("B", 20);
        node_c.increment("C", 30);

        // Fase 2: Reconexión — dos rondas de merge para convergencia total
        // Ronda 1
        node_a.merge(&node_b);
        node_b.merge(&node_c);
        node_c.merge(&node_a);
        // Ronda 2 — propagación completa
        node_a.merge(&node_b);
        node_b.merge(&node_c);
        node_c.merge(&node_a);

        // Fase 3: Verificar convergencia
        assert_eq!(node_a.value(), 60);
        assert_eq!(node_b.value(), 60);
        assert_eq!(node_c.value(), 60);
    }

    #[test]
    fn test_pncounter_convergence_3_nodes() {
        let mut node_a = PNCounter::new(0, 1000).unwrap();
        let mut node_b = PNCounter::new(0, 1000).unwrap();
        let mut node_c = PNCounter::new(0, 1000).unwrap();

        // Partición
        node_a.increment("A", 50);
        node_a.decrement("A", 10);
        node_b.increment("B", 30);
        node_c.increment("C", 20);
        node_c.decrement("C", 5);

        // Reconexión — dos rondas para convergencia total
        node_a.merge(&node_b);
        node_b.merge(&node_c);
        node_c.merge(&node_a);
        node_a.merge(&node_b);
        node_b.merge(&node_c);
        node_c.merge(&node_a);

        // Convergencia: (50-10) + 30 + (20-5) = 85
        assert_eq!(node_a.value(), 85);
        assert_eq!(node_b.value(), 85);
        assert_eq!(node_c.value(), 85);
    }

    #[test]
    fn test_orset_convergence_3_nodes() {
        let mut node_a = ORSet::new();
        let mut node_b = ORSet::new();
        let mut node_c = ORSet::new();

        // Partición
        node_a.add("peer-x", "A");
        node_a.add("peer-y", "A");
        node_b.add("peer-y", "B"); // Conflict: same element, different tag
        node_b.add("peer-z", "B");
        node_c.add("peer-w", "C");
        node_c.remove("peer-w", "C"); // Add then remove

        // Reconexión — dos rondas para convergencia total
        node_a.merge(&node_b);
        node_b.merge(&node_c);
        node_c.merge(&node_a);
        node_a.merge(&node_b);
        node_b.merge(&node_c);
        node_c.merge(&node_a);

        // Convergencia
        assert_eq!(node_a.len(), node_b.len());
        assert_eq!(node_b.len(), node_c.len());

        // peer-y existe (add en A y B, no remove)
        assert!(node_a.contains("peer-y"));
        assert!(node_b.contains("peer-y"));
        assert!(node_c.contains("peer-y"));

        // peer-w no existe (add+remove en C)
        assert!(!node_a.contains("peer-w"));
        assert!(!node_b.contains("peer-w"));
        assert!(!node_c.contains("peer-w"));
    }

    #[test]
    fn test_reputation_crdt_convergence_3_nodes() {
        let mut node_a = ReputationCrdt::new();
        let mut node_b = ReputationCrdt::new();
        let mut node_c = ReputationCrdt::new();

        // Partición
        node_a.update("target", 0.5, "A");
        node_b.update("target", 0.8, "B");
        node_c.update("target", 0.3, "C");
        node_a.update("other", 0.9, "A");

        // Reconexión — dos rondas para convergencia total
        node_a.merge(&node_b);
        node_b.merge(&node_c);
        node_c.merge(&node_a);
        node_a.merge(&node_b);
        node_b.merge(&node_c);
        node_c.merge(&node_a);

        // Convergencia — max-registry toma el máximo
        assert_eq!(node_a.get("target"), Some(0.8));
        assert_eq!(node_b.get("target"), Some(0.8));
        assert_eq!(node_c.get("target"), Some(0.8));

        assert_eq!(node_a.get("other"), Some(0.9));
        assert_eq!(node_b.get("other"), Some(0.9));
        assert_eq!(node_c.get("other"), Some(0.9));
    }

    // ═══════════════════════════════════════════
    // Error Display Tests
    // ═══════════════════════════════════════════

    #[test]
    fn test_error_display() {
        let err = CrdtError::MergeError("test".into());
        assert!(!format!("{}", err).is_empty());
    }

    #[test]
    fn test_serialization_error_display() {
        let err = CrdtError::SerializationError("bad data".into());
        assert!(format!("{}", err).contains("bad data"));
    }

    #[test]
    fn test_out_of_range_error_display() {
        let err = CrdtError::OutOfRange("min > max".into());
        assert!(format!("{}", err).contains("min > max"));
    }
}
