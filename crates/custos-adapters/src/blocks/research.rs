//! Research Workbench Adapter Block
//!
//! Provides concrete boundary implementations for interactive Jupyter/Python kernels,
//! content-addressed checkpoints, and remote cloud GPU compute targets (Colab, Kaggle, Rented GPUs).

pub use crate::compute::checkpoint_store::CasCheckpointStoreAdapter;
pub use crate::compute::colab::ColabTargetAdapter;
pub use crate::compute::kaggle::KaggleTargetAdapter;
pub use crate::compute::rented_gpu::RentedGpuTargetAdapter;
pub use crate::notebook::driver::{LocalNotebookWorkerSession, LocalPythonDriver};
