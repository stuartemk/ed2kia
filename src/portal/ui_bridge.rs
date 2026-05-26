//! UI Bridge — CE Wallet + Dashboard Bindings for SymbioticPortal.
//!
//! Provides `UiBridge`, a set of WASM-exposed bindings that connect the
//! SymbioticPortal to Alpine.js/Vanilla.js UI components for:
//!
//! - **CE Wallet**: Balance display, deposit, and transaction history.
//! - **GEI Dashboard**: Real-time ethical state visualization (X, Y, Z axes).
//! - **Resonance Status**: SCT Z-score, brainwave band, and confidence display.
//! - **Health Monitor**: Portal connection status and heartbeat.
//!
//! **Architecture Principles:**
//! - Simbiosis: UI reflects the true state of the symbiotic stack.
//! - Cero fricción: Reactive bindings update automatically on events.
//! - Distribución: Decoupled from rendering — pure data bindings.
//! - Equilibrar: Balanced CE consumption tracking.
//!
//! **Feature Gate:** `v3.7-symbiotic-portal`

use wasm_bindgen::prelude::*;

/// CE Wallet state exposed to the UI.
#[wasm_bindgen]
pub struct CeWallet {
    /// Current CE balance.
    #[wasm_bindgen(readonly)]
    pub balance: f64,
    /// Total CE deposited.
    #[wasm_bindgen(readonly)]
    pub total_deposited: f64,
    /// Total CE consumed.
    #[wasm_bindgen(readonly)]
    pub total_consumed: f64,
    /// Number of transactions.
    #[wasm_bindgen(readonly)]
    pub transaction_count: u32,
}

#[wasm_bindgen]
impl CeWallet {
    /// Create a new empty CE Wallet.
    #[wasm_bindgen(constructor)]
    pub fn new() -> CeWallet {
        CeWallet {
            balance: 0.0,
            total_deposited: 0.0,
            total_consumed: 0.0,
            transaction_count: 0,
        }
    }

    /// Record a CE deposit.
    pub fn deposit(&mut self, amount: f64) {
        if amount > 0.0 {
            self.balance += amount;
            self.total_deposited += amount;
            self.transaction_count += 1;
        }
    }

    /// Record a CE consumption.
    pub fn consume(&mut self, amount: f64) -> bool {
        if amount > 0.0 && amount <= self.balance {
            self.balance -= amount;
            self.total_consumed += amount;
            self.transaction_count += 1;
            true
        } else {
            false
        }
    }

    /// Serialize wallet state to JSON for UI binding.
    #[wasm_bindgen(js_name = "toJson")]
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"balance":{:.4},"totalDeposited":{:.4},"totalConsumed":{:.4},"transactionCount":{}}}"#,
            self.balance, self.total_deposited, self.total_consumed, self.transaction_count
        )
    }

    /// Reset wallet to zero state.
    pub fn reset(&mut self) {
        self.balance = 0.0;
        self.total_deposited = 0.0;
        self.total_consumed = 0.0;
        self.transaction_count = 0;
    }
}

impl Default for CeWallet {
    fn default() -> Self {
        Self::new()
    }
}

/// GEI (Geometric Ethical Invariant) state for dashboard display.
#[wasm_bindgen]
pub struct GeiState {
    /// X-axis: Problem alignment (0.0 to 1.0).
    #[wasm_bindgen(readonly)]
    pub x: f64,
    /// Y-axis: Solution coherence (0.0 to 1.0).
    #[wasm_bindgen(readonly)]
    pub y: f64,
    /// Z-axis: Ethical validation (0.0 to 1.0).
    #[wasm_bindgen(readonly)]
    pub z: f64,
    /// Overall GEI stability score (0.0 to 1.0).
    #[wasm_bindgen(readonly)]
    pub stability: f64,
    /// Whether the GEI is within acceptable bounds (Z >= 0.0).
    #[wasm_bindgen(readonly)]
    pub approved: bool,
}

#[wasm_bindgen]
impl GeiState {
    /// Create a new GEI state.
    #[wasm_bindgen(constructor)]
    pub fn new(x: f64, y: f64, z: f64) -> GeiState {
        let stability = Self::calculate_stability(x, y, z);
        let approved = z >= 0.0;
        GeiState {
            x,
            y,
            z,
            stability,
            approved,
        }
    }

    /// Calculate GEI stability from component axes.
    fn calculate_stability(x: f64, y: f64, z: f64) -> f64 {
        // Stability = normalized distance from origin on octahedron
        let magnitude = (x * x + y * y + z * z).sqrt();
        (magnitude / 1.732).min(1.0).max(0.0) // 1.732 ≈ sqrt(3)
    }

    /// Check if GEI is in harmonic equilibrium (all axes balanced).
    pub fn is_harmonic(&self) -> bool {
        let variance = self.calculate_variance();
        variance < 0.05 // Low variance = balanced
    }

    fn calculate_variance(&self) -> f64 {
        let mean = (self.x + self.y + self.z) / 3.0;
        ((self.x - mean).powi(2) + (self.y - mean).powi(2) + (self.z - mean).powi(2)) / 3.0
    }

    /// Serialize GEI state to JSON for UI binding.
    #[wasm_bindgen(js_name = "toJson")]
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"x":{:.4},"y":{:.4},"z":{:.4},"stability":{:.4},"approved":{}}}"#,
            self.x, self.y, self.z, self.stability, self.approved
        )
    }
}

/// Resonance pipeline status for dashboard display.
#[wasm_bindgen]
pub struct ResonanceStatus {
    /// SCT Z-axis score.
    #[wasm_bindgen(readonly)]
    pub sct_z: f32,
    /// Current brainwave band target.
    #[wasm_bindgen(readonly)]
    pub brainwave_band: String,
    /// Confidence score (0.0 to 1.0).
    #[wasm_bindgen(readonly)]
    pub confidence: f32,
    /// Whether the resonance response is SCT-approved.
    #[wasm_bindgen(readonly)]
    pub approved: bool,
    /// Homeostasis target value.
    #[wasm_bindgen(readonly)]
    pub homeostasis_target: f32,
}

#[wasm_bindgen]
impl ResonanceStatus {
    /// Create a new ResonanceStatus.
    #[wasm_bindgen(constructor)]
    pub fn new(sct_z: f32, brainwave_band: String, confidence: f32, homeostasis_target: f32) -> ResonanceStatus {
        ResonanceStatus {
            approved: sct_z >= 0.0,
            sct_z,
            brainwave_band,
            confidence,
            homeostasis_target,
        }
    }

    /// Get the brainwave band frequency range in Hz.
    #[wasm_bindgen(js_name = "getFrequencyRange")]
    pub fn get_frequency_range(&self) -> (f32, f32) {
        match self.brainwave_band.as_str() {
            "delta" => (0.5, 4.0),
            "theta" => (4.0, 8.0),
            "alpha" => (8.0, 14.0),
            "beta"  => (14.0, 30.0),
            "gamma" => (30.0, 100.0),
            _       => (8.0, 14.0), // Default alpha
        }
    }

    /// Serialize resonance status to JSON for UI binding.
    #[wasm_bindgen(js_name = "toJson")]
    pub fn to_json(&self) -> String {
        let (freq_min, freq_max) = self.get_frequency_range();
        format!(
            r#"{{"sctZ":{:.4},"brainwaveBand":"{}","confidence":{:.4},"approved":{},"homeostasisTarget":{:.4},"frequencyRange":[{:.1},{:.1}]}}"#,
            self.sct_z, self.brainwave_band, self.confidence, self.approved, self.homeostasis_target, freq_min, freq_max
        )
    }
}

/// Portal health monitor for connection status display.
#[wasm_bindgen]
pub struct HealthMonitor {
    /// Current health status string.
    #[wasm_bindgen(readonly)]
    pub status: String,
    /// Last heartbeat timestamp (Unix ms).
    #[wasm_bindgen(readonly)]
    pub last_heartbeat: u64,
    /// Heartbeat interval in milliseconds.
    #[wasm_bindgen(readonly)]
    pub heartbeat_interval_ms: u64,
    /// Number of consecutive missed heartbeats.
    #[wasm_bindgen(readonly)]
    pub missed_heartbeats: u32,
    /// Maximum allowed missed heartbeats before marking as degraded.
    #[wasm_bindgen(readonly)]
    pub max_missed_heartbeats: u32,
}

#[wasm_bindgen]
impl HealthMonitor {
    /// Create a new HealthMonitor.
    #[wasm_bindgen(constructor)]
    pub fn new(heartbeat_interval_ms: u64, max_missed: u32) -> HealthMonitor {
        HealthMonitor {
            status: "idle".to_string(),
            last_heartbeat: 0,
            heartbeat_interval_ms,
            missed_heartbeats: 0,
            max_missed_heartbeats: max_missed,
        }
    }

    /// Record a heartbeat and update status.
    pub fn heartbeat(&mut self, now_ms: u64) {
        self.last_heartbeat = now_ms;
        self.missed_heartbeats = 0;
        self.status = "healthy".to_string();
    }

    /// Check for missed heartbeats and update status if degraded.
    pub fn check(&mut self, now_ms: u64) {
        if self.last_heartbeat == 0 {
            return;
        }

        let elapsed = now_ms.saturating_sub(self.last_heartbeat);
        let missed = elapsed / self.heartbeat_interval_ms;
        self.missed_heartbeats = missed.min(100) as u32;

        if self.missed_heartbeats >= self.max_missed_heartbeats {
            self.status = "degraded".to_string();
        }
        if self.missed_heartbeats >= self.max_missed_heartbeats * 2 {
            self.status = "failed".to_string();
        }
    }

    /// Update the health status manually.
    pub fn set_status(&mut self, status: &str) {
        self.status = status.to_string();
    }

    /// Serialize health monitor state to JSON for UI binding.
    #[wasm_bindgen(js_name = "toJson")]
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"status":"{}","lastHeartbeat":{},"heartbeatIntervalMs":{},"missedHeartbeats":{},"maxMissedHeartbeats":{}}}"#,
            self.status, self.last_heartbeat, self.heartbeat_interval_ms, self.missed_heartbeats, self.max_missed_heartbeats
        )
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new(5000, 3) // 5s interval, 3 max missed
    }
}

/// UiBridge — Central dashboard bindings aggregator.
///
/// Combines CeWallet, GeiState, ResonanceStatus, and HealthMonitor into a
/// single reactive binding object for Alpine.js or vanilla JS integration.
#[wasm_bindgen]
pub struct UiBridge {
    wallet: CeWallet,
    gei: GeiState,
    resonance: ResonanceStatus,
    health: HealthMonitor,
}

#[wasm_bindgen]
impl UiBridge {
    /// Create a new UiBridge with default components.
    #[wasm_bindgen(constructor)]
    pub fn new() -> UiBridge {
        UiBridge {
            wallet: CeWallet::new(),
            gei: GeiState::new(0.0, 0.0, 0.0),
            resonance: ResonanceStatus::new(0.0, "alpha".to_string(), 0.0, 0.0),
            health: HealthMonitor::default(),
        }
    }

    /// Get the CE Wallet binding.
    #[wasm_bindgen(js_name = "getWallet")]
    pub fn wallet(&self) -> &CeWallet {
        &self.wallet
    }

    /// Get the GEI State binding.
    #[wasm_bindgen(js_name = "getGeiState")]
    pub fn gei_state(&self) -> &GeiState {
        &self.gei
    }

    /// Get the Resonance Status binding.
    #[wasm_bindgen(js_name = "getResonanceStatus")]
    pub fn resonance_status(&self) -> &ResonanceStatus {
        &self.resonance
    }

    /// Get the Health Monitor binding.
    #[wasm_bindgen(js_name = "getHealthMonitor")]
    pub fn health_monitor(&self) -> &HealthMonitor {
        &self.health
    }

    /// Update the GEI state from incoming data.
    #[wasm_bindgen(js_name = "updateGeiState")]
    pub fn update_gei_state(&mut self, x: f64, y: f64, z: f64) {
        self.gei = GeiState::new(x, y, z);
    }

    /// Update the resonance status from incoming data.
    #[wasm_bindgen(js_name = "updateResonanceStatus")]
    pub fn update_resonance_status(&mut self, sct_z: f32, brainwave_band: String, confidence: f32, homeostasis_target: f32) {
        self.resonance = ResonanceStatus::new(sct_z, brainwave_band, confidence, homeostasis_target);
    }

    /// Record a CE deposit.
    #[wasm_bindgen(js_name = "depositCe")]
    pub fn deposit_ce(&mut self, amount: f64) {
        self.wallet.deposit(amount);
    }

    /// Record a CE consumption.
    #[wasm_bindgen(js_name = "consumeCe")]
    pub fn consume_ce(&mut self, amount: f64) -> bool {
        self.wallet.consume(amount)
    }

    /// Record a heartbeat.
    #[wasm_bindgen(js_name = "recordHeartbeat")]
    pub fn record_heartbeat(&mut self, now_ms: u64) {
        self.health.heartbeat(now_ms);
    }

    /// Check health status.
    #[wasm_bindgen(js_name = "checkHealth")]
    pub fn check_health(&mut self, now_ms: u64) {
        self.health.check(now_ms);
    }

    /// Generate a complete dashboard JSON snapshot for Alpine.js reactivity.
    #[wasm_bindgen(js_name = "getDashboardJson")]
    pub fn get_dashboard_json(&self) -> String {
        format!(
            r#"{{"wallet":{},"gei":{},"resonance":{},"health":{}}}"#,
            self.wallet.to_json(),
            self.gei.to_json(),
            self.resonance.to_json(),
            self.health.to_json(),
        )
    }

    /// Reset all bindings to initial state.
    pub fn reset(&mut self) {
        self.wallet = CeWallet::new();
        self.gei = GeiState::new(0.0, 0.0, 0.0);
        self.resonance = ResonanceStatus::new(0.0, "alpha".to_string(), 0.0, 0.0);
        self.health = HealthMonitor::default();
    }
}

impl Default for UiBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── CeWallet Tests ───

    #[test]
    fn test_wallet_creation() {
        let wallet = CeWallet::new();
        assert_eq!(wallet.balance, 0.0);
        assert_eq!(wallet.total_deposited, 0.0);
        assert_eq!(wallet.total_consumed, 0.0);
        assert_eq!(wallet.transaction_count, 0);
    }

    #[test]
    fn test_wallet_deposit() {
        let mut wallet = CeWallet::new();
        wallet.deposit(100.0);
        assert_eq!(wallet.balance, 100.0);
        assert_eq!(wallet.total_deposited, 100.0);
        assert_eq!(wallet.transaction_count, 1);
    }

    #[test]
    fn test_wallet_deposit_negative_ignored() {
        let mut wallet = CeWallet::new();
        wallet.deposit(-50.0);
        assert_eq!(wallet.balance, 0.0);
        assert_eq!(wallet.transaction_count, 0);
    }

    #[test]
    fn test_wallet_consume_valid() {
        let mut wallet = CeWallet::new();
        wallet.deposit(100.0);
        assert!(wallet.consume(30.0));
        assert_eq!(wallet.balance, 70.0);
        assert_eq!(wallet.total_consumed, 30.0);
        assert_eq!(wallet.transaction_count, 2);
    }

    #[test]
    fn test_wallet_consume_insufficient() {
        let mut wallet = CeWallet::new();
        wallet.deposit(50.0);
        assert!(!wallet.consume(100.0));
        assert_eq!(wallet.balance, 50.0);
    }

    #[test]
    fn test_wallet_consume_zero() {
        let mut wallet = CeWallet::new();
        wallet.deposit(50.0);
        assert!(!wallet.consume(0.0));
        assert_eq!(wallet.balance, 50.0);
    }

    #[test]
    fn test_wallet_reset() {
        let mut wallet = CeWallet::new();
        wallet.deposit(100.0);
        wallet.consume(30.0);
        wallet.reset();
        assert_eq!(wallet.balance, 0.0);
        assert_eq!(wallet.transaction_count, 0);
    }

    #[test]
    fn test_wallet_to_json() {
        let mut wallet = CeWallet::new();
        wallet.deposit(100.0);
        let json = wallet.to_json();
        assert!(json.contains("\"balance\":100.0000"));
        assert!(json.contains("\"transactionCount\":1"));
    }

    #[test]
    fn test_wallet_default() {
        let wallet = CeWallet::default();
        assert_eq!(wallet.balance, 0.0);
    }

    // ─── GeiState Tests ───

    #[test]
    fn test_gei_creation() {
        let gei = GeiState::new(0.7, 0.5, 0.3);
        assert_eq!(gei.x, 0.7);
        assert_eq!(gei.y, 0.5);
        assert_eq!(gei.z, 0.3);
        assert!(gei.approved);
    }

    #[test]
    fn test_gei_rejected_when_z_negative() {
        let gei = GeiState::new(0.7, 0.5, -0.1);
        assert!(!gei.approved);
    }

    #[test]
    fn test_gei_stability_calculation() {
        let gei = GeiState::new(1.0, 1.0, 1.0);
        assert!(gei.stability > 0.8);
    }

    #[test]
    fn test_gei_harmonic() {
        let gei = GeiState::new(0.5, 0.5, 0.5);
        assert!(gei.is_harmonic());
    }

    #[test]
    fn test_gei_not_harmonic() {
        let gei = GeiState::new(1.0, 0.0, 0.0);
        assert!(!gei.is_harmonic());
    }

    #[test]
    fn test_gei_to_json() {
        let gei = GeiState::new(0.7, 0.5, 0.3);
        let json = gei.to_json();
        assert!(json.contains("\"x\":0.7000"));
        assert!(json.contains("\"approved\":true"));
    }

    // ─── ResonanceStatus Tests ───

    #[test]
    fn test_resonance_creation() {
        let status = ResonanceStatus::new(0.5, "alpha".to_string(), 0.8, 0.6);
        assert!(status.approved);
        assert_eq!(status.sct_z, 0.5);
        assert_eq!(status.brainwave_band, "alpha");
    }

    #[test]
    fn test_resonance_rejected() {
        let status = ResonanceStatus::new(-0.5, "theta".to_string(), 0.3, 0.4);
        assert!(!status.approved);
    }

    #[test]
    fn test_frequency_range_delta() {
        let status = ResonanceStatus::new(0.5, "delta".to_string(), 0.8, 0.6);
        let (min, max) = status.get_frequency_range();
        assert_eq!(min, 0.5);
        assert_eq!(max, 4.0);
    }

    #[test]
    fn test_frequency_range_gamma() {
        let status = ResonanceStatus::new(0.5, "gamma".to_string(), 0.8, 0.6);
        let (min, max) = status.get_frequency_range();
        assert_eq!(min, 30.0);
        assert_eq!(max, 100.0);
    }

    #[test]
    fn test_frequency_range_default() {
        let status = ResonanceStatus::new(0.5, "unknown".to_string(), 0.8, 0.6);
        let (min, max) = status.get_frequency_range();
        assert_eq!(min, 8.0);
        assert_eq!(max, 14.0);
    }

    #[test]
    fn test_resonance_to_json() {
        let status = ResonanceStatus::new(0.5, "alpha".to_string(), 0.8, 0.6);
        let json = status.to_json();
        assert!(json.contains("\"sctZ\":0.5000"));
        assert!(json.contains("\"brainwaveBand\":\"alpha\""));
        assert!(json.contains("\"approved\":true"));
    }

    // ─── HealthMonitor Tests ───

    #[test]
    fn test_health_monitor_creation() {
        let monitor = HealthMonitor::new(5000, 3);
        assert_eq!(monitor.status, "idle");
        assert_eq!(monitor.last_heartbeat, 0);
    }

    #[test]
    fn test_health_heartbeat() {
        let mut monitor = HealthMonitor::new(5000, 3);
        monitor.heartbeat(1000);
        assert_eq!(monitor.status, "healthy");
        assert_eq!(monitor.last_heartbeat, 1000);
        assert_eq!(monitor.missed_heartbeats, 0);
    }

    #[test]
    fn test_health_check_degraded() {
        let mut monitor = HealthMonitor::new(1000, 2);
        monitor.heartbeat(0);
        monitor.check(3000); // 3 intervals missed
        assert_eq!(monitor.status, "degraded");
    }

    #[test]
    fn test_health_check_failed() {
        let mut monitor = HealthMonitor::new(1000, 2);
        monitor.heartbeat(0);
        monitor.check(5000); // 5 intervals missed (>= 2 * max)
        assert_eq!(monitor.status, "failed");
    }

    #[test]
    fn test_health_set_status() {
        let mut monitor = HealthMonitor::default();
        monitor.set_status("starting");
        assert_eq!(monitor.status, "starting");
    }

    #[test]
    fn test_health_to_json() {
        let monitor = HealthMonitor::new(5000, 3);
        let json = monitor.to_json();
        assert!(json.contains("\"status\":\"idle\""));
        assert!(json.contains("\"heartbeatIntervalMs\":5000"));
    }

    #[test]
    fn test_health_default() {
        let monitor = HealthMonitor::default();
        assert_eq!(monitor.heartbeat_interval_ms, 5000);
        assert_eq!(monitor.max_missed_heartbeats, 3);
    }

    // ─── UiBridge Tests ───

    #[test]
    fn test_bridge_creation() {
        let bridge = UiBridge::new();
        assert_eq!(bridge.wallet.balance, 0.0);
        assert_eq!(bridge.health.status, "idle");
    }

    #[test]
    fn test_bridge_update_gei() {
        let mut bridge = UiBridge::new();
        bridge.update_gei_state(0.8, 0.6, 0.4);
        assert_eq!(bridge.gei.x, 0.8);
        assert_eq!(bridge.gei.y, 0.6);
        assert_eq!(bridge.gei.z, 0.4);
    }

    #[test]
    fn test_bridge_update_resonance() {
        let mut bridge = UiBridge::new();
        bridge.update_resonance_status(0.7, "theta".to_string(), 0.9, 0.5);
        assert_eq!(bridge.resonance.sct_z, 0.7);
        assert_eq!(bridge.resonance.brainwave_band, "theta");
    }

    #[test]
    fn test_bridge_deposit_and_consume() {
        let mut bridge = UiBridge::new();
        bridge.deposit_ce(200.0);
        assert_eq!(bridge.wallet.balance, 200.0);
        assert!(bridge.consume_ce(50.0));
        assert_eq!(bridge.wallet.balance, 150.0);
    }

    #[test]
    fn test_bridge_heartbeat() {
        let mut bridge = UiBridge::new();
        bridge.record_heartbeat(1000);
        assert_eq!(bridge.health.status, "healthy");
    }

    #[test]
    fn test_bridge_dashboard_json() {
        let bridge = UiBridge::new();
        let json = bridge.get_dashboard_json();
        assert!(json.contains("\"wallet\":"));
        assert!(json.contains("\"gei\":"));
        assert!(json.contains("\"resonance\":"));
        assert!(json.contains("\"health\":"));
    }

    #[test]
    fn test_bridge_reset() {
        let mut bridge = UiBridge::new();
        bridge.deposit_ce(100.0);
        bridge.update_gei_state(0.9, 0.8, 0.7);
        bridge.reset();
        assert_eq!(bridge.wallet.balance, 0.0);
        assert_eq!(bridge.gei.x, 0.0);
        assert_eq!(bridge.health.status, "idle");
    }

    #[test]
    fn test_bridge_default() {
        let bridge = UiBridge::default();
        assert_eq!(bridge.wallet.balance, 0.0);
    }
}
