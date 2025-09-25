//! # Timeseries-Specific Functions
//!
//! This module provides a comprehensive collection of time series analysis
//! functions for HyperQL queries. These functions enable temporal pattern
//! detection, window operations, and statistical analysis over time-ordered
//! data while integrating with hyperbolic positioning for spatio-temporal queries.
//!
//! ## Purpose
//!
//! The timeseries functions module serves multiple purposes:
//! - Provides window-based aggregation and analysis functions
//! - Implements temporal pattern detection and forecasting
//! - Enables time-based filtering and transformation operations
//! - Supports statistical analysis over temporal sequences
//! - Integrates temporal operations with spatial positioning
//!
//! ## Function Categories
//!
//! Timeseries functions are organized into specialized categories:
//!
//! ### Window Functions
//! Functions for time-based window operations:
//! - **moving_average()**: Rolling average computations
//! - **exponential_smoothing()**: Exponentially weighted averages
//! - **window_aggregate()**: General window-based aggregations
//! - **lag()** / **lead()**: Time-shifted data access
//! - **first_value()** / **last_value()**: Window boundary values
//!
//! ### Temporal Functions
//! Functions for time-based operations:
//! - **time_bucket()**: Time interval grouping
//! - **time_diff()**: Time difference calculations
//! - **extract()**: Component extraction from timestamps
//! - **date_trunc()**: Timestamp truncation to intervals
//! - **timezone_convert()**: Time zone conversions
//!
//! ## Mathematical Foundations
//!
//! Timeseries functions are grounded in temporal analysis theory:
//!
//! ### Time Series Analysis
//! Mathematical foundations for temporal data:
//! - **Stationarity**: Time-invariant statistical properties
//! - **Autocorrelation**: Self-similarity across time lags
//! - **Seasonality**: Periodic patterns and decomposition
//! - **Trend Analysis**: Long-term directional changes
//!
//! ### Statistical Methods
//! Statistical techniques for time series:
//! - **Moving Averages**: MA(t) = (1/k) Σᵢ₌₀ᵏ⁻¹ x(t-i)
//! - **Exponential Smoothing**: S(t) = αx(t) + (1-α)S(t-1)
//! - **Autoregression**: AR model parameter estimation
//! - **Fourier Analysis**: Frequency domain transformations
//!
//! ## Module Organization
//!
//! The timeseries functions module is organized by functionality:
//!
//! - [`window`]: Window-based function implementations
//! - [`temporal`]: Time-based operation functions
//!
//! This organization enables efficient temporal analysis while supporting
//! both streaming and batch processing scenarios.

pub mod window;
pub mod temporal;