//! Autonomous Edge Swarm & Mesh Network Protocol (`rullst_iot::mesh`).

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

/// Status of a mesh network node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeStatus {
    Online,
    Degraded,
    Offline,
}

/// A single node in the IoT mesh network.
#[derive(Clone, Debug)]
pub struct MeshNode {
    pub node_id: String,
    pub status: NodeStatus,
    pub rssi_dbm: i16,
}

impl MeshNode {
    /// Creates a new mesh node entry.
    pub fn new(node_id: impl Into<String>, rssi_dbm: i16) -> Self {
        Self {
            node_id: node_id.into(),
            status: NodeStatus::Online,
            rssi_dbm,
        }
    }
}

/// Self-healing P2P mesh network topology manager.
pub struct MeshTopology {
    pub nodes: Vec<MeshNode>,
}

impl MeshTopology {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Registers a node into the mesh topology.
    pub fn register(&mut self, node: MeshNode) {
        self.nodes.push(node);
    }

    /// Resolves the best relay path by RSSI strength (highest signal = closest).
    pub fn best_relay(&self) -> Option<&MeshNode> {
        self.nodes
            .iter()
            .filter(|n| n.status == NodeStatus::Online)
            .max_by_key(|n| n.rssi_dbm)
    }
}

impl Default for MeshTopology {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_topology_best_relay() {
        let mut mesh = MeshTopology::new();
        mesh.register(MeshNode::new("node-01", -72));
        mesh.register(MeshNode::new("node-02", -55));
        mesh.register(MeshNode::new("node-03", -80));

        let best = mesh.best_relay().unwrap();
        assert_eq!(best.node_id, "node-02");
    }
}
