//! # Cascade Configuration AST
//!
//! AST nodes for cascade configurations in schema definitions.
//! Cascades enable automatic property aggregation through active edges.

use serde::{Deserialize, Serialize};

/// Cascade configuration for aggregated fields
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CascadeConfiguration {
    /// Aggregation function to apply
    pub aggregation: AggregationFunction,
    
    /// Source property names to aggregate from
    pub source_properties: Vec<String>,
    
    /// Edge direction for cascade propagation (defaults to Incoming)
    pub direction: Option<EdgeDirection>,
    
    /// Specific edge types to follow (None = all active edges)
    pub edge_types: Option<Vec<String>>,
    
    /// Decay function for temporal aggregations
    pub decay: Option<DecayFunction>,
    
    /// Timestamp field for temporal aggregations
    pub timestamp_field: Option<String>,
    
    /// Update frequency in seconds between cascade recalculations
    pub update_frequency: Option<u64>,
}

/// Aggregation functions for cascade operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregationFunction {
    /// Sum all values
    Sum,
    
    /// Calculate average
    Average,
    
    /// Weighted average based on edge weights
    WeightedAverage,
    
    /// Maximum value
    Max,
    
    /// Minimum value
    Min,
    
    /// Count of values
    Count,
    
    /// Most recent value by timestamp
    Latest,
    
    /// First value by timestamp
    First,
    
    /// Moving mean over time window
    MovingMean {
        /// Time window in days
        window_days: u32,
    },
    
    /// Moving median over time window
    MovingMedian {
        /// Time window in days
        window_days: u32,
    },
    
    /// Time-windowed average
    TimeWindowedAverage {
        /// Time window in days
        window_days: u32,
    },
    
    /// Percentile calculation
    Percentile {
        /// Percentile value (0-100)
        percentile: u8,
    },
}

/// Decay functions for temporal weighting
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DecayFunction {
    /// No decay - all values weighted equally
    None,
    
    /// Exponential decay: weight = exp(-alpha * time_diff)
    Exponential {
        /// Decay rate parameter
        alpha: f32,
    },
    
    /// Power law decay: weight = (1 + time_diff)^(-alpha)
    PowerLaw {
        /// Decay exponent parameter
        alpha: f32,
    },
    
    /// Linear decay: weight = max(0, 1 - alpha * time_diff)
    Linear {
        /// Decay rate parameter
        alpha: f32,
    },
}

/// Edge direction for cascade propagation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EdgeDirection {
    /// Aggregate from entities pointing TO this one (default)
    Incoming,
    
    /// Aggregate from entities this one points TO
    Outgoing,
    
    /// Aggregate from all connected entities
    Both,
}
