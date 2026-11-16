//! Canonical Port Registry (re-exported from custos_contracts::contracts)

pub use custos_contracts::contracts::*;
pub use crate::task::trusted::TrustedKernel;

pub mod kernel {
    pub use custos_contracts::contracts::kernel::*;
    pub use crate::task::trusted::TrustedKernel;
}
