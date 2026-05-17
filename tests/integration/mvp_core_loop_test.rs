//! MVP Core Loop Integration Test
//!
//! Feature-gated behind `v2.1-mvp-core`. Validates the full core loop cycle:
//! Discovery → Distribution → Inference → Collection.

#![cfg(feature = "v2.1-mvp-core")]

use ed2kIA::mvp_core::{CoreLoop, CoreLoopResult};

#[tokio::test]
async fn test_full_core_loop_cycle() {
    let mut core = CoreLoop::new();
    let result = core.run_cycle().await.expect("Core loop cycle should complete");

    // Validate scaffold returns consistent mock data
    assert_eq!(result.peers_discovered, 3, "Should discover 3 mock peers");
    assert_eq!(result.tasks_distributed, 3, "Should distribute 3 tasks");
    assert_eq!(result.inference_completed, 3, "Should complete 3 inferences");
    assert_eq!(result.results_collected, 3, "Should collect 3 results");
}

#[tokio::test]
async fn test_core_loop_result_structure() {
    let mut core = CoreLoop::new();
    let result = core.run_cycle().await.unwrap();

    // Validate result structure
    let CoreLoopResult {
        peers_discovered,
        tasks_distributed,
        inference_completed,
        results_collected,
    } = result;

    // All counts should be positive
    assert!(peers_discovered > 0);
    assert!(tasks_distributed > 0);
    assert!(inference_completed > 0);
    assert!(results_collected > 0);

    // Pipeline should be consistent (each stage processes same count)
    assert_eq!(peers_discovered, tasks_distributed);
    assert_eq!(tasks_distributed, inference_completed);
    assert_eq!(inference_completed, results_collected);
}

#[tokio::test]
async fn test_core_loop_multiple_cycles() {
    let mut core = CoreLoop::new();

    // Run cycle multiple times to validate state reset
    for i in 1..=3 {
        let result = core.run_cycle().await.expect(&format!("Cycle {} should complete", i));
        assert!(result.peers_discovered > 0, "Cycle {} should discover peers", i);
    }
}
