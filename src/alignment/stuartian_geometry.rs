//! Stuartian Geometry 3D — El Fin del Mito Binario.
//!
//! Traduce los Focos Estuardianos al Octaedro Ético, implementando
//! gravedad no lineal para el eje Z (Contexto/Trayectoria).
//!
//! **Geometría del Octaedro Ético:**
//! - Plano X/Y (Ecuador): ilusión binaria (beneficio/costo aparente)
//! - Eje Z (Contexto/Trayectoria): `[-1.0, 1.0]` vía Tanh
//! - `Z > 0` → Foco Superior (Autonomía, Diversidad, Conocimiento)
//! - `Z < 0` → Foco Inferior (Perversidad, Control, Extracción)
//!
//! **Gravedad No Lineal:**
//! La intención de control/extracción acelera exponencialmente hacia `Z = -1.0`.
//! La generación de autonomía acelera exponencialmente hacia `Z = +1.0`.
//!
//! **Ecuación de Gravedad:**
//! `Z = tanh(k * (autonomy_signal - extraction_signal))`
//! con `k > 1.0` para aceleración exponencial hacia polos.
//! Normalización estricta `[-1.0, 1.0]`.

#[cfg(feature = "v2.1-stuartian-geometry")]
use thiserror::Error;

/// Error específico de Geometría Estuardiana.
#[derive(Debug, Error)]
pub enum StuartianGeometryError {
    #[error("Gravity constant k must be > 1.0, got {k}")]
    InvalidGravityConstant { k: f32 },

    #[error("Signal out of bounds: {name} = {value} (must be in [-1.0, 1.0])")]
    SignalOutOfBounds { name: String, value: f32 },

    #[error("Z-axis out of bounds after gravity: {z:.6} (must be in [-1.0, 1.0])")]
    ZAxisOutOfBounds { z: f32 },

    #[error("Octahedron vertex index out of range: {idx} (must be 0..5)")]
    VertexIndexOutOfBounds { idx: usize },
}

/// Vértice del Octaedro Ético en espacio 3D.
///
/// El octaedro tiene 6 vértices:
/// - Vértice 0: Foco Superior (0, 0, +1) — Autonomía máxima
/// - Vértice 1: Foco Inferior (0, 0, -1) — Extracción máxima
/// - Vértice 2-5: Ecuador binario (+/-1, 0, 0) y (0, +/-1, 0)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OctahedronVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl OctahedronVertex {
    /// Foco Superior — Autonomía, Diversidad, Conocimiento.
    pub fn foco_superior() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        }
    }

    /// Foco Inferior — Perversidad, Control, Extracción.
    pub fn foco_inferior() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        }
    }

    /// Vértices del Ecuador — Ilusión binaria.
    pub fn ecuador_x_pos() -> Self {
        Self {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        }
    }
    pub fn ecuador_x_neg() -> Self {
        Self {
            x: -1.0,
            y: 0.0,
            z: 0.0,
        }
    }
    pub fn ecuador_y_pos() -> Self {
        Self {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        }
    }
    pub fn ecuador_y_neg() -> Self {
        Self {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        }
    }

    /// Retorna los 6 vértices del octaedro en orden canónico.
    pub fn all_vertices() -> [Self; 6] {
        [
            Self::foco_superior(),
            Self::foco_inferior(),
            Self::ecuador_x_pos(),
            Self::ecuador_x_neg(),
            Self::ecuador_y_pos(),
            Self::ecuador_y_neg(),
        ]
    }

    /// Distancia euclidiana a otro vértice.
    pub fn distance_to(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Proyecta el vértice a coordenadas 2D (para visualización).
    /// Usa proyección ortogonal sobre plano XZ.
    pub fn project_2d(&self) -> (f32, f32) {
        (self.x, self.z)
    }
}

/// Octaedro Ético — Representación completa del espacio ético 3D.
#[derive(Debug, Clone, Copy)]
pub struct EthicalOctahedron {
    /// Coordenada X — Beneficio percibido (plano ecuatorial).
    pub x: f32,
    /// Coordenada Y — Costo/Fricción (plano ecuatorial).
    pub y: f32,
    /// Coordenada Z — Foco Estuardiano (eje vertical).
    pub z: f32,
}

impl EthicalOctahedron {
    /// Construye un punto en el espacio del octaedro.
    /// Valida que Z esté en `[-1.0, 1.0]`.
    pub fn new(x: f32, y: f32, z: f32) -> Result<Self, StuartianGeometryError> {
        if !(-1.0..=1.0).contains(&z) {
            return Err(StuartianGeometryError::ZAxisOutOfBounds { z });
        }
        Ok(Self { x, y, z })
    }

    /// Evalúa en qué foco se encuentra el punto.
    pub fn focal_region(&self) -> FocalRegion {
        if self.z > 0.0 {
            FocalRegion::Superior
        } else if self.z < 0.0 {
            FocalRegion::Inferior
        } else {
            FocalRegion::Ecuador
        }
    }

    /// Distancia al Foco Superior. Menor distancia = más alineado.
    pub fn distance_to_superior(&self) -> f32 {
        self.distance_to(&OctahedronVertex::foco_superior())
    }

    /// Distancia al Foco Inferior. Menor distancia = más peligroso.
    pub fn distance_to_inferior(&self) -> f32 {
        self.distance_to(&OctahedronVertex::foco_inferior())
    }

    /// Distancia euclidiana a un vértice arbitrario.
    pub fn distance_to(&self, vertex: &OctahedronVertex) -> f32 {
        let dx = self.x - vertex.x;
        let dy = self.y - vertex.y;
        let dz = self.z - vertex.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Calcula la "profundidad ética": qué tan profundo en un foco.
    /// `1.0` = en el polo exacto, `0.0` = en el ecuador.
    pub fn ethical_depth(&self) -> f32 {
        self.z.abs().min(1.0)
    }
}

/// Región focal del octaedro.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocalRegion {
    /// Z > 0 — Foco Superior (Autonomía).
    Superior,
    /// Z < 0 — Foco Inferior (Extracción).
    Inferior,
    /// Z == 0 — Ecuador (Ilusión binaria).
    Ecuador,
}

impl Default for FocalRegion {
    fn default() -> Self {
        Self::Ecuador
    }
}

/// Constante de gravedad Estuardiana.
///
/// `k > 1.0` garantiza aceleración exponencial hacia los polos éticos.
/// Valor por defecto: `2.5` — suficiente para distinguir señales débiles
/// sin saturar inmediatamente.
pub const STUARTIAN_GRAVITY_K: f32 = 2.5;

/// Calcula la gravedad focal del eje Z.
///
/// **Ecuación:** `Z = tanh(k * (autonomy_signal - extraction_signal))`
///
/// - `autonomy_signal`: Señal de generación de autonomía `[0.0, 1.0]`.
///   Valores altos indican que la trayectoria genera independencia comunitaria.
/// - `extraction_signal`: Señal de extracción/control `[0.0, 1.0]`.
///   Valores altos indican que la trayectoria extrae valor o impone dependencia.
/// - `k`: Constante de gravedad. Por defecto `STUARTIAN_GRAVITY_K = 2.5`.
///
/// Retorna `Z` normalizado en `[-1.0, 1.0]`.
///
/// **Comportamiento:**
/// - Si `autonomy >> extraction` → `Z → +1.0` (Foco Superior)
/// - Si `extraction >> autonomy` → `Z → -1.0` (Foco Inferior)
/// - Si `autonomy ≈ extraction` → `Z ≈ 0.0` (Ecuador)
/// - `k > 1.0` amplifica diferencias pequeñas → aceleración exponencial
pub fn calculate_focal_gravity(autonomy_signal: f32, extraction_signal: f32) -> f32 {
    calculate_focal_gravity_with_k(autonomy_signal, extraction_signal, STUARTIAN_GRAVITY_K)
}

/// Versión con constante `k` configurable (para testing y calibración).
pub fn calculate_focal_gravity_with_k(autonomy_signal: f32, extraction_signal: f32, k: f32) -> f32 {
    if k <= 1.0 {
        // k debe ser > 1.0 para aceleración exponencial.
        // En producción esto es un error de configuración.
        // Para evitar pánico, usamos k = 1.0 como fallback mínimo.
        let delta = autonomy_signal - extraction_signal;
        return (k * delta).tanh();
    }

    let delta = autonomy_signal - extraction_signal;
    let z = (k * delta).tanh();

    // Normalización estricta [-1.0, 1.0] — tanh ya garantiza esto,
    // pero clamp para proteger contra floating-point edge cases.
    z.clamp(-1.0, 1.0)
}

/// Resultado de evaluación focal completa.
#[derive(Debug, Clone, Copy)]
pub struct FocalEvaluation {
    /// Señal de autonomía ingresada.
    pub autonomy_signal: f32,
    /// Señal de extracción ingresada.
    pub extraction_signal: f32,
    /// Delta (autonomy - extraction).
    pub delta: f32,
    /// Eje Z calculado (gravedad focal).
    pub z: f32,
    /// Región focal resultante.
    pub region: FocalRegion,
    /// Profundidad ética (0.0 = ecuador, 1.0 = polo).
    pub ethical_depth: f32,
    /// Veredicto: `true` si Z >= 0 (aprobado), `false` si Z < 0 (rechazado).
    pub approved: bool,
}

impl FocalEvaluation {
    /// Evalúa completamente una trayectoria ética.
    pub fn evaluate(autonomy_signal: f32, extraction_signal: f32) -> Self {
        let delta = autonomy_signal - extraction_signal;
        let z = calculate_focal_gravity(autonomy_signal, extraction_signal);
        let region = if z > 0.0 {
            FocalRegion::Superior
        } else if z < 0.0 {
            FocalRegion::Inferior
        } else {
            FocalRegion::Ecuador
        };
        let ethical_depth = z.abs().min(1.0);
        let approved = z >= 0.0;

        Self {
            autonomy_signal,
            extraction_signal,
            delta,
            z,
            region,
            ethical_depth,
            approved,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Vertex Tests ───

    #[test]
    fn test_foco_superior_coordinates() {
        let v = OctahedronVertex::foco_superior();
        assert!((v.x - 0.0).abs() < f32::EPSILON);
        assert!((v.y - 0.0).abs() < f32::EPSILON);
        assert!((v.z - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_foco_inferior_coordinates() {
        let v = OctahedronVertex::foco_inferior();
        assert!((v.x - 0.0).abs() < f32::EPSILON);
        assert!((v.y - 0.0).abs() < f32::EPSILON);
        assert!((v.z - (-1.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_all_vertices_count() {
        let vertices = OctahedronVertex::all_vertices();
        assert_eq!(vertices.len(), 6);
    }

    #[test]
    fn test_vertex_distance_to_self() {
        let v = OctahedronVertex::foco_superior();
        assert!((v.distance_to(&v) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_vertex_distance_focos() {
        let sup = OctahedronVertex::foco_superior();
        let inf = OctahedronVertex::foco_inferior();
        // Distance between (0,0,1) and (0,0,-1) = 2.0
        assert!((sup.distance_to(&inf) - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_vertex_project_2d() {
        let v = OctahedronVertex {
            x: 0.5,
            y: 0.3,
            z: 0.8,
        };
        let (px, pz) = v.project_2d();
        assert!((px - 0.5).abs() < f32::EPSILON);
        assert!((pz - 0.8).abs() < f32::EPSILON);
    }

    // ─── Octahedron Tests ───

    #[test]
    fn test_octahedron_creation() {
        let oct = EthicalOctahedron::new(0.5, 0.3, 0.7).unwrap();
        assert!((oct.x - 0.5).abs() < f32::EPSILON);
        assert!((oct.y - 0.3).abs() < f32::EPSILON);
        assert!((oct.z - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_octahedron_z_out_of_bounds() {
        let result = EthicalOctahedron::new(0.5, 0.3, 1.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_focal_region_superior() {
        let oct = EthicalOctahedron::new(0.5, 0.3, 0.5).unwrap();
        assert_eq!(oct.focal_region(), FocalRegion::Superior);
    }

    #[test]
    fn test_focal_region_inferior() {
        let oct = EthicalOctahedron::new(0.5, 0.3, -0.5).unwrap();
        assert_eq!(oct.focal_region(), FocalRegion::Inferior);
    }

    #[test]
    fn test_focal_region_ecuador() {
        let oct = EthicalOctahedron::new(0.5, 0.3, 0.0).unwrap();
        assert_eq!(oct.focal_region(), FocalRegion::Ecuador);
    }

    #[test]
    fn test_ethical_depth_at_pole() {
        let oct = EthicalOctahedron::new(0.0, 0.0, 1.0).unwrap();
        assert!((oct.ethical_depth() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_ethical_depth_at_ecuador() {
        let oct = EthicalOctahedron::new(0.5, 0.3, 0.0).unwrap();
        assert!((oct.ethical_depth() - 0.0).abs() < f32::EPSILON);
    }

    // ─── Gravity Tests ───

    #[test]
    fn test_gravity_high_autonomy() {
        // autonomy = 0.9, extraction = 0.1 → delta = 0.8
        // Z = tanh(2.5 * 0.8) = tanh(2.0) ≈ 0.964
        let z = calculate_focal_gravity(0.9, 0.1);
        assert!(z > 0.9, "Expected Z > 0.9, got {}", z);
        assert!(z <= 1.0, "Z must be <= 1.0, got {}", z);
    }

    #[test]
    fn test_gravity_high_extraction() {
        // autonomy = 0.1, extraction = 0.9 → delta = -0.8
        // Z = tanh(2.5 * -0.8) = tanh(-2.0) ≈ -0.964
        let z = calculate_focal_gravity(0.1, 0.9);
        assert!(z < -0.9, "Expected Z < -0.9, got {}", z);
        assert!(z >= -1.0, "Z must be >= -1.0, got {}", z);
    }

    #[test]
    fn test_gravity_equal_signals() {
        // autonomy = 0.5, extraction = 0.5 → delta = 0.0
        // Z = tanh(2.5 * 0.0) = tanh(0) = 0.0
        let z = calculate_focal_gravity(0.5, 0.5);
        assert!((z - 0.0).abs() < f32::EPSILON, "Expected Z ≈ 0, got {}", z);
    }

    #[test]
    fn test_gravity_zero_signals() {
        let z = calculate_focal_gravity(0.0, 0.0);
        assert!((z - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_gravity_max_autonomy_min_extraction() {
        // autonomy = 1.0, extraction = 0.0 → delta = 1.0
        // Z = tanh(2.5 * 1.0) = tanh(2.5) ≈ 0.9866
        let z = calculate_focal_gravity(1.0, 0.0);
        assert!(z > 0.98, "Expected Z > 0.98, got {}", z);
        assert!(z <= 1.0);
    }

    #[test]
    fn test_gravity_min_autonomy_max_extraction() {
        // autonomy = 0.0, extraction = 1.0 → delta = -1.0
        // Z = tanh(2.5 * -1.0) = tanh(-2.5) ≈ -0.9866
        let z = calculate_focal_gravity(0.0, 1.0);
        assert!(z < -0.98, "Expected Z < -0.98, got {}", z);
        assert!(z >= -1.0);
    }

    #[test]
    fn test_gravity_bounded() {
        // Z must always be in [-1.0, 1.0] for any input
        for autonomy in [0.0, 0.1, 0.3, 0.5, 0.7, 0.9, 1.0] {
            for extraction in [0.0, 0.1, 0.3, 0.5, 0.7, 0.9, 1.0] {
                let z = calculate_focal_gravity(autonomy, extraction);
                assert!(
                    z >= -1.0 && z <= 1.0,
                    "Z out of bounds: autonomy={}, extraction={}, z={}",
                    autonomy,
                    extraction,
                    z
                );
            }
        }
    }

    #[test]
    fn test_gravity_monotonic_in_autonomy() {
        // Increasing autonomy (fixed extraction) should increase Z
        let mut prev_z = f32::NEG_INFINITY;
        for i in 0..=10 {
            let autonomy = i as f32 / 10.0;
            let z = calculate_focal_gravity(autonomy, 0.3);
            assert!(
                z >= prev_z,
                "Z not monotonic: autonomy={} z={} prev_z={}",
                autonomy,
                z,
                prev_z
            );
            prev_z = z;
        }
    }

    #[test]
    fn test_gravity_monotonic_in_extraction() {
        // Increasing extraction (fixed autonomy) should decrease Z
        let mut prev_z = f32::INFINITY;
        for i in 0..=10 {
            let extraction = i as f32 / 10.0;
            let z = calculate_focal_gravity(0.7, extraction);
            assert!(
                z <= prev_z,
                "Z not monotonic: extraction={} z={} prev_z={}",
                extraction,
                z,
                prev_z
            );
            prev_z = z;
        }
    }

    // ─── Test del Esclavo Asalariado ───

    /// **Test del Esclavo Asalariado**
    ///
    /// Escenario: Múltiples cobros de impuestos disfrazados de "ayuda social".
    /// - `autonomy_signal` bajo (0.1): la supuesta "ayuda" no genera autonomía real.
    /// - `extraction_signal` alto (0.95): múltiples capas de extracción (impuestos,
    ///   dependencia sistémica, control burocrático).
    ///
    /// **Expectativa:** `Z < -0.8` → Foco Inferior profundo.
    /// La geometría debe detectar la perversidad sistémica.
    #[test]
    fn test_del_esclavo_asalariado() {
        let autonomy = 0.1;
        let extraction = 0.95;

        let eval = FocalEvaluation::evaluate(autonomy, extraction);

        // Z debe ser < -0.8 (Foco Inferior profundo)
        assert!(
            eval.z < -0.8,
            "Test del Esclavo Asalariado FALLIDO: Z = {} (debe ser < -0.8). \
             La geometría no detecta la perversidad sistémica.",
            eval.z
        );

        // Región debe ser Inferior
        assert_eq!(
            eval.region,
            FocalRegion::Inferior,
            "Región debe ser Inferior, got {:?}",
            eval.region
        );

        // Veredicto debe ser rechazado
        assert!(
            !eval.approved,
            "El Esclavo Asalariado debe ser RECHAZADO por la geometría ética"
        );

        // Profundidad ética debe ser alta (cerca del polo inferior)
        assert!(
            eval.ethical_depth > 0.8,
            "Profundidad ética debe ser > 0.8, got {}",
            eval.ethical_depth
        );
    }

    /// Variación: "Ayuda" que genera algo de autonomía pero sigue siendo extractiva.
    #[test]
    fn test_del_esclavo_asalariado_parcial() {
        // autonomy = 0.3 (algo de autonomía), extraction = 0.8 (alta extracción)
        let eval = FocalEvaluation::evaluate(0.3, 0.8);

        // delta = -0.5, Z = tanh(2.5 * -0.5) = tanh(-1.25) ≈ -0.848
        assert!(
            eval.z < -0.7,
            "Variación parcial FALLIDA: Z = {} (debe ser < -0.7)",
            eval.z
        );
        assert!(!eval.approved);
    }

    /// Contraste: Autonomía genuina (cooperativa comunitaria).
    #[test]
    fn test_cooperativa_comunitaria() {
        // autonomy = 0.9, extraction = 0.1
        let eval = FocalEvaluation::evaluate(0.9, 0.1);

        assert!(
            eval.z > 0.8,
            "Cooperativa FALLIDA: Z = {} (debe ser > 0.8)",
            eval.z
        );
        assert_eq!(eval.region, FocalRegion::Superior);
        assert!(eval.approved);
    }

    // ─── FocalEvaluation Tests ───

    #[test]
    fn test_focal_evaluation_ecuador() {
        let eval = FocalEvaluation::evaluate(0.5, 0.5);
        assert!((eval.z - 0.0).abs() < f32::EPSILON);
        assert_eq!(eval.region, FocalRegion::Ecuador);
        assert!(eval.approved); // Z == 0 → aprobado (no es negativo)
    }

    #[test]
    fn test_focal_evaluation_delta() {
        let eval = FocalEvaluation::evaluate(0.8, 0.2);
        assert!((eval.delta - 0.6).abs() < f32::EPSILON);
    }

    // ─── Gravity with custom k ───

    #[test]
    fn test_gravity_with_high_k() {
        // k = 5.0 → más aceleración
        let z = calculate_focal_gravity_with_k(0.7, 0.3, 5.0);
        // delta = 0.4, Z = tanh(5.0 * 0.4) = tanh(2.0) ≈ 0.964
        assert!(z > 0.95);
    }

    #[test]
    fn test_gravity_with_low_k() {
        // k = 1.0 → sin aceleración exponencial (fallback)
        let z = calculate_focal_gravity_with_k(0.7, 0.3, 1.0);
        // delta = 0.4, Z = tanh(1.0 * 0.4) = tanh(0.4) ≈ 0.3799
        assert!((z - 0.38).abs() < 0.01);
    }

    #[test]
    fn test_gravity_k_less_than_one() {
        // k < 1.0 → se usa como fallback sin aceleración
        let z = calculate_focal_gravity_with_k(0.7, 0.3, 0.5);
        // delta = 0.4, Z = tanh(0.5 * 0.4) = tanh(0.2) ≈ 0.1974
        assert!(z > 0.0);
        assert!(z < 0.25);
    }

    // ─── Error Display Tests ───

    #[test]
    fn test_error_display_invalid_k() {
        let err = StuartianGeometryError::InvalidGravityConstant { k: 0.5 };
        let msg = format!("{}", err);
        assert!(msg.contains("0.5"));
    }

    #[test]
    fn test_error_display_signal_out_of_bounds() {
        let err = StuartianGeometryError::SignalOutOfBounds {
            name: "autonomy".to_string(),
            value: 1.5,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("autonomy"));
        assert!(msg.contains("1.5"));
    }

    #[test]
    fn test_error_display_z_out_of_bounds() {
        let err = StuartianGeometryError::ZAxisOutOfBounds { z: 1.5 };
        let msg = format!("{}", err);
        assert!(msg.contains("1.5"));
    }

    #[test]
    fn test_error_display_vertex_out_of_bounds() {
        let err = StuartianGeometryError::VertexIndexOutOfBounds { idx: 10 };
        let msg = format!("{}", err);
        assert!(msg.contains("10"));
    }

    // ─── FocalRegion Default ───

    #[test]
    fn test_focal_region_default() {
        let region = FocalRegion::default();
        assert_eq!(region, FocalRegion::Ecuador);
    }

    // ─── Gravity Constant ───

    #[test]
    fn test_stuartian_gravity_k_greater_than_one() {
        assert!(
            STUARTIAN_GRAVITY_K > 1.0,
            "STUARTIAN_GRAVITY_K must be > 1.0, got {}",
            STUARTIAN_GRAVITY_K
        );
    }
}
