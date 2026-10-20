//! Hosted Compute & GPU Adapters (F10 / P3).
//!
//! Concrete adapters for:
//! 1. `KaggleTargetAdapter` — Kaggle Kernels via Kaggle CLI / REST API.
//! 2. `ColabTargetAdapter` — Google Colab runtimes via Colab CLI (`google-colab-cli`).
//! 3. `RentedGpuTargetAdapter` — External rented GPUs (RunPod, Vast.ai, Lambda Cloud, Dedicated SSH).

pub mod colab;
pub mod kaggle;
pub mod rented_gpu;

pub use colab::ColabTargetAdapter;
pub use kaggle::KaggleTargetAdapter;
pub use rented_gpu::RentedGpuTargetAdapter;
