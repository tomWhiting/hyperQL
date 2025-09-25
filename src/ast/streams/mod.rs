//! # Stream AST Nodes
//!
//! This module defines AST nodes for stream operations in HyperQL, enabling
//! declarative stream processing and event-driven query capabilities.

pub mod operations;
pub mod triggers;

// Re-export key types
pub use operations::{StreamOperation, CreateStreamStatement, ProduceStatement, ConsumeStatement};
pub use triggers::{CreateTriggerStatement, TriggerCondition as AstTriggerCondition};