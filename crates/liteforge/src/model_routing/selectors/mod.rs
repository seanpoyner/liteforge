//! Concrete [`ModelSelector`](crate::routing::ModelSelector) implementations.

pub mod embedding_head;
pub mod mf;
pub mod remote_classifier;
pub mod semantic;
pub mod static_selector;

pub use embedding_head::{EmbeddingHeadSelector, HeadSpec};
pub use mf::MfSelector;
pub use remote_classifier::{ClassifierEndpoint, ClassifierResponse, RemoteClassifierSelector};
pub use semantic::{SemanticRoute, SemanticSelector};
pub use static_selector::StaticSelector;
