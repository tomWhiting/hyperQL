//! # Schema DDL AST Definitions
//!
//! This module defines the Abstract Syntax Tree nodes for schema DDL statements
//! in HyperQL. Schemas provide type definitions and field specifications for
//! collections, enabling schema-aware property comparison and cascade configuration.
//!
//! ## Purpose
//!
//! Schema DDL statements allow users to:
//! - Define collection schemas with typed fields
//! - Configure cascaded fields for automatic aggregation
//! - Specify property scopes for similarity comparison
//! - Control schema extensibility modes
//! - Manage schema lifecycle (create, alter, drop)
//!
//! ## Statement Types
//!
//! ### CREATE SCHEMA
//! Define a new schema for a collection with field specifications:
//! - Field definitions with types and constraints
//! - Property scopes (Metadata, CollectionSpecific, DomainShared)
//! - Field kinds (Regular, Cascaded, Calculated, Computed)
//! - Extensibility mode (Closed, Open, Typed)
//!
//! ### ALTER SCHEMA
//! Modify an existing schema:
//! - Add new fields
//! - Drop existing fields
//! - Modify field definitions
//! - Change extensibility mode
//!
//! ### DROP SCHEMA
//! Remove a schema definition from a collection
//!
//! ### DESCRIBE SCHEMA
//! Inspect schema definitions and metadata
//!
//! ## Field Specifications
//!
//! Fields support 17 fundamental types:
//! - Primitives: String, Integer, Float, Boolean
//! - Temporal: Timestamp, Duration, Date
//! - Structured: Json, Array, Map
//! - References: Reference, Edge
//! - Special: Embedding, Position3D, Bytes
//!
//! ## Property Scopes
//!
//! Property scopes control similarity comparison behavior:
//! - **Metadata**: Excluded from similarity (e.g., created_at, id)
//! - **CollectionSpecific**: Compared within collection only (e.g., status codes)
//! - **DomainShared**: Globally comparable (e.g., sentiment, price)
//!
//! ## Cascade Configuration
//!
//! Cascaded fields enable automatic property aggregation:
//! - 12 aggregation functions (Sum, Average, Max, Min, Count, etc.)
//! - 4 decay functions (None, Exponential, PowerLaw, Linear)
//! - Directional control (Incoming, Outgoing, Both)
//! - Edge type filtering
//! - Temporal windowing
//!
//! ## Integration
//!
//! Schema AST nodes integrate with:
//! - **Parser**: Converts DDL syntax to AST nodes
//! - **Compiler**: Validates and compiles schema definitions
//! - **SchemaEngine**: Stores and manages runtime schemas
//! - **HNSW**: Uses scopes for property distance calculation

pub mod operations;
pub mod field_spec;
pub mod cascade;
mod tests;

pub use operations::*;
pub use field_spec::*;
pub use cascade::*;
