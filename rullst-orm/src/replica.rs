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
    use super::ReplicaPool;
    use crate::pool::tests::lazy_pool;

    #[tokio::test]
    async fn reads_rotate_over_the_replicas_and_writes_use_the_primary() {
        let replica_pool = ReplicaPool::new(lazy_pool(), vec![lazy_pool(), lazy_pool()]);
        assert_eq!(replica_pool.replica_count(), 2);
        let picks: Vec<*const crate::RullstPool> = (0..5)
            .map(|_| std::ptr::from_ref(replica_pool.read()))
            .collect();
        let first = std::ptr::from_ref(&replica_pool.replicas[0]);
        let second = std::ptr::from_ref(&replica_pool.replicas[1]);
        assert_eq!(picks, [first, second, first, second, first]);
        assert!(std::ptr::eq(replica_pool.write(), &replica_pool.primary));

        let primary_only = ReplicaPool::primary_only(lazy_pool());
        assert_eq!(primary_only.replica_count(), 0);
        assert!(std::ptr::eq(primary_only.read(), &primary_only.primary));
    }
}
