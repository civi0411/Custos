//! Canonical Port Registry (re-exported from custos_contracts::contracts)

pub use crate::task::trusted::TrustedKernel;
pub use custos_contracts::contracts::*;

pub mod kernel {
    pub use crate::task::trusted::TrustedKernel;
    pub use custos_contracts::contracts::kernel::*;
}
