#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(pub i64);

impl Price {
    pub fn from_f64(price: f64) -> Self {
        Self((price * 1_000_000.0).round() as i64)
    }

    pub fn as_f64(self) -> f64 {
        self.0 as f64 / 1_000_000.0
    }
}
