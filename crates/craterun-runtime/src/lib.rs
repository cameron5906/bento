pub mod adapter;
pub mod detect;
pub mod types;
pub mod adapters;

pub use adapter::RuntimeAdapter;
pub use types::{ImageRef, RuntimeDetectionResult, RuntimePlan, PlannedService, RemoveOptions};
