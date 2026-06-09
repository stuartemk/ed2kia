//! Global Metrics Dashboard — Unified dashboard aggregating simulation, production,
//! and awakening metrics for the Noosfera Global Ecosystem.
//!
//! **Sprint 126:** The Noosfera Awakening & Global Ecosystem Symbiosis.
//! Provides real-time dashboard generation combining planetary simulation results,
//! production telemetry, and awakening adoption metrics into a single cohesive view.

use crate::edge_runtime::ProductionMetrics;
use crate::planetary_sim::{AwakeningMetrics, PlanetarySimResult};

/// Aggregated dashboard data combining all global metrics.
#[derive(Debug, Clone)]
pub struct DashboardData {
    /// Timestamp or simulation step
    pub timestamp: u64,
    /// Network overview section
    pub network: NetworkSection,
    /// Performance section
    pub performance: PerformanceSection,
    /// Awakening / adoption section
    pub awakening: AwakeningSection,
    /// Energy & sustainability section
    pub energy: EnergySection,
    /// Trust & governance section
    pub trust: TrustSection,
    /// Overall health score (0.0 to 1.0)
    pub overall_health: f64,
    /// Production readiness flag
    pub is_production_ready: bool,
    /// Alert messages
    pub alerts: Vec<String>,
}

impl DashboardData {
    /// Create a new dashboard data structure.
    pub fn new(
        timestamp: u64,
        network: NetworkSection,
        performance: PerformanceSection,
        awakening: AwakeningSection,
        energy: EnergySection,
        trust: TrustSection,
        overall_health: f64,
        is_production_ready: bool,
        alerts: Vec<String>,
    ) -> Self {
        Self {
            timestamp,
            network,
            performance,
            awakening,
            energy,
            trust,
            overall_health,
            is_production_ready,
            alerts,
        }
    }

    /// Generate a human-readable dashboard summary.
    pub fn summary(&self) -> String {
        format!(
            "Dashboard[t={}] health={:.1}% ready={} alerts={}\nnetwork={}/{} perf={:.1}ms awakening={:.1}% energy={:.2}mWh trust={:.3}",
            self.timestamp,
            self.overall_health * 100.0,
            if self.is_production_ready { "✓" } else { "✗" },
            self.alerts.len(),
            self.network.active_nodes,
            self.network.total_nodes,
            self.performance.avg_latency_ms,
            self.awakening.adoption_rate * 100.0,
            self.energy.total_energy_mwh,
            self.trust.avg_trust_score,
        )
    }
}

/// Network overview section of the dashboard.
#[derive(Debug, Clone)]
pub struct NetworkSection {
    /// Total nodes in the network
    pub total_nodes: usize,
    /// Currently active nodes
    pub active_nodes: usize,
    /// Nodes that churned
    pub churned_nodes: usize,
    /// Nodes that rejoined
    pub rejoined_nodes: usize,
    /// Network resilience score (0.0 to 1.0)
    pub resilience_score: f64,
}

impl NetworkSection {
    pub fn new(
        total_nodes: usize,
        active_nodes: usize,
        churned_nodes: usize,
        rejoined_nodes: usize,
        resilience_score: f64,
    ) -> Self {
        Self {
            total_nodes,
            active_nodes,
            churned_nodes,
            rejoined_nodes,
            resilience_score,
        }
    }
}

/// Performance section of the dashboard.
#[derive(Debug, Clone)]
pub struct PerformanceSection {
    /// Average inference latency (ms)
    pub avg_latency_ms: f64,
    /// P99 latency (ms)
    pub p99_latency_ms: f64,
    /// Total inferences processed
    pub total_inferences: u64,
    /// Steer success rate (0.0 to 1.0)
    pub steer_success_rate: f64,
    /// Error rate (0.0 to 1.0)
    pub error_rate: f64,
}

impl PerformanceSection {
    pub fn new(
        avg_latency_ms: f64,
        p99_latency_ms: f64,
        total_inferences: u64,
        steer_success_rate: f64,
        error_rate: f64,
    ) -> Self {
        Self {
            avg_latency_ms,
            p99_latency_ms,
            total_inferences,
            steer_success_rate,
            error_rate,
        }
    }
}

/// Awakening / adoption section of the dashboard.
#[derive(Debug, Clone)]
pub struct AwakeningSection {
    /// Current adoption rate (0.0 to 1.0)
    pub adoption_rate: f64,
    /// Awakened node count
    pub awakened_nodes: usize,
    /// Whether tipping point was reached
    pub tipping_point_reached: bool,
    /// Month when tipping point occurred
    pub tipping_point_month: u32,
    /// Network effect multiplier
    pub network_effect_multiplier: f64,
    /// Collective intelligence score
    pub collective_intelligence_score: f64,
    /// Knowledge diffusion rate (nodes/month)
    pub knowledge_diffusion_rate: f64,
}

impl AwakeningSection {
    pub fn new(
        adoption_rate: f64,
        awakened_nodes: usize,
        tipping_point_reached: bool,
        tipping_point_month: u32,
        network_effect_multiplier: f64,
        collective_intelligence_score: f64,
        knowledge_diffusion_rate: f64,
    ) -> Self {
        Self {
            adoption_rate,
            awakened_nodes,
            tipping_point_reached,
            tipping_point_month,
            network_effect_multiplier,
            collective_intelligence_score,
            knowledge_diffusion_rate,
        }
    }
}

/// Energy & sustainability section of the dashboard.
#[derive(Debug, Clone)]
pub struct EnergySection {
    /// Total energy consumed (mWh)
    pub total_energy_mwh: f64,
    /// Energy rate (mWh/s)
    pub energy_rate_mwh_per_s: f64,
    /// Datacenter baseline energy (mWh)
    pub dc_baseline_mwh: f64,
    /// Energy saved vs baseline (mWh)
    pub energy_saved_mwh: f64,
    /// Savings percentage
    pub savings_pct: f64,
}

impl EnergySection {
    pub fn new(
        total_energy_mwh: f64,
        energy_rate_mwh_per_s: f64,
        dc_baseline_mwh: f64,
        energy_saved_mwh: f64,
        savings_pct: f64,
    ) -> Self {
        Self {
            total_energy_mwh,
            energy_rate_mwh_per_s,
            dc_baseline_mwh,
            energy_saved_mwh,
            savings_pct,
        }
    }
}

/// Trust & governance section of the dashboard.
#[derive(Debug, Clone)]
pub struct TrustSection {
    /// Average trust score across network
    pub avg_trust_score: f64,
    /// Average awakened node trust
    pub avg_awakened_trust: f64,
    /// Memory utilization (0.0 to 1.0)
    pub memory_utilization: f64,
    /// CPU utilization (0.0 to 1.0)
    pub cpu_utilization: f64,
    /// Node uptime (seconds)
    pub uptime_seconds: u64,
}

impl TrustSection {
    pub fn new(
        avg_trust_score: f64,
        avg_awakened_trust: f64,
        memory_utilization: f64,
        cpu_utilization: f64,
        uptime_seconds: u64,
    ) -> Self {
        Self {
            avg_trust_score,
            avg_awakened_trust,
            memory_utilization,
            cpu_utilization,
            uptime_seconds,
        }
    }
}

/// Generate a global dashboard from simulation results and production metrics.
///
/// Aggregates planetary simulation data, production telemetry, and awakening
/// adoption metrics into a unified dashboard for monitoring the Noosfera
/// ecosystem.
///
/// # Arguments
/// * `sim_result` — Planetary simulation results
/// * `prod_metrics` — Production telemetry metrics
/// * `awakening` — Optional awakening metrics (if None, computed from sim)
/// * `timestamp` — Dashboard timestamp
///
/// # Returns
/// `DashboardData` with all sections populated
pub fn generate_global_dashboard(
    sim_result: &PlanetarySimResult,
    prod_metrics: &ProductionMetrics,
    awakening: Option<&AwakeningMetrics>,
    timestamp: u64,
) -> DashboardData {
    let mut alerts = Vec::new();

    // Network section
    let network = NetworkSection::new(
        sim_result.total_nodes,
        sim_result.active_nodes,
        sim_result.churned_nodes,
        sim_result.rejoined_nodes,
        sim_result.resilience_score,
    );

    // Performance section
    let performance = PerformanceSection::new(
        prod_metrics.avg_latency_ms,
        prod_metrics.p99_latency_ms,
        prod_metrics.total_inferences,
        sim_result.steer_success_rate,
        prod_metrics.error_rate,
    );

    // Awakening section
    let awakening_section = if let Some(aw) = awakening {
        AwakeningSection::new(
            aw.adoption_rate,
            aw.awakened_nodes,
            aw.tipping_point_reached,
            aw.tipping_point_month,
            aw.network_effect_multiplier,
            aw.collective_intelligence_score,
            aw.knowledge_diffusion_rate,
        )
    } else {
        // Derive from simulation data
        let adoption_rate = sim_result.resilience_score;
        AwakeningSection::new(
            adoption_rate,
            sim_result.active_nodes,
            adoption_rate > 0.5,
            0,
            1.0,
            adoption_rate * sim_result.avg_trust,
            0.0,
        )
    };

    // Energy section
    let total_energy = sim_result.total_energy_mwh;
    let dc_baseline = total_energy * 1.5; // Datacenter baseline estimate
    let energy_saved = (dc_baseline - total_energy).max(0.0);
    let savings_pct = if dc_baseline > 0.0 {
        (energy_saved / dc_baseline) * 100.0
    } else {
        0.0
    };

    let energy = EnergySection::new(
        total_energy,
        prod_metrics.energy_rate_mwh_per_s,
        dc_baseline,
        energy_saved,
        savings_pct,
    );

    // Trust section
    let trust = TrustSection::new(
        sim_result.avg_trust,
        awakening_section.adoption_rate,
        prod_metrics.memory_utilization,
        prod_metrics.cpu_utilization,
        prod_metrics.uptime_seconds,
    );

    // Compute overall health score
    let health_components: Vec<f64> = vec![
        sim_result.resilience_score,
        sim_result.steer_success_rate,
        1.0 - prod_metrics.error_rate,
        prod_metrics.trust_score,
        1.0 - prod_metrics.memory_utilization,
        1.0 - prod_metrics.cpu_utilization,
        awakening_section.adoption_rate,
    ];
    let overall_health = health_components.iter().sum::<f64>() / health_components.len() as f64;

    // Check production readiness
    let is_production_ready =
        prod_metrics.is_healthy() && sim_result.resilience_score > 0.5 && overall_health > 0.6;

    // Generate alerts
    if sim_result.resilience_score < 0.5 {
        alerts.push("CRITICAL: Network resilience below 50%".to_string());
    }
    if prod_metrics.error_rate > 0.1 {
        alerts.push("WARNING: Error rate exceeds 10%".to_string());
    }
    if prod_metrics.memory_utilization > 0.9 {
        alerts.push("WARNING: Memory utilization above 90%".to_string());
    }
    if prod_metrics.cpu_utilization > 0.9 {
        alerts.push("WARNING: CPU utilization above 90%".to_string());
    }
    if awakening_section.tipping_point_reached {
        alerts.push("INFO: Noosfera adoption tipping point reached".to_string());
    }
    if sim_result.avg_trust < 0.3 {
        alerts.push("WARNING: Average trust score critically low".to_string());
    }

    DashboardData::new(
        timestamp,
        network,
        performance,
        awakening_section,
        energy,
        trust,
        overall_health,
        is_production_ready,
        alerts,
    )
}

/// Generate a minimal dashboard from simulation results only.
///
/// Convenience wrapper when production metrics are not available.
pub fn generate_dashboard_from_sim(
    sim_result: &PlanetarySimResult,
    timestamp: u64,
) -> DashboardData {
    let default_prod = ProductionMetrics {
        uptime_seconds: sim_result.duration_seconds as u64,
        total_inferences: sim_result.total_steers,
        avg_latency_ms: sim_result.avg_latency_ms,
        energy_rate_mwh_per_s: if sim_result.duration_seconds > 0.0 {
            sim_result.total_energy_mwh / sim_result.duration_seconds
        } else {
            0.0
        },
        trust_score: sim_result.avg_trust,
        p99_latency_ms: sim_result.avg_latency_ms * 1.5,
        error_rate: if sim_result.total_steers + sim_result.total_failures > 0 {
            sim_result.total_failures as f64
                / (sim_result.total_steers + sim_result.total_failures) as f64
        } else {
            0.0
        },
        memory_utilization: 0.5,
        cpu_utilization: 0.5,
    };
    generate_global_dashboard(sim_result, &default_prod, None, timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_section_new() {
        let section = NetworkSection::new(1000, 800, 200, 50, 0.8);
        assert_eq!(section.total_nodes, 1000);
        assert_eq!(section.active_nodes, 800);
        assert_eq!(section.churned_nodes, 200);
        assert_eq!(section.rejoined_nodes, 50);
        assert_eq!(section.resilience_score, 0.8);
    }

    #[test]
    fn test_performance_section_new() {
        let section = PerformanceSection::new(50.0, 150.0, 10000, 0.95, 0.05);
        assert_eq!(section.avg_latency_ms, 50.0);
        assert_eq!(section.p99_latency_ms, 150.0);
        assert_eq!(section.total_inferences, 10000);
        assert_eq!(section.steer_success_rate, 0.95);
        assert_eq!(section.error_rate, 0.05);
    }

    #[test]
    fn test_awakening_section_new() {
        let section = AwakeningSection::new(0.6, 6000, true, 8, 3.0, 0.9, 500.0);
        assert_eq!(section.adoption_rate, 0.6);
        assert_eq!(section.awakened_nodes, 6000);
        assert!(section.tipping_point_reached);
        assert_eq!(section.tipping_point_month, 8);
        assert_eq!(section.network_effect_multiplier, 3.0);
        assert_eq!(section.collective_intelligence_score, 0.9);
        assert_eq!(section.knowledge_diffusion_rate, 500.0);
    }

    #[test]
    fn test_energy_section_new() {
        let section = EnergySection::new(100.0, 0.05, 150.0, 50.0, 33.3);
        assert_eq!(section.total_energy_mwh, 100.0);
        assert_eq!(section.energy_rate_mwh_per_s, 0.05);
        assert_eq!(section.dc_baseline_mwh, 150.0);
        assert_eq!(section.energy_saved_mwh, 50.0);
        assert_eq!(section.savings_pct, 33.3);
    }

    #[test]
    fn test_trust_section_new() {
        let section = TrustSection::new(0.75, 0.8, 0.6, 0.4, 3600);
        assert_eq!(section.avg_trust_score, 0.75);
        assert_eq!(section.avg_awakened_trust, 0.8);
        assert_eq!(section.memory_utilization, 0.6);
        assert_eq!(section.cpu_utilization, 0.4);
        assert_eq!(section.uptime_seconds, 3600);
    }

    #[test]
    fn test_dashboard_data_new() {
        let network = NetworkSection::new(100, 80, 20, 5, 0.8);
        let performance = PerformanceSection::new(50.0, 150.0, 1000, 0.9, 0.1);
        let awakening = AwakeningSection::new(0.6, 60, true, 5, 2.0, 0.7, 10.0);
        let energy = EnergySection::new(10.0, 0.01, 15.0, 5.0, 33.3);
        let trust = TrustSection::new(0.7, 0.8, 0.5, 0.5, 3600);

        let dashboard = DashboardData::new(
            1000,
            network,
            performance,
            awakening,
            energy,
            trust,
            0.85,
            true,
            vec!["Alert".to_string()],
        );

        assert_eq!(dashboard.timestamp, 1000);
        assert_eq!(dashboard.overall_health, 0.85);
        assert!(dashboard.is_production_ready);
        assert_eq!(dashboard.alerts.len(), 1);
    }

    #[test]
    fn test_dashboard_summary() {
        let network = NetworkSection::new(1000, 800, 200, 50, 0.8);
        let performance = PerformanceSection::new(50.0, 150.0, 10000, 0.95, 0.05);
        let awakening = AwakeningSection::new(0.6, 600, true, 8, 3.0, 0.9, 50.0);
        let energy = EnergySection::new(100.0, 0.05, 150.0, 50.0, 33.3);
        let trust = TrustSection::new(0.75, 0.8, 0.6, 0.4, 3600);

        let dashboard = DashboardData::new(
            100,
            network,
            performance,
            awakening,
            energy,
            trust,
            0.8,
            true,
            vec![],
        );

        let summary = dashboard.summary();
        assert!(summary.contains("Dashboard"));
        assert!(summary.contains("800"));
        assert!(summary.contains("1000"));
        assert!(summary.contains("✓"));
    }

    #[test]
    fn test_generate_global_dashboard_basic() {
        let sim = PlanetarySimResult {
            total_nodes: 1000,
            active_nodes: 800,
            churned_nodes: 200,
            rejoined_nodes: 50,
            total_steers: 10000,
            total_failures: 1000,
            avg_trust: 0.75,
            total_energy_mwh: 500.0,
            avg_latency_ms: 80.0,
            duration_seconds: 3600.0,
            steps: 60,
            steer_success_rate: 0.91,
            resilience_score: 0.8,
        };

        let prod = ProductionMetrics {
            uptime_seconds: 3600,
            total_inferences: 10000,
            avg_latency_ms: 80.0,
            energy_rate_mwh_per_s: 0.14,
            trust_score: 0.75,
            p99_latency_ms: 120.0,
            error_rate: 0.05,
            memory_utilization: 0.6,
            cpu_utilization: 0.5,
        };

        let dashboard = generate_global_dashboard(&sim, &prod, None, 1000);

        assert_eq!(dashboard.timestamp, 1000);
        assert_eq!(dashboard.network.total_nodes, 1000);
        assert_eq!(dashboard.network.active_nodes, 800);
        assert_eq!(dashboard.performance.total_inferences, 10000);
        assert!(dashboard.overall_health > 0.0);
        assert!(dashboard.overall_health <= 1.0);
    }

    #[test]
    fn test_generate_global_dashboard_with_awakening() {
        let sim = PlanetarySimResult::default();
        let prod = ProductionMetrics::default();

        let awakening =
            AwakeningMetrics::new(10000, 6000, 0.6, 8, true, 3.0, 0.8, 0.9, 500.0, 24, vec![]);

        let dashboard = generate_global_dashboard(&sim, &prod, Some(&awakening), 2000);

        assert_eq!(dashboard.awakening.adoption_rate, 0.6);
        assert_eq!(dashboard.awakening.awakened_nodes, 6000);
        assert!(dashboard.awakening.tipping_point_reached);
        assert_eq!(dashboard.awakening.tipping_point_month, 8);
    }

    #[test]
    fn test_generate_global_dashboard_alerts_low_resilience() {
        let sim = PlanetarySimResult {
            resilience_score: 0.3,
            ..PlanetarySimResult::default()
        };
        let prod = ProductionMetrics::default();

        let dashboard = generate_global_dashboard(&sim, &prod, None, 0);
        let has_critical = dashboard.alerts.iter().any(|a| a.contains("CRITICAL"));
        assert!(has_critical);
    }

    #[test]
    fn test_generate_global_dashboard_alerts_high_error_rate() {
        let sim = PlanetarySimResult::default();
        let prod = ProductionMetrics {
            error_rate: 0.15,
            ..ProductionMetrics::default()
        };

        let dashboard = generate_global_dashboard(&sim, &prod, None, 0);
        let has_error_alert = dashboard.alerts.iter().any(|a| a.contains("Error rate"));
        assert!(has_error_alert);
    }

    #[test]
    fn test_generate_global_dashboard_alerts_high_memory() {
        let sim = PlanetarySimResult::default();
        let prod = ProductionMetrics {
            memory_utilization: 0.95,
            ..ProductionMetrics::default()
        };

        let dashboard = generate_global_dashboard(&sim, &prod, None, 0);
        let has_memory_alert = dashboard.alerts.iter().any(|a| a.contains("Memory"));
        assert!(has_memory_alert);
    }

    #[test]
    fn test_generate_global_dashboard_alerts_high_cpu() {
        let sim = PlanetarySimResult::default();
        let prod = ProductionMetrics {
            cpu_utilization: 0.95,
            ..ProductionMetrics::default()
        };

        let dashboard = generate_global_dashboard(&sim, &prod, None, 0);
        let has_cpu_alert = dashboard.alerts.iter().any(|a| a.contains("CPU"));
        assert!(has_cpu_alert);
    }

    #[test]
    fn test_generate_global_dashboard_alerts_tipping_point() {
        let sim = PlanetarySimResult::default();
        let prod = ProductionMetrics::default();

        let awakening =
            AwakeningMetrics::new(1000, 600, 0.6, 5, true, 2.0, 0.7, 0.8, 50.0, 12, vec![]);

        let dashboard = generate_global_dashboard(&sim, &prod, Some(&awakening), 0);
        let has_tipping_alert = dashboard.alerts.iter().any(|a| a.contains("tipping point"));
        assert!(has_tipping_alert);
    }

    #[test]
    fn test_generate_global_dashboard_alerts_low_trust() {
        let sim = PlanetarySimResult {
            avg_trust: 0.2,
            ..PlanetarySimResult::default()
        };
        let prod = ProductionMetrics::default();

        let dashboard = generate_global_dashboard(&sim, &prod, None, 0);
        let has_trust_alert = dashboard.alerts.iter().any(|a| a.contains("trust"));
        assert!(has_trust_alert);
    }

    #[test]
    fn test_generate_global_dashboard_no_alerts_healthy() {
        let sim = PlanetarySimResult {
            total_nodes: 1000,
            active_nodes: 900,
            churned_nodes: 100,
            rejoined_nodes: 20,
            total_steers: 10000,
            total_failures: 500,
            avg_trust: 0.8,
            total_energy_mwh: 500.0,
            avg_latency_ms: 50.0,
            duration_seconds: 3600.0,
            steps: 60,
            steer_success_rate: 0.95,
            resilience_score: 0.9,
        };

        let prod = ProductionMetrics {
            uptime_seconds: 3600,
            total_inferences: 10000,
            avg_latency_ms: 50.0,
            energy_rate_mwh_per_s: 0.14,
            trust_score: 0.8,
            p99_latency_ms: 75.0,
            error_rate: 0.02,
            memory_utilization: 0.5,
            cpu_utilization: 0.4,
        };

        let dashboard = generate_global_dashboard(&sim, &prod, None, 100);

        // Should have no critical or warning alerts
        let has_warning = dashboard
            .alerts
            .iter()
            .any(|a| a.contains("WARNING") || a.contains("CRITICAL"));
        assert!(!has_warning);
        assert!(dashboard.is_production_ready);
    }

    #[test]
    fn test_generate_dashboard_from_sim() {
        let sim = PlanetarySimResult {
            total_nodes: 500,
            active_nodes: 400,
            churned_nodes: 100,
            rejoined_nodes: 10,
            total_steers: 5000,
            total_failures: 500,
            avg_trust: 0.7,
            total_energy_mwh: 200.0,
            avg_latency_ms: 60.0,
            duration_seconds: 1800.0,
            steps: 30,
            steer_success_rate: 0.91,
            resilience_score: 0.8,
        };

        let dashboard = generate_dashboard_from_sim(&sim, 500);

        assert_eq!(dashboard.timestamp, 500);
        assert_eq!(dashboard.network.total_nodes, 500);
        assert_eq!(dashboard.network.active_nodes, 400);
        assert!(dashboard.overall_health > 0.0);
        assert!(dashboard.overall_health <= 1.0);
    }

    #[test]
    fn test_dashboard_health_score_bounded() {
        let sim = PlanetarySimResult::default();
        let prod = ProductionMetrics::default();

        let dashboard = generate_global_dashboard(&sim, &prod, None, 0);
        assert!(dashboard.overall_health >= 0.0);
        assert!(dashboard.overall_health <= 1.0);
    }

    #[test]
    fn test_dashboard_production_readiness_unhealthy() {
        let sim = PlanetarySimResult {
            resilience_score: 0.3,
            ..PlanetarySimResult::default()
        };
        let prod = ProductionMetrics {
            error_rate: 0.5,
            trust_score: 0.1,
            memory_utilization: 0.95,
            cpu_utilization: 0.95,
            ..ProductionMetrics::default()
        };

        let dashboard = generate_global_dashboard(&sim, &prod, None, 0);
        assert!(!dashboard.is_production_ready);
    }

    #[test]
    fn test_full_dashboard_pipeline() {
        // Full pipeline: planetary sim → awakening → dashboard
        use crate::planetary_sim::{simulate_noosfera_awakening, simulate_planetary_mesh};

        let sim = simulate_planetary_mesh(1000, 0.05, 3600.0, None);
        let awakening = simulate_noosfera_awakening(sim.total_nodes, 24);

        let prod = ProductionMetrics {
            uptime_seconds: sim.duration_seconds as u64,
            total_inferences: sim.total_steers,
            avg_latency_ms: sim.avg_latency_ms,
            energy_rate_mwh_per_s: sim.total_energy_mwh / sim.duration_seconds,
            trust_score: sim.avg_trust,
            p99_latency_ms: sim.avg_latency_ms * 1.5,
            error_rate: 0.05,
            memory_utilization: 0.6,
            cpu_utilization: 0.5,
        };

        let dashboard = generate_global_dashboard(&sim, &prod, Some(&awakening), 9999);

        assert_eq!(dashboard.timestamp, 9999);
        assert_eq!(dashboard.network.total_nodes, sim.total_nodes);
        assert_eq!(dashboard.awakening.adoption_rate, awakening.adoption_rate);
        assert!(dashboard.overall_health > 0.0);

        let summary = dashboard.summary();
        assert!(summary.contains("Dashboard"));
        assert!(summary.contains("9999"));
    }
}
