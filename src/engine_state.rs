use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EngineState {
    Stopped = 0,
    Running = 1,
    ShuttingDown = 2,
}

pub struct EngineLifecycle {
    state: AtomicU8,
}

impl EngineLifecycle {
    pub fn new() -> Self {
        Self {
            state: AtomicU8::new(EngineState::Stopped as u8),
        }
    }

    pub fn start(&self) -> bool {
        self.state
            .compare_exchange(
                EngineState::Stopped as u8,
                EngineState::Running as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }
    pub fn shutdown(&self) -> bool {
    self.state
        .compare_exchange(
            EngineState::Running as u8,
            EngineState::ShuttingDown as u8,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_ok()
}
pub fn is_running(&self) -> bool {
    self.state.load(Ordering::Acquire)
        == EngineState::Running as u8
}
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_can_start_once() {
        let lifecycle = EngineLifecycle::new();

        assert!(lifecycle.start());
        assert!(!lifecycle.start());
    }
}
