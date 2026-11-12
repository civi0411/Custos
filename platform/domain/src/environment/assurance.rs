use crate::authority::action::Assurance;
use serde::{Deserialize, Serialize};

/// Measured assurance and isolation guarantees of an active environment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentAssurance {
    pub measured_assurance: Assurance,
    pub filesystem_confined: bool,
    pub network_restricted: bool,
    pub process_isolated: bool,
    pub isolation_driver: String,
}

impl EnvironmentAssurance {
    pub fn unconstrained() -> Self {
        Self {
            measured_assurance: Assurance::ObserveOnly,
            filesystem_confined: false,
            network_restricted: false,
            process_isolated: false,
            isolation_driver: "none".into(),
        }
    }
}

