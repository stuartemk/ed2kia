//! ed2k-consensus — Consensus Mechanisms
//!
//! Proof of Symbiosis (PoSym), Proof of Useful Symbiosis (PoUS),
//! Evolutionary Game Dynamics, and Hierarchical Sharding
//! for planetary-scale distributed consensus.

pub mod emergence;
pub mod eternal_governance;
pub mod governance;
pub mod hierarchical_sharding;
pub mod posym;
pub mod pous;
pub mod replicator;
pub mod value_alignment;

/// Configuración para la Dinámica de Replicador Evolutivo.
pub struct ReplicatorConfig {
    /// Peso de la entropía de diversidad (anti-monopolio).
    pub eta: f64,
    /// Peso de la penalización bizantina.
    pub delta: f64,
    /// Paso de tiempo para integración de Euler.
    pub dt: f64,
    /// Fitness promedio de la red (placeholder para gossip aggregation).
    pub bar_f: f64,
}

impl Default for ReplicatorConfig {
    fn default() -> Self {
        Self {
            eta: 0.1,
            delta: 0.5,
            dt: 0.1,
            bar_f: 0.5,
        }
    }
}

impl ReplicatorConfig {
    /// Crear configuración con fitness promedio personalizado.
    pub fn with_bar_f(mut self, bar_f: f64) -> Self {
        self.bar_f = bar_f;
        self
    }

    /// Crear configuración con peso de diversidad personalizado.
    pub fn with_eta(mut self, eta: f64) -> Self {
        self.eta = eta;
        self
    }

    /// Crear configuración con peso de penalización bizantina personalizado.
    pub fn with_delta(mut self, delta: f64) -> Self {
        self.delta = delta;
        self
    }

    /// Crear configuración con paso de tiempo personalizado.
    pub fn with_dt(mut self, dt: f64) -> Self {
        self.dt = dt;
        self
    }
}

/// Resultado de un paso de dinámica de replicador.
#[derive(Debug, Clone)]
pub struct ReplicatorResult {
    /// Nueva proporción de influencia del nodo.
    pub new_share: f64,
    /// Fitness individual del nodo.
    pub fitness_i: f64,
    /// Derivada dx/dt del paso.
    pub dx_dt: f64,
}

impl std::fmt::Display for ReplicatorResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ReplicatorResult {{ new_share: {:.4}, fitness_i: {:.4}, dx_dt: {:.6} }}",
            self.new_share, self.fitness_i, self.dx_dt
        )
    }
}

/// Núcleo de Dinámica de Replicador Evolutivo (Multi-Objetivo).
///
/// Ecuación:
/// ```text
/// dx_i/dt = x_i * (f_i(x, φ) - f̄ + η * C(x) - δ * B_i)
/// ```
/// donde:
/// - `f_i = TCM_coherence - energy_cost` (Fitness individual)
/// - `f̄` = Fitness promedio de la red
/// - `C(x)` = Entropía de diversidad (anti-monopolio)
/// - `B_i` = Penalización por comportamiento bizantino
pub struct EvolutionaryGameEngine {
    pub config: ReplicatorConfig,
}

impl Default for EvolutionaryGameEngine {
    fn default() -> Self {
        Self {
            config: ReplicatorConfig::default(),
        }
    }
}

impl EvolutionaryGameEngine {
    pub fn new(config: ReplicatorConfig) -> Self {
        Self { config }
    }

    /// Dinámica de Replicador Evolutivo (Multi-Objetivo)
    ///
    /// `dx_i/dt = x_i * (f_i(x, φ) - f̄ + η * C(x) - δ * B_i)`
    ///
    /// Parámetros:
    /// - `current_share`: Proporción de influencia del nodo (x_i)
    /// - `tcm_coherence`: Coherencia topológica aportada
    /// - `energy_cost`: Costo termodinámico
    /// - `diversity_entropy`: C(x) - Entropía de diversidad de la red
    /// - `byzantine_score`: B_i - Penalización por comportamiento malicioso
    pub fn compute_replicator_dynamics(
        &self,
        current_share: f64,
        tcm_coherence: f64,
        energy_cost: f64,
        diversity_entropy: f64,
        byzantine_score: f64,
    ) -> ReplicatorResult {
        // 1. Fitness individual: f_i = Coherencia - Costo de Energía
        let fitness_i = tcm_coherence - energy_cost;

        // 2. Ecuación diferencial del replicador
        let dx_dt = current_share
            * (fitness_i - self.config.bar_f
                + (self.config.eta * diversity_entropy)
                - (self.config.delta * byzantine_score));

        // 3. Actualizar la proporción de influencia (Euler step)
        let new_share = (current_share + dx_dt * self.config.dt).clamp(0.0, 1.0);

        ReplicatorResult {
            new_share,
            fitness_i,
            dx_dt,
        }
    }

    /// Simular múltiples pasos de dinámica de replicador.
    pub fn simulate(
        &self,
        initial_share: f64,
        tcm_coherence: f64,
        energy_cost: f64,
        diversity_entropy: f64,
        byzantine_score: f64,
        steps: usize,
    ) -> Vec<ReplicatorResult> {
        let mut results = Vec::with_capacity(steps);
        let mut share = initial_share;

        for _ in 0..steps {
            let result = self.compute_replicator_dynamics(
                share,
                tcm_coherence,
                energy_cost,
                diversity_entropy,
                byzantine_score,
            );
            share = result.new_share;
            results.push(result);
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replicator_config_default() {
        let cfg = ReplicatorConfig::default();
        assert!((cfg.eta - 0.1).abs() < 1e-9);
        assert!((cfg.delta - 0.5).abs() < 1e-9);
        assert!((cfg.dt - 0.1).abs() < 1e-9);
        assert!((cfg.bar_f - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_replicator_config_builders() {
        let cfg = ReplicatorConfig::default()
            .with_bar_f(0.8)
            .with_eta(0.2)
            .with_delta(1.0);
        assert!((cfg.bar_f - 0.8).abs() < 1e-9);
        assert!((cfg.eta - 0.2).abs() < 1e-9);
        assert!((cfg.delta - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_evolutionary_game_engine_default() {
        let engine = EvolutionaryGameEngine::default();
        assert!((engine.config.eta - 0.1).abs() < 1e-9);
    }

    #[test]
    fn test_evolutionary_game_engine_new() {
        let cfg = ReplicatorConfig::default().with_bar_f(0.7);
        let engine = EvolutionaryGameEngine::new(cfg);
        assert!((engine.config.bar_f - 0.7).abs() < 1e-9);
    }

    #[test]
    fn test_altruistic_node_gains_influence() {
        let engine = EvolutionaryGameEngine::default();
        // Nodo altruista: alta coherencia, bajo costo, sin bizantino
        let result = engine.compute_replicator_dynamics(
            0.5,  // current_share
            1.0,  // tcm_coherence (alta)
            0.1,  // energy_cost (bajo)
            0.5,  // diversity_entropy
            0.0,  // byzantine_score (limpio)
        );
        // fitness_i = 1.0 - 0.1 = 0.9 > bar_f (0.5) → dx_dt > 0 → new_share > current_share
        assert!(result.new_share > 0.5, "Altruista debe ganar influencia");
        assert!(result.fitness_i > 0.0, "Fitness debe ser positivo");
        assert!(result.dx_dt > 0.0, "Derivada debe ser positiva");
    }

    #[test]
    fn test_parasitic_node_loses_influence() {
        let engine = EvolutionaryGameEngine::default();
        // Nodo parásito: baja coherencia, alto costo, alto bizantino
        let result = engine.compute_replicator_dynamics(
            0.5,  // current_share
            0.1,  // tcm_coherence (baja)
            0.9,  // energy_cost (alto)
            0.5,  // diversity_entropy
            1.0,  // byzantine_score (malicioso)
        );
        // fitness_i = 0.1 - 0.9 = -0.8 << bar_f (0.5) → dx_dt << 0 → new_share << current_share
        assert!(result.new_share < 0.5, "Parásito debe perder influencia");
        assert!(result.fitness_i < 0.0, "Fitness debe ser negativo");
        assert!(result.dx_dt < 0.0, "Derivada debe ser negativa");
    }

    #[test]
    fn test_byzantine_penalty_elimination() {
        let engine = EvolutionaryGameEngine::default();
        // Nodo con penalización bizantina extrema
        let results = engine.simulate(
            0.5,  // initial_share
            0.5,  // tcm_coherence
            0.5,  // energy_cost
            0.5,  // diversity_entropy
            2.0,  // byzantine_score (extremo)
            100,  // steps
        );
        let final_share = results.last().unwrap().new_share;
        assert!(
            final_share < 0.01,
            "Nodo bizantino debe ser eliminado (share → 0), got: {:.4}",
            final_share
        );
    }

    #[test]
    fn test_altruistic_dominance() {
        let engine = EvolutionaryGameEngine::default();
        // Nodo altruista perfecto
        let results = engine.simulate(
            0.1,  // initial_share (empieza pequeño)
            1.0,  // tcm_coherence (máxima)
            0.0,  // energy_cost (cero)
            1.0,  // diversity_entropy (máxima)
            0.0,  // byzantine_score (limpio)
            200,  // steps
        );
        let final_share = results.last().unwrap().new_share;
        assert!(
            final_share > 0.9,
            "Altruista debe dominar (share → 1), got: {:.4}",
            final_share
        );
    }

    #[test]
    fn test_share_clamped_to_zero() {
        let engine = EvolutionaryGameEngine::default();
        let result = engine.compute_replicator_dynamics(
            0.0,  // current_share = 0
            0.0,  // tcm_coherence
            1.0,  // energy_cost
            0.0,  // diversity_entropy
            1.0,  // byzantine_score
        );
        assert!(
            (result.new_share - 0.0).abs() < 1e-9,
            "Share en 0 debe permanecer en 0"
        );
    }

    #[test]
    fn test_share_clamped_to_one() {
        let engine = EvolutionaryGameEngine::default();
        // Forzar share > 1 con Euler step grande
        let cfg = ReplicatorConfig::default().with_dt(10.0);
        let engine = EvolutionaryGameEngine::new(cfg);
        let result = engine.compute_replicator_dynamics(
            0.99, // current_share
            10.0, // tcm_coherence (extremo)
            0.0,  // energy_cost
            0.0,  // diversity_entropy
            0.0,  // byzantine_score
        );
        assert!(
            result.new_share <= 1.0,
            "Share no puede exceder 1.0, got: {:.4}",
            result.new_share
        );
    }

    #[test]
    fn test_nash_equilibrium_stability() {
        let engine = EvolutionaryGameEngine::default();
        // En equilibrio: fitness_i ≈ bar_f → dx_dt ≈ 0
        let result = engine.compute_replicator_dynamics(
            0.5,  // current_share
            0.8,  // tcm_coherence
            0.3,  // energy_cost → fitness_i = 0.5 = bar_f
            0.5,  // diversity_entropy → η * C = 0.05
            0.1,  // byzantine_score → δ * B = 0.05 → se cancelan
        );
        // fitness_i - bar_f + η*C - δ*B = 0.5 - 0.5 + 0.05 - 0.05 = 0
        assert!(
            (result.dx_dt).abs() < 1e-9,
            "En equilibrio, dx_dt debe ser ≈ 0, got: {:.6}",
            result.dx_dt
        );
        assert!(
            (result.new_share - 0.5).abs() < 1e-9,
            "Share debe permanecer estable en equilibrio"
        );
    }

    #[test]
    fn test_replicator_result_display() {
        let result = ReplicatorResult {
            new_share: 0.6,
            fitness_i: 0.3,
            dx_dt: 0.01,
        };
        let display = format!("{}", result);
        assert!(display.contains("0.6000"));
        assert!(display.contains("ReplicatorResult"));
    }

    #[test]
    fn test_diversity_entropy_bonus() {
        let engine = EvolutionaryGameEngine::default();
        // Con entropía alta
        let result_high = engine.compute_replicator_dynamics(0.5, 0.5, 0.0, 2.0, 0.0);
        // Sin entropía
        let result_low = engine.compute_replicator_dynamics(0.5, 0.5, 0.0, 0.0, 0.0);
        assert!(
            result_high.new_share > result_low.new_share,
            "Mayor entropía de diversidad debe dar mayor share"
        );
    }

    #[test]
    fn test_simulate_returns_correct_length() {
        let engine = EvolutionaryGameEngine::default();
        let results = engine.simulate(0.5, 0.8, 0.2, 0.5, 0.0, 50);
        assert_eq!(results.len(), 50);
    }

    #[test]
    fn test_full_ess_demonstration() {
        // Demostración completa del Equilibrio de Nash Evolutivamente Estable (ESS)
        let engine = EvolutionaryGameEngine::default();

        // Estrategia Simbiótica (Altruista)
        let symbiotic = engine.simulate(0.5, 1.0, 0.1, 1.0, 0.0, 100);
        let symbiotic_final = symbiotic.last().unwrap().new_share;

        // Estrategia Parásita (Egoísta)
        let parasitic = engine.simulate(0.5, 0.1, 0.9, 0.0, 1.0, 100);
        let parasitic_final = parasitic.last().unwrap().new_share;

        // Estrategia Bizantina (Maliciosa)
        let byzantine = engine.simulate(0.5, 0.3, 0.7, 0.0, 2.0, 100);
        let byzantine_final = byzantine.last().unwrap().new_share;

        println!("=== ESS Demonstration ===");
        println!("Simbiótica (Altruista): {:.4}", symbiotic_final);
        println!("Parásita (Egoísta):     {:.4}", parasitic_final);
        println!("Bizantina (Maliciosa):  {:.4}", byzantine_final);

        // ESS: La estrategia simbiótica domina
        assert!(symbiotic_final > 0.9, "Simbiótica debe dominar");
        assert!(parasitic_final < 0.1, "Parásita debe ser eliminada");
        assert!(byzantine_final < 0.01, "Bizantina debe ser eliminada");
        assert!(
            symbiotic_final > parasitic_final,
            "Simbiótica > Parásita (ESS)"
        );
        assert!(
            symbiotic_final > byzantine_final,
            "Simbiótica > Bizantina (ESS)"
        );
    }
}
