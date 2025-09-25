//! # Timeseries-Specific AST Nodes
//!
//! This module defines AST nodes specifically for temporal operations and time
//! series analysis within HyperQL. These nodes represent window functions, temporal
//! aggregations, and time-based expressions that leverage both temporal patterns
//! and hyperbolic positioning for spatio-temporal queries.
//!
//! ## Purpose
//!
//! The timeseries AST nodes enable sophisticated temporal query representations:
//! - Window functions over time-ordered data streams
//! - Temporal aggregations with various time bucketing strategies
//! - Time-based filtering and constraint expressions
//! - Temporal pattern matching and sequence detection
//! - Integration with spatial positioning for spatio-temporal analysis
//!
//! ## Design Principles
//!
//! Timeseries AST nodes follow specific design principles:
//!
//! ### Temporal Semantics
//! Nodes maintain consistent temporal semantics:
//! - Well-defined time ordering and precedence
//! - Proper handling of time zones and daylight saving
//! - Consistent timestamp resolution and precision
//! - Support for various temporal granularities
//!
//! ### Streaming Support
//! Nodes are designed for streaming time series:
//! - Incremental computation capabilities
//! - Memory-bounded window operations
//! - Real-time processing support
//! - Late data handling and out-of-order processing
//!
//! ### Spatio-Temporal Integration
//! Temporal nodes integrate with spatial positioning:
//! - Combined temporal-spatial analysis
//! - Movement pattern detection in hyperbolic space
//! - Spatial clustering of temporal sequences
//! - Trajectory analysis with hyperbolic metrics
//!
//! ## Mathematical Foundations
//!
//! Timeseries AST nodes are grounded in temporal analysis theory:
//!
//! ### Time Series Analysis
//! Classical time series concepts:
//! - **Stationarity**: Time-invariant statistical properties
//! - **Autocorrelation**: Self-similarity across time lags
//! - **Seasonality**: Periodic patterns in data
//! - **Trend Analysis**: Long-term directional changes
//!
//! ### Window Mathematics
//! Mathematical foundations for window operations:
//! - **Sliding Windows**: Overlapping time intervals
//! - **Tumbling Windows**: Non-overlapping fixed intervals
//! - **Session Windows**: Activity-based dynamic intervals
//! - **Exponential Windows**: Exponentially decaying weights
//!
//! ### Temporal Statistics
//! Statistical measures for time series:
//! - **Moving Averages**: MA(t) = (1/k) Σᵢ₌₀ᵏ⁻¹ x(t-i)
//! - **Exponential Smoothing**: S(t) = αx(t) + (1-α)S(t-1)
//! - **Temporal Variance**: Var(t) = E[(X(t) - μ(t))²]
//! - **Covariance Functions**: Cov(s,t) = E[(X(s)-μ(s))(X(t)-μ(t))]
//!
//! ## Core AST Node Types
//!
//! The timeseries AST includes several categories of nodes:
//!
//! ### Window Nodes
//! Nodes for window-based operations:
//! - **SlidingWindowNode**: Overlapping window operations
//! - **TumblingWindowNode**: Non-overlapping window operations
//! - **SessionWindowNode**: Activity-based window detection
//! - **LandmarkWindowNode**: Fixed-start window operations
//!
//! ### Temporal Nodes
//! Nodes for time-based operations:
//! - **TemporalFilterNode**: Time-based filtering expressions
//! - **TemporalJoinNode**: Time-aware join operations
//! - **LagLeadNode**: Time-shifted data access
//! - **TemporalUnionNode**: Time-ordered result merging
//!
//! ### Aggregation Nodes
//! Nodes for temporal aggregations:
//! - **TemporalSumNode**: Time-bucketed summation
//! - **MovingAverageNode**: Moving average computation
//! - **TemporalCountNode**: Time-based counting
//! - **TemporalMinMaxNode**: Time-windowed extrema
//!
//! ### Pattern Nodes
//! Nodes for temporal pattern detection:
//! - **SequencePatternNode**: Temporal sequence matching
//! - **TrendDetectionNode**: Trend pattern identification
//! - **SeasonalityNode**: Periodic pattern analysis
//! - **AnomalyDetectionNode**: Outlier identification
//!
//! ## Performance Features
//!
//! Timeseries AST nodes include performance optimizations:
//!
//! ### Streaming Optimization
//! - Incremental computation for window operations
//! - Memory-bounded processing for large time series
//! - Late data handling with watermarks
//! - Out-of-order processing with buffering
//!
//! ### Index Integration
//! - Time-based index utilization for range queries
//! - Temporal clustering for efficient access
//! - Multi-resolution indices for different granularities
//! - Compound indices for spatio-temporal queries
//!
//! ### Parallel Processing
//! - Partition-based parallel processing
//! - Pipeline parallelism for streaming operations
//! - Work distribution across time ranges
//! - NUMA-aware memory placement for large datasets
//!
//! ## Integration Features
//!
//! Timeseries AST nodes integrate with other system components:
//!
//! ### Spatial Integration
//! - Combined temporal-spatial analysis
//! - Movement trajectory analysis in hyperbolic space
//! - Spatial clustering of temporal patterns
//! - Geographic time series analysis
//!
//! ### Vector Integration
//! - Time series of vector embeddings
//! - Temporal similarity analysis
//! - Dynamic vector clustering over time
//! - Embedding trajectory analysis
//!
//! ### Graph Integration
//! - Temporal graph analysis
//! - Dynamic relationship patterns
//! - Time-evolving network properties
//! - Temporal centrality measures
//!
//! ## Module Organization
//!
//! The timeseries AST module is organized by functionality:
//!
//! - [`window`]: Window operation AST nodes
//! - [`temporal`]: Temporal operator AST nodes
//!
//! ## Usage Examples
//!
//! Timeseries AST nodes enable sophisticated temporal queries:
//!
//! ```hyperql
//! -- Moving average with spatial clustering
//! SELECT time_bucket('1 hour', timestamp) as hour,
//!        spatial_cluster,
//!        avg(value) OVER (
//!          PARTITION BY spatial_cluster 
//!          ORDER BY timestamp 
//!          ROWS BETWEEN 23 PRECEDING AND CURRENT ROW
//!        ) as moving_avg_24h
//! FROM sensor_data
//! WHERE timestamp >= now() - interval '7 days'
//!   AND hyperbolic_distance(position, @center) < 5.0
//! GROUP BY hour, hyperbolic_cluster(position, 2.0)
//! ORDER BY hour, spatial_cluster;
//!
//! -- Temporal pattern detection with trajectory analysis
//! SELECT entity_id,
//!        detect_trend(values ORDER BY timestamp) as trend,
//!        trajectory_similarity(positions ORDER BY timestamp, @reference_path) as path_sim
//! FROM time_series_data
//! WHERE timestamp BETWEEN @start_time AND @end_time
//! GROUP BY entity_id
//! HAVING trend = 'increasing' AND path_sim > 0.7
//! ORDER BY path_sim DESC;
//! ```
//!
//! This comprehensive timeseries AST enables HyperQL to express complex
//! temporal queries while leveraging hyperbolic positioning for enhanced
//! spatio-temporal analysis and pattern recognition.

pub mod window;
pub mod temporal;