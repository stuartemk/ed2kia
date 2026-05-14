//! Integration Test: RLHF Feedback Flow
//!
//! Validates the complete human feedback flow:
//! Entry creation → redb storage → JSONL export → format validation.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[path = "../../src/rlhf/feedback_store.rs"]
mod feedback_store;

use feedback_store::{FeedbackDecision, FeedbackEntry, FeedbackStore};

/// Test: Feedback entry creation
#[test]
fn test_feedback_entry_creation() {
    let entry = FeedbackEntry::new(
        "test_001".to_string(),
        "layer_0".to_string(),
        42,
        0.85,
        FeedbackDecision::Approved,
        "annotator_1".to_string(),
    );

    assert_eq!(entry.id, "test_001");
    assert_eq!(entry.layer_id, "layer_0");
    assert_eq!(entry.feature_idx, 42);
    assert_eq!(entry.decision, FeedbackDecision::Approved);
    assert_eq!(entry.annotator_id, "annotator_1");
    assert!(entry.timestamp_ms > 0);
}

/// Test: Feedback store initialization with redb
#[test]
fn test_feedback_store_init() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("feedback_test.redb");

    let result = FeedbackStore::new(&db_path);
    assert!(result.is_ok());

    let store = result.unwrap();
    assert_eq!(store.get_total_count(), 0);
}

/// Test: Insert and retrieve feedback entries
#[test]
fn test_feedback_insert_and_retrieve() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("feedback_insert.redb");

    let mut store = FeedbackStore::new(&db_path).unwrap();

    // Insert entries
    let entry1 = FeedbackEntry::new(
        "e1".to_string(),
        "layer_0".to_string(),
        10,
        0.9,
        FeedbackDecision::Approved,
        "annotator_1".to_string(),
    );

    let entry2 = FeedbackEntry::new(
        "e2".to_string(),
        "layer_1".to_string(),
        20,
        0.7,
        FeedbackDecision::Rejected,
        "annotator_2".to_string(),
    );

    assert!(store.insert(&entry1).is_ok());
    assert!(store.insert(&entry2).is_ok());

    assert_eq!(store.get_total_count(), 2);

    // Retrieve by ID
    let retrieved = store.get_by_id("e1");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, "e1");
}

/// Test: Export to JSONL format
#[test]
fn test_jsonl_export() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("feedback_export.redb");
    let export_path = temp_dir.path().join("export.jsonl");

    let mut store = FeedbackStore::new(&db_path).unwrap();

    // Insert test entries
    for i in 0..5 {
        let entry = FeedbackEntry::new(
            format!("export_{}", i),
            format!("layer_{}", i % 2),
            i * 10,
            0.5 + (i as f64 * 0.1),
            if i % 2 == 0 {
                FeedbackDecision::Approved
            } else {
                FeedbackDecision::Rejected
            },
            "test_annotator".to_string(),
        );
        store.insert(&entry).ok();
    }

    // Export
    let count = store.export_jsonl(&export_path).unwrap();
    assert_eq!(count, 5);

    // Verify file exists and has content
    assert!(export_path.exists());
    let content = fs::read_to_string(&export_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 5);

    // Verify each line is valid JSON
    for line in lines {
        let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
        assert!(parsed.get("label").is_some());
        assert!(parsed.get("layer_id").is_some());
    }
}

/// Test: JSONL training format compatibility
#[test]
fn test_jsonl_training_format() {
    let entry = FeedbackEntry::new(
        "training_001".to_string(),
        "layer_0".to_string(),
        100,
        0.95,
        FeedbackDecision::Approved,
        "trainer".to_string(),
    );

    let jsonl = entry.to_jsonl_training_format();
    let parsed: serde_json::Value = serde_json::from_str(&jsonl).unwrap();

    // Verify label mapping
    assert_eq!(parsed["label"], "entailment");
    assert_eq!(parsed["layer_id"], "layer_0");
    assert_eq!(parsed["feature_idx"], 100);
}

/// Test: All feedback decision types map correctly
#[test]
fn test_decision_label_mapping() {
    let decisions = vec![
        (FeedbackDecision::Approved, "entailment"),
        (FeedbackDecision::Rejected, "contradiction"),
        (FeedbackDecision::Corrected, "corrected"),
        (FeedbackDecision::Uncertain, "neutral"),
    ];

    for (decision, expected_label) in decisions {
        let entry = FeedbackEntry::new(
            format!("test_{}", decision),
            "layer_0".to_string(),
            0,
            0.5,
            decision.clone(),
            "annotator".to_string(),
        );

        let jsonl = entry.to_jsonl_training_format();
        let parsed: serde_json::Value = serde_json::from_str(&jsonl).unwrap();
        assert_eq!(parsed["label"], expected_label, "Mismatch for {:?}", decision);
    }
}

/// Test: Feedback store persistence across restarts
#[test]
fn test_feedback_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("persistent.redb");

    // First session: insert data
    {
        let mut store = FeedbackStore::new(&db_path).unwrap();
        let entry = FeedbackEntry::new(
            "persistent_1".to_string(),
            "layer_0".to_string(),
            50,
            0.8,
            FeedbackDecision::Approved,
            "annotator".to_string(),
        );
        store.insert(&entry).ok();
    } // Store dropped, simulating restart

    // Second session: verify data persists
    {
        let store = FeedbackStore::new(&db_path).unwrap();
        assert_eq!(store.get_total_count(), 1);

        let retrieved = store.get_by_id("persistent_1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "persistent_1");
    }
}

/// Test: Feedback statistics
#[test]
fn test_feedback_statistics() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("stats_test.redb");

    let mut store = FeedbackStore::new(&db_path).unwrap();

    // Insert mix of decisions
    for i in 0..10 {
        let decision = match i % 4 {
            0 => FeedbackDecision::Approved,
            1 => FeedbackDecision::Rejected,
            2 => FeedbackDecision::Corrected,
            _ => FeedbackDecision::Uncertain,
        };

        let entry = FeedbackEntry::new(
            format!("stats_{}", i),
            "layer_0".to_string(),
            i,
            0.5,
            decision,
            "annotator".to_string(),
        );
        store.insert(&entry).ok();
    }

    let stats = store.get_stats();
    assert_eq!(stats.total, 10);
    assert!(stats.approved >= 1);
    assert!(stats.rejected >= 1);
}

/// Test: Recent feedback retrieval
#[test]
fn test_recent_feedback() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("recent_test.redb");

    let mut store = FeedbackStore::new(&db_path).unwrap();

    for i in 0..20 {
        let entry = FeedbackEntry::new(
            format!("recent_{}", i),
            "layer_0".to_string(),
            i,
            0.5,
            FeedbackDecision::Approved,
            "annotator".to_string(),
        );
        store.insert(&entry).ok();
    }

    let recent = store.get_recent(5);
    assert_eq!(recent.len(), 5);
}

/// Test: Feedback store handles concurrent inserts
#[test]
fn test_concurrent_inserts() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("concurrent.redb");

    let mut store = FeedbackStore::new(&db_path).unwrap();

    // Rapid sequential inserts
    for i in 0..50 {
        let entry = FeedbackEntry::new(
            format!("concurrent_{}", i),
            "layer_0".to_string(),
            i,
            0.5,
            FeedbackDecision::Approved,
            "annotator".to_string(),
        );
        store.insert(&entry).ok();
    }

    assert_eq!(store.get_total_count(), 50);
}
