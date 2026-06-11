//! ed2k-p2p — P2P Networking Layer
//!
//! libp2p-based peer-to-peer networking for ed2kIA distributed nodes.
//! Handles peer discovery, gossipsub, and request-response protocols.

pub mod distributed_sae;
pub mod gossip;
pub mod peer;
pub mod request_response;
pub mod swarm;
pub mod universal_symbiosis_protocol;
