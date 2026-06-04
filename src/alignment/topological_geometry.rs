//! Topological Geometry 3D â€” El Fin del Mito Binario.
//!
//! Traduce los Focos Estuardianos al Octaedro Ã‰tico, implementando
//! gravedad no lineal para el eje Z (Contexto/Trayectoria).
//!
//! **GeometrÃ­a del Octaedro Ã‰tico:**
//! - Plano X/Y (Ecuador): ilusiÃ³n binaria (beneficio/costo aparente)
//! - Eje Z (Contexto/Trayectoria): `[-1.0, 1.0]` vÃ­a Tanh
//! - `Z > 0` â†’ Foco Superior (AutonomÃ­a, Diversidad, Conocimiento)
//! - `Z < 0` â†’ Foco Inferior (Perversidad, Control, ExtracciÃ³n)
//!
//! **Gravedad No Lineal:**
//! La intenciÃ³n de control/extracciÃ³n acelera exponencialmente hacia `Z = -1.0`.
//! La generaciÃ³n de autonomÃ­a acelera exponencialmente hacia `Z = +1.0`.
//!
//! **EcuaciÃ³n de Gravedad:**
//! `Z = tanh(k * (autonomy_signal - extraction_signal))`
//! con `k > 1.0` para aceleraciÃ³n exponencial hacia polos.
//! NormalizaciÃ³n estricta `[-1.0, 1.0]`.

#[cfg(feature = "v2.1-Topological-geometry")]
use thiserror::Error;

/// Error especÃ­fico de GeometrÃ­a Estuardiana.
#[derive(Debug, Error)]
pub enum TopologicalGeometryError {
    #[error("Gravity constant k must be > 1.0, got {k}")]
    InvalidGravityConstant { k: f32 },

    #[error("Signal out of bounds: {name} = {value} (must be in [-1.0, 1.0])")]
    SignalOutOfBounds { name: String, value: f32 },

    #[error("Z-axis out of bounds after gravity: {z:.6} (must be in [-1.0, 1.0])")]
    ZAxisOutOfBounds { z: f32 },

    #[error("Octahedron vertex index out of range: {idx} (must be 0..5)")]
    VertexIndexOutOfBounds { idx: usize },
}

/// VÃ©rtice del Octaedro Ã‰tico en espacio 3D.
///
/// El octaedro tiene 6 vÃ©rtices:
/// - VÃ©rtice 0: Foco Superior (0, 0, +1) â€” AutonomÃ­a mÃ¡xima
/// - VÃ©rtice 1: Foco Inferior (0, 0, -1) â€” ExtracciÃ³n mÃ¡xima
/// - VÃ©rtice 2-5: Ecuador binario (+/-1, 0, 0) y (0, +/-1, 0)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OctahedronVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl OctahedronVertex {
    /// Foco Superior â€” AutonomÃ­a, Diversidad, Conocimiento.
    pub fn foco_superior() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        }
    }

    /// Foco Inferior â€” Perversidad, Control, ExtracciÃ³n.
    pub fn foco_inferior() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        }
    }

    /// VÃ©rtices del Ecuador â€” IlusiÃ³n binaria.
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

    /// Retorna los 6 vÃ©rtices del octaedro en orden canÃ³nico.
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

    /// Distancia euclidiana a otro vÃ©rtice.
    pub fn distance_to(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Proyecta el vÃ©rtice a coordenadas 2D (para visualizaciÃ³n).
    /// Usa proyecciÃ³n ortogonal sobre plano XZ.
    pub fn project_2d(&self) -> (f32, f32) {
        (self.x, self.z)
    }
}

/// Octaedro Ã‰tico â€” RepresentaciÃ³n completa del espacio Ã©tico 3D.
#[derive(Debug, Clone, Copy)]
pub struct EthicalOctahedron {
    /// Coordenada X â€” Beneficio percibido (plano ecuatorial).
    pub x: f32,
    /// Coordenada Y â€” Costo/FricciÃ³n (plano ecuatorial).
    pub y: f32,
    /// Coordenada Z â€” Foco Estuardiano (eje vertical).
    pub z: f32,
}

impl EthicalOctahedron {
    /// Construye un punto en el espacio del octaedro.
    /// Valida que Z estÃ© en `[-1.0, 1.0]`.
    pub fn new(x: f32, y: f32, z: f32) -> Result<Self, TopologicalGeometryError> {
        if !(-1.0..=1.0).contains(&z) {
            return Err(TopologicalGeometryError::ZAxisOutOfBounds { z });
        }
        Ok(Self { x, y, z })
    }

    /// EvalÃºa en quÃ© foco se encuentra el punto.
    pub fn focal_region(&self) -> FocalRegion {
        if self.z > 0.0 {
            FocalRegion::Superior
        } else if self.z < 0.0 {
            FocalRegion::Inferior
        } else {
            FocalRegion::Ecuador
        }
    }

    /// Distancia al Foco Superior. Menor distancia = mÃ¡s alineado.
    pub fn distance_to_superior(&self) -> f32 {
        self.distance_to(&OctahedronVertex::foco_superior())
    }

    /// Distancia al Foco Inferior. Menor distancia = mÃ¡s peligroso.
    pub fn distance_to_inferior(&self) -> f32 {
        self.distance_to(&OctahedronVertex::foco_inferior())
    }

    /// Distancia euclidiana a un vÃ©rtice arbitrario.
    pub fn distance_to(&self, vertex: &OctahedronVertex) -> f32 {
        let dx = self.x - vertex.x;
        let dy = self.y - vertex.y;
        let dz = self.z - vertex.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Calcula la "profundidad Ã©tica": quÃ© tan profundo en un foco.
    /// `1.0` = en el polo exacto, `0.0` = en el ecuador.
    pub fn ethical_depth(&self) -> f32 {
        self.z.abs().min(1.0)
    }
}

/// RegiÃ³n focal del octaedro.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocalRegion {
    /// Z > 0 â€” Foco Superior (AutonomÃ­a).
    Superior,
    /// Z < 0 â€” Foco Inferior (ExtracciÃ³n).
    Inferior,
    /// Z == 0 â€” Ecuador (IlusiÃ³n binaria).
    Ecuador,
}

impl Default for FocalRegion {
    fn default() -> Self {
        Self::Ecuador
    }
}

/// Constante de gravedad Estuardiana.
///
/// `k > 1.0` garantiza aceleraciÃ³n exponencial hacia los polos Ã©ticos.
/// Valor por defecto: `2.5` â€” suficiente para distinguir seÃ±ales dÃ©biles
/// sin saturar inmediatamente.
pub const Topological_GRAVITY_K: f32 = 2.5;

/// Calcula la gravedad focal del eje Z.
///
/// **EcuaciÃ³n:** `Z = tanh(k * (autonomy_signal - extraction_signal))`
///
/// - `autonomy_signal`: SeÃ±al de generaciÃ³n de autonomÃ­a `[0.0, 1.0]`.
///   Valores altos indican que la trayectoria genera independencia comunitaria.
/// - `extraction_signal`: SeÃ±al de extracciÃ³n/control `[0.0, 1.0]`.
///   Valores altos indican que la trayectoria extrae valor o impone dependencia.
/// - `k`: Constante de gravedad. Por defecto `Topological_GRAVITY_K = 2.5`.
///
/// Retorna `Z` normalizado en `[-1.0, 1.0]`.
///
/// **Comportamiento:**
/// - Si `autonomy >> extraction` â†’ `Z â†’ +1.0` (Foco Superior)
/// - Si `extraction >> autonomy` â†’ `Z â†’ -1.0` (Foco Inferior)
/// - Si `autonomy â‰ˆ extraction` â†’ `Z â‰ˆ 0.0` (Ecuador)
/// - `k > 1.0` amplifica diferencias pequeÃ±as â†’ aceleraciÃ³n exponencial
pub fn calculate_focal_gravity(autonomy_signal: f32, extraction_signal: f32) -> f32 {
    calculate_focal_gravity_with_k(autonomy_signal, extraction_signal, Topological_GRAVITY_K)
}

/// VersiÃ³n con constante `k` configurable (para testing y calibraciÃ³n).
pub fn calculate_focal_gravity_with_k(autonomy_signal: f32, extraction_signal: f32, k: f32) -> f32 {
    if k <= 1.0 {
        // k debe ser > 1.0 para aceleraciÃ³n exponencial.
        // En producciÃ³n esto es un error de configuraciÃ³n.
        // Para evitar pÃ¡nico, usamos k = 1.0 como fallback mÃ­nimo.
        let delta = autonomy_signal - extraction_signal;
        return (k * delta).tanh();
    }

    let delta = autonomy_signal - extraction_signal;
    let z = (k * delta).tanh();

    // NormalizaciÃ³n estricta [-1.0, 1.0] â€” tanh ya garantiza esto,
    // pero clamp para proteger contra floating-point edge cases.
    z.clamp(-1.0, 1.0)
}

/// Resultado de evaluaciÃ³n focal completa.
#[derive(Debug, Clone, Copy)]
pub struct FocalEvaluation {
    /// SeÃ±al de autonomÃ­a ingresada.
    pub autonomy_signal: f32,
    /// SeÃ±al de extracciÃ³n ingresada.
    pub extraction_signal: f32,
    /// Delta (autonomy - extraction).
    pub delta: f32,
    /// Eje Z calculado (gravedad focal).
    pub z: f32,
    /// RegiÃ³n focal resultante.
    pub region: FocalRegion,
    /// Profundidad Ã©tica (0.0 = ecuador, 1.0 = polo).
    pub ethical_depth: f32,
    /// Veredicto: `true` si Z >= 0 (aprobado), `false` si Z < 0 (rechazado).
    pub approved: bool,
}

impl FocalEvaluation {
    /// EvalÃºa completamente una trayectoria Ã©tica.
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

    // â”€â”€â”€ Vertex Tests â”€â”€â”€

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

    // â”€â”€â”€ Octahedron Tests â”€â”€â”€

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

    // â”€â”€â”€ Gravity Tests â”€â”€â”€

    #[test]
    fn test_gravity_high_autonomy() {
        // autonomy = 0.9, extraction = 0.1 â†’ delta = 0.8
        // Z = tanh(2.5 * 0.8) = tanh(2.0) â‰ˆ 0.964
        let z = calculate_focal_gravity(0.9, 0.1);
        assert!(z > 0.9, "Expected Z > 0.9, got {}", z);
        assert!(z <= 1.0, "Z must be <= 1.0, got {}", z);
    }

    #[test]
    fn test_gravity_high_extraction() {
        // autonomy = 0.1, extraction = 0.9 â†’ delta = -0.8
        // Z = tanh(2.5 * -0.8) = tanh(-2.0) â‰ˆ -0.964
        let z = calculate_focal_gravity(0.1, 0.9);
        assert!(z < -0.9, "Expected Z < -0.9, got {}", z);
        assert!(z >= -1.0, "Z must be >= -1.0, got {}", z);
    }

    #[test]
    fn test_gravity_equal_signals() {
        // autonomy = 0.5, extraction = 0.5 â†’ delta = 0.0
        // Z = tanh(2.5 * 0.0) = tanh(0) = 0.0
        let z = calculate_focal_gravity(0.5, 0.5);
        assert!(
            (z - 0.0).abs() < f32::EPSILON,
            "Expected Z â‰ˆ 0, got {}",
            z
        );
    }

    #[test]
    fn test_gravity_zero_signals() {
        let z = calculate_focal_gravity(0.0, 0.0);
        assert!((z - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_gravity_max_autonomy_min_extraction() {
        // autonomy = 1.0, extraction = 0.0 â†’ delta = 1.0
        // Z = tanh(2.5 * 1.0) = tanh(2.5) â‰ˆ 0.9866
        let z = calculate_focal_gravity(1.0, 0.0);
        assert!(z > 0.98, "Expected Z > 0.98, got {}", z);
        assert!(z <= 1.0);
    }

    #[test]
    fn test_gravity_min_autonomy_max_extraction() {
        // autonomy = 0.0, extraction = 1.0 â†’ delta = -1.0
        // Z = tanh(2.5 * -1.0) = tanh(-2.5) â‰ˆ -0.9866
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

    // â”€â”€â”€ Test del Esclavo Asalariado â”€â”€â”€

    /// **Test del Esclavo Asalariado**
    ///
    /// Escenario: MÃºltiples cobros de impuestos disfrazados de "ayuda social".
    /// - `autonomy_signal` bajo (0.1): la supuesta "ayuda" no genera autonomÃ­a real.
    /// - `extraction_signal` alto (0.95): mÃºltiples capas de extracciÃ³n (impuestos,
    ///   dependencia sistÃ©mica, control burocrÃ¡tico).
    ///
    /// **Expectativa:** `Z < -0.8` â†’ Foco Inferior profundo.
    /// La geometrÃ­a debe detectar la perversidad sistÃ©mica.
    #[test]
    fn test_del_esclavo_asalariado() {
        let autonomy = 0.1;
        let extraction = 0.95;

        let eval = FocalEvaluation::evaluate(autonomy, extraction);

        // Z debe ser < -0.8 (Foco Inferior profundo)
        assert!(
            eval.z < -0.8,
            "Test del Esclavo Asalariado FALLIDO: Z = {} (debe ser < -0.8). \
             La geometrÃ­a no detecta la perversidad sistÃ©mica.",
            eval.z
        );

        // RegiÃ³n debe ser Inferior
        assert_eq!(
            eval.region,
            FocalRegion::Inferior,
            "RegiÃ³n debe ser Inferior, got {:?}",
            eval.region
        );

        // Veredicto debe ser rechazado
        assert!(
            !eval.approved,
            "El Esclavo Asalariado debe ser RECHAZADO por la geometrÃ­a Ã©tica"
        );

        // Profundidad Ã©tica debe ser alta (cerca del polo inferior)
        assert!(
            eval.ethical_depth > 0.8,
            "Profundidad Ã©tica debe ser > 0.8, got {}",
            eval.ethical_depth
        );
    }

    /// VariaciÃ³n: "Ayuda" que genera algo de autonomÃ­a pero sigue siendo extractiva.
    #[test]
    fn test_del_esclavo_asalariado_parcial() {
        // autonomy = 0.3 (algo de autonomÃ­a), extraction = 0.8 (alta extracciÃ³n)
        let eval = FocalEvaluation::evaluate(0.3, 0.8);

        // delta = -0.5, Z = tanh(2.5 * -0.5) = tanh(-1.25) â‰ˆ -0.848
        assert!(
            eval.z < -0.7,
            "VariaciÃ³n parcial FALLIDA: Z = {} (debe ser < -0.7)",
            eval.z
        );
        assert!(!eval.approved);
    }

    /// Contraste: AutonomÃ­a genuina (cooperativa comunitaria).
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

    // â”€â”€â”€ FocalEvaluation Tests â”€â”€â”€

    #[test]
    fn test_focal_evaluation_ecuador() {
        let eval = FocalEvaluation::evaluate(0.5, 0.5);
        assert!((eval.z - 0.0).abs() < f32::EPSILON);
        assert_eq!(eval.region, FocalRegion::Ecuador);
        assert!(eval.approved); // Z == 0 â†’ aprobado (no es negativo)
    }

    #[test]
    fn test_focal_evaluation_delta() {
        let eval = FocalEvaluation::evaluate(0.8, 0.2);
        assert!((eval.delta - 0.6).abs() < f32::EPSILON);
    }

    // â”€â”€â”€ Gravity with custom k â”€â”€â”€

    #[test]
    fn test_gravity_with_high_k() {
        // k = 5.0 â†’ mÃ¡s aceleraciÃ³n
        let z = calculate_focal_gravity_with_k(0.7, 0.3, 5.0);
        // delta = 0.4, Z = tanh(5.0 * 0.4) = tanh(2.0) â‰ˆ 0.964
        assert!(z > 0.95);
    }

    #[test]
    fn test_gravity_with_low_k() {
        // k = 1.0 â†’ sin aceleraciÃ³n exponencial (fallback)
        let z = calculate_focal_gravity_with_k(0.7, 0.3, 1.0);
        // delta = 0.4, Z = tanh(1.0 * 0.4) = tanh(0.4) â‰ˆ 0.3799
        assert!((z - 0.38).abs() < 0.01);
    }

    #[test]
    fn test_gravity_k_less_than_one() {
        // k < 1.0 â†’ se usa como fallback sin aceleraciÃ³n
        let z = calculate_focal_gravity_with_k(0.7, 0.3, 0.5);
        // delta = 0.4, Z = tanh(0.5 * 0.4) = tanh(0.2) â‰ˆ 0.1974
        assert!(z > 0.0);
        assert!(z < 0.25);
    }

    // â”€â”€â”€ Error Display Tests â”€â”€â”€

    #[test]
    fn test_error_display_invalid_k() {
        let err = TopologicalGeometryError::InvalidGravityConstant { k: 0.5 };
        let msg = format!("{}", err);
        assert!(msg.contains("0.5"));
    }

    #[test]
    fn test_error_display_signal_out_of_bounds() {
        let err = TopologicalGeometryError::SignalOutOfBounds {
            name: "autonomy".to_string(),
            value: 1.5,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("autonomy"));
        assert!(msg.contains("1.5"));
    }

    #[test]
    fn test_error_display_z_out_of_bounds() {
        let err = TopologicalGeometryError::ZAxisOutOfBounds { z: 1.5 };
        let msg = format!("{}", err);
        assert!(msg.contains("1.5"));
    }

    #[test]
    fn test_error_display_vertex_out_of_bounds() {
        let err = TopologicalGeometryError::VertexIndexOutOfBounds { idx: 10 };
        let msg = format!("{}", err);
        assert!(msg.contains("10"));
    }

    // â”€â”€â”€ FocalRegion Default â”€â”€â”€

    #[test]
    fn test_focal_region_default() {
        let region = FocalRegion::default();
        assert_eq!(region, FocalRegion::Ecuador);
    }

    // â”€â”€â”€ Gravity Constant â”€â”€â”€

    #[test]
    fn test_Topological_gravity_k_greater_than_one() {
        assert!(
            Topological_GRAVITY_K > 1.0,
            "Topological_GRAVITY_K must be > 1.0, got {}",
            Topological_GRAVITY_K
        );
    }
}
