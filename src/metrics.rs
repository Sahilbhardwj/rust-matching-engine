use std::sync::atomic::{AtomicU64, Ordering};

pub struct EngineMetrics {
    pub orders_processed: AtomicU64,
    pub trades_executed: AtomicU64,
}

impl EngineMetrics {
    pub fn new() -> Self {
        Self {
            orders_processed: AtomicU64::new(0),
            trades_executed: AtomicU64::new(0),
        }
    }

  
   
     pub fn snapshot(&self) -> (u64, u64) {
        (
            self.orders_processed.load(Ordering::Relaxed),
            self.trades_executed.load(Ordering::Relaxed),
        )
    }
}