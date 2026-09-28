#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RdcPhase {
    Resolve,
    Delegate,
    Check,
}
