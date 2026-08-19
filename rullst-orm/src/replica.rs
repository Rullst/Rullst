use crate::RullstPool;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Multi-Database Read Replica Load Balancer.
/// Transparently load-balances read queries across secondary replicas while sending write queries to the primary database.
pub struct ReplicaPool {
    primary: RullstPool,
    replicas: Vec<RullstPool>,
    current_replica: AtomicUsize,
}

impl ReplicaPool {
    /// Creates a new ReplicaPool with a primary pool and a list of read replica pools.
    pub fn new(primary: RullstPool, replicas: Vec<RullstPool>) -> Self {
        Self {
            primary,
            replicas,
            current_replica: AtomicUsize::new(0),
        }
    }

    /// Creates a ReplicaPool with only a primary pool (no read replicas).
    pub fn primary_only(primary: RullstPool) -> Self {
        Self::new(primary, Vec::new())
    }

    /// Returns a pool reference for read queries using Round-Robin balancing across replicas.
    /// Falls back to the primary pool if no replicas are available.
    pub fn read(&self) -> &RullstPool {
        if self.replicas.is_empty() {
            &self.primary
        } else {
            let idx = self.current_replica.fetch_add(1, Ordering::Relaxed) % self.replicas.len();
            &self.replicas[idx]
        }
    }

    /// Returns a pool reference to the primary database for write/mutation operations.
    pub fn write(&self) -> &RullstPool {
        &self.primary
    }

    /// Returns the number of active read replicas configured.
    pub fn replica_count(&self) -> usize {
        self.replicas.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_replica_pool_count_and_rotation() {
        let pool = RullstPool::connect("sqlite::memory:").await.unwrap();
        let replica1 = RullstPool::connect("sqlite::memory:").await.unwrap();
        let replica2 = RullstPool::connect("sqlite::memory:").await.unwrap();

        let replica_pool = ReplicaPool::new(pool.clone(), vec![replica1, replica2]);
        assert_eq!(replica_pool.replica_count(), 2);

        // Test read round-robin
        let _r1 = replica_pool.read();
        let _r2 = replica_pool.read();
        let _r3 = replica_pool.read();

        let primary_only = ReplicaPool::primary_only(pool);
        assert_eq!(primary_only.replica_count(), 0);
        let _p_read = primary_only.read();
    }
}
