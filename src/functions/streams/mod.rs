//! # Stream Functions
//!
//! This module provides built-in functions for stream operations and processing in HyperQL.

pub mod operations;
pub mod windowing;

// Re-export key types
pub use operations::{StreamOperationFunction, stream_create, stream_produce, stream_consume};
pub use windowing::{WindowFunction, window_tumbling, window_sliding, window_hyperbolic};