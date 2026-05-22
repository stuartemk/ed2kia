//! BFT Aggregator — Agregación tolerante a fallas bizantinas.
//!
//! Coordinate-wise Median + Multi-Krum sobre gradientes QLoRA.
//! Rechazo automático de outliers > umbral configurable (anti-backdoor sutil).
//!
//! Ley 2 (Reconocimiento del Error): mediana robusta + Krum scoring.
//! Ley 3 (Cero desperdicio): iteración por coordenadas, memoria eficiente.
//!
//! Feature gate: `#[cfg(feature = "v2.1-bft-aggregation")]`

use std::fmt;

// ─── Errors ───

#[derive(Debug, Clone, PartialEq)]
pub enum BftError {
    InsufficientGradients { requested: usize, got: usize },
    DimensionMismatch { expected: usize, got: usize },
    AllRejected,
    InvalidThreshold(f64),
}

impl fmt::Display for BftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BftError::InsufficientGradients { requested, got } => {
                write!(
                    f,
                    "Gradientes insuficientes: solicitado {}, obtenido {}",
                    requested, got
                )
            }
            BftError::DimensionMismatch { expected, got } => {
                write!(f, "Dimensión: esperado {}, obtenido {}", expected, got)
            }
            BftError::AllRejected => write!(f, "Todos los gradientes rechazados como outliers"),
            BftError::InvalidThreshold(t) => {
                write!(f, "Umbral inválido: {} (debe estar en [0, 1])", t)
            }
        }
    }
}

impl std::error::Error for BftError {}

// ─── BftConfig ───

/// Configuración de agregación BFT.
#[derive(Debug, Clone)]
pub struct BftConfig {
    /// Umbral de outlier rejection (desviación en sigma).
    pub outlier_sigma: f64,
    /// Fracción máxima de bizantinos tolerados (f < n/3 para mediana).
    pub max_byzantine_fraction: f64,
    /// Mínimo de gradientes válidos para proceder.
    pub min_valid_gradients: usize,
}

impl BftConfig {
    pub fn new(
        outlier_sigma: f64,
        max_byzantine_fraction: f64,
        min_valid_gradients: usize,
    ) -> Result<Self, BftError> {
        if !(0.0..1.0).contains(&max_byzantine_fraction) {
            return Err(BftError::InvalidThreshold(max_byzantine_fraction));
        }
        Ok(Self {
            outlier_sigma,
            max_byzantine_fraction,
            min_valid_gradients,
        })
    }

    /// Crear config por defecto: 3σ rejection, tolera 1/3 bizantinos.
    pub fn default_config() -> Self {
        Self {
            outlier_sigma: 3.0,
            max_byzantine_fraction: 1.0 / 3.0,
            min_valid_gradients: 3,
        }
    }
}

impl Default for BftConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

// ─── Coordinate-wise Median ───

/// Coordinate-wise Median: para cada dimensión d, calcula la mediana de gradients[i][d].
///
/// Algoritmo streaming por coordenadas para evitar cargar toda la matriz en RAM.
/// Complejidad: O(N * D) tiempo, O(N) memoria por coordenada.
pub fn coordinate_wise_median(gradients: &[Vec<f32>]) -> Result<Vec<f32>, BftError> {
    if gradients.is_empty() {
        return Err(BftError::InsufficientGradients {
            requested: 1,
            got: 0,
        });
    }

    let dim = gradients[0].len();

    // Validar dimensiones
    for grad in gradients.iter() {
        if grad.len() != dim {
            return Err(BftError::DimensionMismatch {
                expected: dim,
                got: grad.len(),
            });
        }
    }

    let mut result = Vec::with_capacity(dim);
    // Buffer reutilizable para mediana por coordenada
    let mut coord_values = Vec::with_capacity(gradients.len());

    for d in 0..dim {
        coord_values.clear();
        for grad in gradients {
            coord_values.push(grad[d]);
        }
        coord_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = median_value(&coord_values);
        result.push(median);
    }

    Ok(result)
}

fn median_value(sorted: &[f32]) -> f32 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

// ─── Multi-Krum ───

/// Multi-Krum: selecciona los `m` mejores gradientes por distancia euclidiana.
///
/// Para cada gradiente i, calcula la suma de las `m` distancias más pequeñas
/// a otros gradientes. Los `m` con menor score son seleccionados.
pub fn multi_krum_select(gradients: &[Vec<f32>], m: usize) -> Result<Vec<usize>, BftError> {
    let n = gradients.len();
    if n < 2 * m + 1 {
        return Err(BftError::InsufficientGradients {
            requested: 2 * m + 1,
            got: n,
        });
    }

    let dim = gradients[0].len();

    // Calcular matriz de distancias
    let mut scores = Vec::with_capacity(n);
    for i in 0..n {
        let mut dists = Vec::with_capacity(n - 1);
        for j in 0..n {
            if i == j {
                continue;
            }
            let d = euclidean_distance(&gradients[i], &gradients[j], dim);
            dists.push(d);
        }
        dists.sort_by(|a, b| a.partial_cmp(b).unwrap());
        // Suma de las m-1 distancias más pequeñas
        let score: f64 = dists.iter().take(m.saturating_sub(1)).sum();
        scores.push((i, score));
    }

    // Ordenar por score (menor = mejor)
    scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    let selected: Vec<usize> = scores.into_iter().take(m).map(|(i, _)| i).collect();
    Ok(selected)
}

fn euclidean_distance(a: &[f32], b: &[f32], dim: usize) -> f64 {
    let mut sum = 0.0f64;
    for d in 0..dim {
        let diff = (a[d] - b[d]) as f64;
        sum += diff * diff;
    }
    sum.sqrt()
}

// ─── Outlier Rejection ───

/// Detectar y rechazar outliers por desviación sigma de la mediana.
pub fn filter_outliers(
    gradients: &[Vec<f32>],
    config: &BftConfig,
) -> Result<Vec<(usize, Vec<f32>)>, BftError> {
    if gradients.is_empty() {
        return Err(BftError::InsufficientGradients {
            requested: 1,
            got: 0,
        });
    }

    let median = coordinate_wise_median(gradients)?;
    let dim = median.len();

    // MAD (Median Absolute Deviation) - robusto a outliers
    let mut mad = vec![0.0f64; dim];
    for d in 0..dim {
        let mut abs_devs: Vec<f64> = gradients
            .iter()
            .map(|g| ((g[d] - median[d]) as f64).abs())
            .collect();
        abs_devs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = abs_devs.len();
        mad[d] = if n.is_multiple_of(2) {
            (abs_devs[n / 2 - 1] + abs_devs[n / 2]) / 2.0
        } else {
            abs_devs[n / 2]
        };
    }

    // Filtrar usando MAD: outlier si |x - median| > sigma * 1.4826 * MAD
    // Factor 1.4826 normaliza MAD para distribuciones normales
    let mad_scale = 1.4826f64;
    let mut valid = Vec::new();
    for (i, grad) in gradients.iter().enumerate() {
        let mut is_outlier = false;
        for d in 0..dim {
            let diff = ((grad[d] - median[d]) as f64).abs();
            let threshold = config.outlier_sigma * mad_scale * mad[d];
            if threshold > 0.0 && diff > threshold {
                is_outlier = true;
                break;
            }
            // Si MAD es 0 pero hay diferencia, es outlier
            if mad[d] == 0.0 && diff > 1e-9 {
                is_outlier = true;
                break;
            }
        }
        if !is_outlier {
            valid.push((i, grad.clone()));
        }
    }

    if valid.is_empty() {
        return Err(BftError::AllRejected);
    }

    if valid.len() < config.min_valid_gradients {
        return Err(BftError::InsufficientGradients {
            requested: config.min_valid_gradients,
            got: valid.len(),
        });
    }

    Ok(valid)
}

// ─── BftAggregator ───

/// Agregador BFT completo: outlier rejection → coordinate-wise median.
#[derive(Debug, Clone)]
pub struct BftAggregator {
    config: BftConfig,
}

impl BftAggregator {
    pub fn new(config: BftConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self {
            config: BftConfig::default_config(),
        }
    }

    /// Pipeline completo: filter outliers → coordinate-wise median.
    pub fn aggregate(&self, gradients: &[Vec<f32>]) -> Result<Vec<f32>, BftError> {
        let valid = filter_outliers(gradients, &self.config)?;
        let valid_grads: Vec<Vec<f32>> = valid.into_iter().map(|(_, g)| g).collect();
        coordinate_wise_median(&valid_grads)
    }
}

impl Default for BftAggregator {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_median_basic() {
        let grads = vec![
            vec![1.0, 2.0, 3.0],
            vec![2.0, 4.0, 6.0],
            vec![3.0, 6.0, 9.0],
        ];
        let median = coordinate_wise_median(&grads).unwrap();
        assert!(approx_eq(median[0], 2.0));
        assert!(approx_eq(median[1], 4.0));
        assert!(approx_eq(median[2], 6.0));
    }

    #[test]
    fn test_median_even_count() {
        let grads = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let median = coordinate_wise_median(&grads).unwrap();
        assert!(approx_eq(median[0], 2.0));
        assert!(approx_eq(median[1], 3.0));
    }

    #[test]
    fn test_median_empty() {
        let result = coordinate_wise_median(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_median_dimension_mismatch() {
        let grads = vec![vec![1.0, 2.0, 3.0], vec![1.0, 2.0]];
        let result = coordinate_wise_median(&grads);
        match result {
            Err(BftError::DimensionMismatch { expected, got }) => {
                assert_eq!(expected, 3);
                assert_eq!(got, 2);
            }
            other => panic!("Expected DimensionMismatch, got {:?}", other),
        }
    }

    #[test]
    fn test_bft_rejects_byzantine() {
        // 10 gradientes legítimos ~[1,2,3], 3 maliciosos ~[100,200,300]
        let mut grads: Vec<Vec<f32>> = (0..10).map(|_| vec![1.0, 2.0, 3.0]).collect();
        grads.push(vec![100.0, 200.0, 300.0]);
        grads.push(vec![100.0, 200.0, 300.0]);
        grads.push(vec![100.0, 200.0, 300.0]);

        let config = BftConfig::default_config();
        let valid = filter_outliers(&grads, &config).unwrap();

        // Los 3 maliciosos deben ser rechazados
        for (_, g) in &valid {
            assert!(approx_eq(g[0], 1.0), "Legitimate gradient filtered out");
        }
        assert_eq!(valid.len(), 10);
    }

    #[test]
    fn test_bft_aggregate_converges_to_truth() {
        // Simular 10 gradientes, 3 maliciosos
        let mut grads: Vec<Vec<f32>> = (0..10).map(|_| vec![1.0, 2.0, 3.0]).collect();
        grads.push(vec![1000.0, 2000.0, 3000.0]);
        grads.push(vec![1000.0, 2000.0, 3000.0]);
        grads.push(vec![1000.0, 2000.0, 3000.0]);

        let aggregator = BftAggregator::with_defaults();
        let result = aggregator.aggregate(&grads).unwrap();

        // Mediana debe converger a [1, 2, 3]
        assert!(approx_eq(result[0], 1.0));
        assert!(approx_eq(result[1], 2.0));
        assert!(approx_eq(result[2], 3.0));
    }

    #[test]
    fn test_multi_krum_select() {
        let grads = vec![
            vec![1.0, 1.0],
            vec![1.1, 1.1],
            vec![1.2, 1.2],
            vec![1.3, 1.3],
            vec![100.0, 100.0], // outlier
        ];
        let selected = multi_krum_select(&grads, 2).unwrap();
        assert_eq!(selected.len(), 2);
        // Los seleccionados deben ser los cercanos (índices 0, 1, 2, 3)
        for idx in &selected {
            assert!(*idx < 4, "Outlier selected by Krum");
        }
    }

    #[test]
    fn test_multi_krum_insufficient() {
        let grads = vec![vec![1.0], vec![2.0]];
        let result = multi_krum_select(&grads, 2);
        match result {
            Err(BftError::InsufficientGradients { requested, got }) => {
                assert_eq!(requested, 5); // 2*m+1
                assert_eq!(got, 2);
            }
            other => panic!("Expected InsufficientGradients, got {:?}", other),
        }
    }

    #[test]
    fn test_filter_all_rejected() {
        // Todos son outliers entre sí
        let grads = vec![vec![0.0, 0.0], vec![1000.0, 1000.0]];
        let config = BftConfig {
            outlier_sigma: 0.5,
            ..BftConfig::default_config()
        };
        let result = filter_outliers(&grads, &config);
        // Con min_valid_gradients=3, falla por insuficientes
        assert!(result.is_err());
    }

    #[test]
    fn test_bft_config_default() {
        let config = BftConfig::default_config();
        assert_eq!(config.outlier_sigma, 3.0);
        assert_eq!(config.min_valid_gradients, 3);
    }

    #[test]
    fn test_bft_config_invalid_fraction() {
        let result = BftConfig::new(3.0, 1.5, 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_display() {
        let err = BftError::AllRejected;
        let msg = format!("{}", err);
        assert!(msg.contains("rechazados"));
    }

    #[test]
    fn test_median_value_single() {
        assert!(approx_eq(median_value(&[5.0]), 5.0));
    }

    #[test]
    fn test_median_value_even() {
        assert!(approx_eq(median_value(&[1.0, 3.0]), 2.0));
    }

    #[test]
    fn test_median_value_odd() {
        assert!(approx_eq(median_value(&[1.0, 2.0, 3.0]), 2.0));
    }

    #[test]
    fn test_euclidean_distance() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        let dist = euclidean_distance(&a, &b, 2);
        assert!((dist - 5.0).abs() < 1e-6);
    }
}
