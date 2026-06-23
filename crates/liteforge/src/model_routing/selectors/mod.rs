//! Concrete [`ModelSelector`](crate::routing::ModelSelector) implementations.

pub mod mf;
pub mod remote_classifier;
pub mod semantic;
pub mod static_selector;

pub use mf::MfSelector;
pub use remote_classifier::{ClassifierEndpoint, ClassifierResponse, RemoteClassifierSelector};
pub use semantic::{SemanticRoute, SemanticSelector};
pub use static_selector::StaticSelector;
