//! Semantic Graph — In-memory semantic graph using petgraph + dashmap.
//!
//! Provides `SemanticGraph` for mapping activations between SAE feature IDs
//! and natural language tokens, enabling the "Piedra Rosetta" translation layer.

#[cfg(feature = "v2.1-semantic-graph")]
use dashmap::DashMap;
#[cfg(feature = "v2.1-semantic-graph")]
use petgraph::stable_graph::{NodeIndex, StableGraph};
#[cfg(feature = "v2.1-semantic-graph")]
use petgraph::visit::EdgeRef;
#[cfg(feature = "v2.1-semantic-graph")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "v2.1-semantic-graph")]
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Token,
    Feature,
}

#[cfg(feature = "v2.1-semantic-graph")]
#[derive(Debug, Clone)]
pub struct ConceptNode {
    pub label: String,
    pub node_type: NodeType,
    pub weight: f64,
}

#[cfg(feature = "v2.1-semantic-graph")]
#[derive(Debug, Clone)]
pub struct ActivationEdge {
    pub weight: f64,
}

/// In-memory semantic graph mapping tokens ↔ SAE features.
///
/// Uses `StableGraph` for the graph structure and `DashMap` for O(1) label→index lookups.
#[cfg(feature = "v2.1-semantic-graph")]
pub struct SemanticGraph {
    index_map: Arc<DashMap<String, NodeIndex>>,
    graph: Arc<Mutex<StableGraph<ConceptNode, ActivationEdge>>>,
}

#[cfg(feature = "v2.1-semantic-graph")]
impl SemanticGraph {
    /// Create a new empty `SemanticGraph`.
    pub fn new() -> Self {
        Self {
            index_map: Arc::new(DashMap::new()),
            graph: Arc::new(Mutex::new(StableGraph::new())),
        }
    }

    /// Insert or update an activation between a token and a feature.
    ///
    /// Creates the token node, feature node, and weighted edge if they don't exist.
    /// If the edge already exists, updates the weight.
    pub fn insert_activation(&self, token: &str, feature_id: &str, weight: f64) {
        let token_index =
            Self::ensure_node(&self.index_map, &self.graph, token, NodeType::Token, weight);
        let feature_index = Self::ensure_node(
            &self.index_map,
            &self.graph,
            feature_id,
            NodeType::Feature,
            weight,
        );

        // Update or insert edge
        {
            let mut g = self.graph.lock().unwrap();
            let edge_exists = g.edges(token_index).any(|e| e.target() == feature_index);
            if edge_exists {
                let edge_ref = g
                    .edge_indices()
                    .find(|&idx| {
                        let (s, t) = g.edge_endpoints(idx).unwrap();
                        (s == token_index && t == feature_index)
                            || (s == feature_index && t == token_index)
                    })
                    .expect("edge must exist");
                let edge = g.edge_weight_mut(edge_ref).unwrap();
                edge.weight = weight;
            } else {
                g.add_edge(token_index, feature_index, ActivationEdge { weight });
            }
        }
    }

    /// Query the top `k` features for a given token by activation weight.
    pub fn get_top_features_for_token(&self, token: &str, top_k: usize) -> Vec<(String, f64)> {
        let index = match self.index_map.get(token) {
            Some(val) => *val.value(),
            None => return Vec::new(),
        };

        let g = self.graph.lock().unwrap();
        let mut edges: Vec<(String, f64)> = g
            .edges_directed(index, petgraph::Direction::Outgoing)
            .filter_map(|e| {
                let target_node = g.node_weight(e.target()).unwrap();
                if target_node.node_type == NodeType::Feature {
                    Some((target_node.label.clone(), e.weight().weight))
                } else {
                    None
                }
            })
            .collect();
        edges.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        edges.into_iter().take(top_k).collect()
    }

    /// Query the top `k` tokens for a given feature by activation weight.
    pub fn get_top_tokens_for_feature(&self, feature_id: &str, top_k: usize) -> Vec<(String, f64)> {
        let index = match self.index_map.get(feature_id) {
            Some(val) => *val.value(),
            None => return Vec::new(),
        };

        let g = self.graph.lock().unwrap();
        let mut edges: Vec<(String, f64)> = g
            .edges_directed(index, petgraph::Direction::Incoming)
            .filter_map(|e| {
                let source_node = g.node_weight(e.source()).unwrap();
                if source_node.node_type == NodeType::Token {
                    Some((source_node.label.clone(), e.weight().weight))
                } else {
                    None
                }
            })
            .collect();
        edges.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        edges.into_iter().take(top_k).collect()
    }

    /// Get all nodes as a vector of `(label, node_type, weight)` for visualization.
    pub fn get_all_nodes(&self) -> Vec<(String, NodeType, f64)> {
        let g = self.graph.lock().unwrap();
        g.node_weights()
            .map(|n| (n.label.clone(), n.node_type.clone(), n.weight))
            .collect()
    }

    /// Get all edges as a vector of `(source, target, weight)` for visualization.
    pub fn get_all_edges(&self) -> Vec<(String, String, f64)> {
        let g = self.graph.lock().unwrap();
        g.edge_indices()
            .filter_map(|idx| {
                let (src, tgt) = g.edge_endpoints(idx).unwrap();
                let src_label = g.node_weight(src).unwrap().label.clone();
                let tgt_label = g.node_weight(tgt).unwrap().label.clone();
                let weight = g.edge_weight(idx).unwrap().weight;
                Some((src_label, tgt_label, weight))
            })
            .collect()
    }

    /// Return the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.graph.lock().unwrap().node_count()
    }

    /// Return the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.graph.lock().unwrap().edge_count()
    }

    /// Ensure a node exists in the graph, creating it if necessary.
    fn ensure_node(
        index_map: &DashMap<String, NodeIndex>,
        graph: &Mutex<StableGraph<ConceptNode, ActivationEdge>>,
        label: &str,
        node_type: NodeType,
        weight: f64,
    ) -> NodeIndex {
        if let Some(val) = index_map.get(label) {
            return *val.value();
        }
        let mut g = graph.lock().unwrap();
        let index = g.add_node(ConceptNode {
            label: label.to_string(),
            node_type,
            weight,
        });
        index_map.insert(label.to_string(), index);
        index
    }
}

#[cfg(feature = "v2.1-semantic-graph")]
impl Default for SemanticGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "v2.1-semantic-graph"))]
mod tests {
    use super::*;

    #[test]
    fn test_graph_creation() {
        let g = SemanticGraph::new();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn test_insert_activation() {
        let g = SemanticGraph::new();
        g.insert_activation("neural", "feat-1", 0.9);
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn test_get_top_features_for_token() {
        let g = SemanticGraph::new();
        g.insert_activation("neural", "feat-1", 0.9);
        g.insert_activation("neural", "feat-2", 0.7);
        g.insert_activation("neural", "feat-3", 0.5);
        let top = g.get_top_features_for_token("neural", 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "feat-1");
        assert_eq!(top[0].1, 0.9);
        assert_eq!(top[1].0, "feat-2");
        assert_eq!(top[1].1, 0.7);
    }

    #[test]
    fn test_get_top_tokens_for_feature() {
        let g = SemanticGraph::new();
        g.insert_activation("neural", "feat-1", 0.9);
        g.insert_activation("deep", "feat-1", 0.8);
        let top = g.get_top_tokens_for_feature("feat-1", 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "neural");
        assert_eq!(top[0].1, 0.9);
    }

    #[test]
    fn test_insert_activation_updates_weight() {
        let g = SemanticGraph::new();
        g.insert_activation("neural", "feat-1", 0.5);
        g.insert_activation("neural", "feat-1", 0.9);
        let top = g.get_top_features_for_token("neural", 1);
        assert_eq!(top[0].1, 0.9);
    }

    #[test]
    fn test_get_top_features_unknown_token() {
        let g = SemanticGraph::new();
        let top = g.get_top_features_for_token("unknown", 5);
        assert!(top.is_empty());
    }

    #[test]
    fn test_get_all_nodes() {
        let g = SemanticGraph::new();
        g.insert_activation("neural", "feat-1", 0.9);
        let nodes = g.get_all_nodes();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn test_get_all_edges() {
        let g = SemanticGraph::new();
        g.insert_activation("neural", "feat-1", 0.9);
        let edges = g.get_all_edges();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].2, 0.9);
    }

    #[test]
    fn test_default() {
        let g = SemanticGraph::default();
        assert_eq!(g.node_count(), 0);
    }
}
