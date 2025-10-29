//! # Security Sandbox - Runtime Isolation and Resource Management
//!
//! The sandbox module provides security types and resource limit definitions.
//!
//! ## Sandboxing Provided by Embedding Application
//!
//! HyperQL is designed as an embeddable query language. Security sandboxing is provided
//! by the embedding application (e.g., Hyperspatial), not by HyperQL itself.
//!
//! When HyperQL is embedded in Hyperspatial:
//! - Lua sandbox uses mlua with removed dangerous globals (io, os, etc.)
//! - WASM sandbox uses Wasmtime with fuel metering and memory limits
//! - Resource limits are enforced by the compute engines
//!
//! This module defines the types and limits that the embedding application uses.

use super::{RuntimeResult, RuntimeError, RuntimeConfig};
use std::time::{Duration, Instant};

/// Security sandbox for runtime execution
pub struct SecuritySandbox {
    /// Resource limits configuration
    limits: ResourceLimits,
    /// Current resource usage tracking
    usage: ResourceUsage,
    /// Security policies
    policies: SecurityPolicies,
    /// Execution monitoring
    monitor: ExecutionMonitor,
}

/// Resource limits for sandbox execution
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory allocation in bytes
    pub max_memory_bytes: usize,
    /// Maximum execution time
    pub max_execution_time: Duration,
    /// Maximum CPU usage percentage
    pub max_cpu_percent: f64,
    /// Maximum number of function calls
    pub max_function_calls: u32,
    /// Maximum recursion depth
    pub max_recursion_depth: u32,
    /// Maximum file descriptors
    pub max_file_descriptors: u32,
    /// Maximum network connections
    pub max_network_connections: u32,
}

/// Current resource usage tracking
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// Current memory usage in bytes
    pub memory_bytes: usize,
    /// Peak memory usage in bytes
    pub peak_memory_bytes: usize,
    /// Execution start time
    pub execution_start: Option<Instant>,
    /// Total execution time
    pub total_execution_time: Duration,
    /// Number of function calls made
    pub function_calls: u32,
    /// Current recursion depth
    pub recursion_depth: u32,
    /// Open file descriptors count
    pub open_file_descriptors: u32,
    /// Active network connections
    pub network_connections: u32,
}

/// Security policies for function execution
#[derive(Debug, Clone)]
pub struct SecurityPolicies {
    /// Whether file system access is allowed
    pub allow_file_access: bool,
    /// Whether network access is allowed
    pub allow_network_access: bool,
    /// Whether system calls are allowed
    pub allow_system_calls: bool,
    /// Allowed file system paths (if file access enabled)
    pub allowed_paths: Vec<String>,
    /// Allowed network hosts (if network access enabled)
    pub allowed_hosts: Vec<String>,
    /// Allowed system calls (if system calls enabled)
    pub allowed_syscalls: Vec<String>,
    /// Whether debugging features are enabled
    pub allow_debugging: bool,
}

/// Execution monitoring and intrusion detection
#[derive(Debug, Clone, Default)]
pub struct ExecutionMonitor {
    /// Number of security violations detected
    pub violations_count: u32,
    /// Types of violations encountered
    pub violation_types: Vec<ViolationType>,
    /// Resource usage warnings issued
    pub warnings_count: u32,
    /// Last health check timestamp
    pub last_health_check: Option<Instant>,
}

/// Types of security violations
#[derive(Debug, Clone, PartialEq)]
pub enum ViolationType {
    /// Attempted unauthorized file access
    UnauthorizedFileAccess(String),
    /// Attempted unauthorized network access
    UnauthorizedNetworkAccess(String),
    /// Attempted forbidden system call
    ForbiddenSystemCall(String),
    /// Resource limit exceeded
    ResourceLimitExceeded(ResourceType),
    /// Suspicious execution pattern
    SuspiciousPattern(String),
    /// Code injection attempt
    CodeInjection,
}

/// Resource types for limit tracking
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceType {
    Memory,
    ExecutionTime,
    CpuUsage,
    FunctionCalls,
    RecursionDepth,
    FileDescriptors,
    NetworkConnections,
}

/// Sandbox execution context
pub struct SandboxContext {
    /// Unique execution ID
    pub execution_id: String,
    /// Function being executed
    pub function_name: String,
    /// Runtime type (Lua or WASM)
    pub runtime_type: RuntimeType,
    /// Execution start time
    pub start_time: Instant,
    /// Parent query context (if any)
    pub query_context: Option<String>,
}

/// Runtime type enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeType {
    Lua,
    Wasm,
    Native,
}

impl SecuritySandbox {
    /// Create a new security sandbox with the given configuration
    pub fn new(config: RuntimeConfig) -> Self {
        let limits = ResourceLimits {
            max_memory_bytes: config.max_memory_bytes,
            max_execution_time: Duration::from_millis(config.max_execution_time_ms),
            max_cpu_percent: 80.0,
            max_function_calls: 10_000,
            max_recursion_depth: config.max_stack_depth as u32,
            max_file_descriptors: 0,
            max_network_connections: 0,
        };

        let policies = SecurityPolicies {
            allow_file_access: false,
            allow_network_access: false,
            allow_system_calls: false,
            allowed_paths: Vec::new(),
            allowed_hosts: Vec::new(),
            allowed_syscalls: Vec::new(),
            allow_debugging: false,
        };

        Self {
            limits,
            usage: ResourceUsage::default(),
            policies,
            monitor: ExecutionMonitor::default(),
        }
    }

    /// Begin sandboxed execution
    ///
    /// Sandbox enforcement is handled by the embedding application.
    pub fn begin_execution(&mut self, context: SandboxContext) -> RuntimeResult<SandboxGuard<'_>> {
        self.usage.execution_start = Some(Instant::now());
        self.usage.function_calls = 0;
        self.usage.recursion_depth = 0;

        Ok(SandboxGuard {
            sandbox: self,
            context,
        })
    }

    /// Check if resource limits are being respected
    pub fn check_resource_limits(&mut self) -> RuntimeResult<()> {
        if self.usage.memory_bytes > self.limits.max_memory_bytes {
            self.record_violation(ViolationType::ResourceLimitExceeded(ResourceType::Memory));
            return Err(RuntimeError::ResourceLimitExceeded(
                format!("Memory limit exceeded: {} > {}",
                    self.usage.memory_bytes, self.limits.max_memory_bytes)
            ));
        }

        if let Some(start_time) = self.usage.execution_start {
            let elapsed = start_time.elapsed();
            if elapsed > self.limits.max_execution_time {
                self.record_violation(ViolationType::ResourceLimitExceeded(ResourceType::ExecutionTime));
                return Err(RuntimeError::ResourceLimitExceeded(
                    format!("Execution time limit exceeded: {:?} > {:?}",
                        elapsed, self.limits.max_execution_time)
                ));
            }
        }

        Ok(())
    }

    /// Record a security violation
    fn record_violation(&mut self, violation: ViolationType) {
        self.monitor.violations_count += 1;
        self.monitor.violation_types.push(violation);
    }

    /// Update resource usage statistics
    pub fn update_memory_usage(&mut self, new_usage: usize) {
        self.usage.memory_bytes = new_usage;
        if new_usage > self.usage.peak_memory_bytes {
            self.usage.peak_memory_bytes = new_usage;
        }
    }

    /// Get current resource usage statistics
    pub fn get_usage_stats(&self) -> &ResourceUsage {
        &self.usage
    }

    /// Get security violation summary
    pub fn get_violation_summary(&self) -> ViolationSummary {
        ViolationSummary {
            total_violations: self.monitor.violations_count,
            violation_types: self.monitor.violation_types.clone(),
            warnings_issued: self.monitor.warnings_count,
        }
    }
}

/// RAII guard for sandbox execution
pub struct SandboxGuard<'a> {
    sandbox: &'a mut SecuritySandbox,
    context: SandboxContext,
}

impl<'a> SandboxGuard<'a> {
    /// Get the execution context
    pub fn context(&self) -> &SandboxContext {
        &self.context
    }

    /// Check resource limits during execution
    pub fn check_limits(&mut self) -> RuntimeResult<()> {
        self.sandbox.check_resource_limits()
    }
}

impl<'a> Drop for SandboxGuard<'a> {
    fn drop(&mut self) {
        if let Some(start_time) = self.sandbox.usage.execution_start {
            self.sandbox.usage.total_execution_time += start_time.elapsed();
            self.sandbox.usage.execution_start = None;
        }
    }
}

/// Summary of security violations
#[derive(Debug, Clone)]
pub struct ViolationSummary {
    pub total_violations: u32,
    pub violation_types: Vec<ViolationType>,
    pub warnings_issued: u32,
}
