#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Value {
    Unknown,
    Low,
    High,
}

impl std::ops::Not for Value {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Value::Low => Value::High,
            Value::High => Value::Low,
            Value::Unknown => Value::Unknown,
        }
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        if b {
            Self::High
        } else {
            Self::Low
        }
    }
}
