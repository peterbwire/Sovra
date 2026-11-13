//! Opt-in logical execution fuel and cooperative Rust-host control.
//! These component limits are not heap, total-work, time or host-I/O sandboxes.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

/// Permanent cooperative cancellation shared by trusted supervising threads.
/// Cancelling a clone cancels all invocations using this token; there is no reset.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);
impl CancellationToken {
    /// Create a token which has not been cancelled.
    pub fn new() -> Self {
        Self::default()
    }
    /// Request cancellation at the next interpreter checkpoint.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    /// Observe whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Optional Rust-host cooperative controls, independent of logical fuel.
/// These cannot interrupt a helper traversal, cleanup or blocked host operation.
#[derive(Clone, Debug)]
pub struct ExecutionControl {
    /// Shared permanent cancellation request; None disables cancellation checks.
    pub cancellation: Option<CancellationToken>,
    /// Per-invocation monotonic elapsed allowance, including declaration validation.
    /// Zero refuses the first checkpoint. This is not a hard wall-clock deadline.
    pub max_elapsed: Option<Duration>,
}
impl ExecutionControl {
    /// Preserve earlier execution behavior without cooperative control.
    pub const UNLIMITED: Self = Self {
        cancellation: None,
        max_elapsed: None,
    };

    // Inject elapsed time privately so boundary tests do not depend on sleeps.
    fn check_with_elapsed(&self, elapsed: impl FnOnce() -> Duration) -> Result<(), String> {
        if self
            .cancellation
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(CANCELLED_ERROR.into());
        }
        if self.max_elapsed.is_some_and(|limit| elapsed() >= limit) {
            return Err(ELAPSED_ERROR.into());
        }
        Ok(())
    }
}

pub(crate) const CANCELLED_ERROR: &str = "execution cancelled";
pub(crate) const ELAPSED_ERROR: &str = "execution elapsed time allowance exceeded";

struct ActiveControl {
    settings: ExecutionControl,
    started: Option<Instant>,
    #[cfg(test)]
    elapsed_override: Option<Box<dyn Fn() -> Duration>>,
}
impl ActiveControl {
    fn new(settings: ExecutionControl) -> Self {
        let started = settings.max_elapsed.map(|_| Instant::now());
        Self {
            settings,
            started,
            #[cfg(test)]
            elapsed_override: None,
        }
    }
    fn checkpoint(&self) -> Result<(), String> {
        #[cfg(test)]
        if let Some(clock) = &self.elapsed_override {
            return self.settings.check_with_elapsed(clock);
        }
        self.settings.check_with_elapsed(|| {
            self.started
                .expect("elapsed control has an origin")
                .elapsed()
        })
    }
}

/// Per-invocation allowances shared across all user-function frames.
/// None is unlimited; zero refuses the next instruction/output byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionLimits {
    /// Maximum dispatched IR instructions, not work inside an instruction.
    pub max_instructions: Option<u64>,
    /// Maximum printed UTF-8 bytes, including one LF per line.
    pub max_output_bytes: Option<u64>,
}
impl ExecutionLimits {
    /// Preserve existing execution behavior without these optional allowances.
    pub const UNLIMITED: Self = Self {
        max_instructions: None,
        max_output_bytes: None,
    };
}

/// Version of the approved logical materialization cost/effect table.
pub const MATERIALIZATION_MODEL_VERSION: u32 = 1;

/// Cumulative logical materialization allowances, never refunded on release.
/// These units do not measure allocator capacity, scratch buffers or RSS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterializationLimits {
    /// New value nodes, including copied descendants and implicit Unit results.
    pub max_value_nodes: Option<u64>,
    /// New UTF-8 String/record-label/printed-line bytes; printed LF is separate.
    pub max_text_bytes: Option<u64>,
}
impl MaterializationLimits {
    /// No optional materialization limits.
    pub const UNLIMITED: Self = Self {
        max_value_nodes: None,
        max_text_bytes: None,
    };
}

/// Explicit composed runtime policy. Earlier ExecutionLimits APIs stay compatible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimePolicy {
    /// Optional instruction and printed-output allowances.
    pub execution: ExecutionLimits,
    /// Optional model-v1 cumulative node/text allowances.
    pub materialization: MaterializationLimits,
}
impl RuntimePolicy {
    /// Preserve existing policies without these optional component allowances.
    pub const UNLIMITED: Self = Self {
        execution: ExecutionLimits::UNLIMITED,
        materialization: MaterializationLimits::UNLIMITED,
    };
}

pub(crate) const NODE_LIMIT_ERROR: &str = "execution value node allowance exceeded";
pub(crate) const TEXT_LIMIT_ERROR: &str = "execution text materialization allowance exceeded";
pub(crate) const SIZE_ERROR: &str = "logical materialization size overflow";
pub(crate) fn text_bytes(text: &str) -> Result<u64, String> {
    u64::try_from(text.len()).map_err(|_| SIZE_ERROR.into())
}
pub(crate) fn add_bytes(left: u64, right: u64) -> Result<u64, String> {
    left.checked_add(right).ok_or_else(|| SIZE_ERROR.into())
}
pub(crate) fn label_bytes<'a>(
    kind: &str,
    mut fields: impl Iterator<Item = &'a str>,
) -> Result<u64, String> {
    fields.try_fold(text_bytes(kind)?, |total, name| {
        add_bytes(total, text_bytes(name)?)
    })
}

pub(crate) const INSTRUCTION_LIMIT_ERROR: &str = "execution instruction allowance exceeded";
pub(crate) const OUTPUT_LIMIT_ERROR: &str = "execution output byte allowance exceeded";

pub(crate) struct Budget {
    policy: RuntimePolicy,
    control: ActiveControl,
}
impl Budget {
    pub fn new(limits: RuntimePolicy) -> Self {
        Self {
            policy: limits,
            control: ActiveControl::new(ExecutionControl::UNLIMITED),
        }
    }
    pub fn with_control(limits: RuntimePolicy, control: ExecutionControl) -> Self {
        let mut budget = Self::new(limits);
        budget.control = ActiveControl::new(control);
        budget
    }
    pub fn checkpoint(&self) -> Result<(), String> {
        self.control.checkpoint()
    }
    #[cfg(test)]
    pub(crate) fn with_test_clock(
        limits: RuntimePolicy,
        control: ExecutionControl,
        clock: impl Fn() -> Duration + 'static,
    ) -> Self {
        let mut budget = Self::with_control(limits, control);
        budget.control.elapsed_override = Some(Box::new(clock));
        budget
    }
    pub fn materialization_enabled(&self) -> bool {
        self.policy.materialization.max_value_nodes.is_some()
            || self.policy.materialization.max_text_bytes.is_some()
    }
    pub fn preflight(&self, nodes: u64, text: u64) -> Result<(), String> {
        if self
            .policy
            .materialization
            .max_value_nodes
            .is_some_and(|remaining| nodes > remaining)
        {
            return Err(NODE_LIMIT_ERROR.into());
        }
        if self
            .policy
            .materialization
            .max_text_bytes
            .is_some_and(|remaining| text > remaining)
        {
            return Err(TEXT_LIMIT_ERROR.into());
        }
        Ok(())
    }
    pub fn materialize(&mut self, nodes: u64, text: u64) -> Result<(), String> {
        self.preflight(nodes, text)?;
        if let Some(remaining) = &mut self.policy.materialization.max_value_nodes {
            *remaining -= nodes;
        }
        if let Some(remaining) = &mut self.policy.materialization.max_text_bytes {
            *remaining -= text;
        }
        Ok(())
    }
    pub fn snapshot(&mut self, value: &super::interpreter::Value) -> Result<(), String> {
        if self.materialization_enabled() {
            let size = value.logical_footprint().ok_or(SIZE_ERROR)?;
            self.materialize(size.value_nodes, size.owned_text_bytes)?;
        }
        Ok(())
    }
    pub fn preflight_output(&self, payload: u64) -> Result<(), String> {
        if let Some(remaining) = self.policy.execution.max_output_bytes {
            if payload
                .checked_add(1)
                .map_or(true, |bytes| bytes > remaining)
            {
                return Err(OUTPUT_LIMIT_ERROR.into());
            }
        }
        Ok(())
    }
    pub fn instruction(&mut self) -> Result<(), String> {
        if let Some(remaining) = &mut self.policy.execution.max_instructions {
            *remaining = remaining.checked_sub(1).ok_or(INSTRUCTION_LIMIT_ERROR)?;
        }
        Ok(())
    }
    pub fn line_payload_limit(&self) -> Result<Option<usize>, String> {
        match self.policy.execution.max_output_bytes {
            Some(0) => Err(OUTPUT_LIMIT_ERROR.into()),
            Some(remaining) => Ok(usize::try_from(remaining - 1).ok()),
            None => Ok(None),
        }
    }
    pub fn output_size(&mut self, payload: u64) -> Result<(), String> {
        if let Some(remaining) = &mut self.policy.execution.max_output_bytes {
            let bytes = payload.checked_add(1).ok_or(OUTPUT_LIMIT_ERROR)?;
            *remaining = remaining.checked_sub(bytes).ok_or(OUTPUT_LIMIT_ERROR)?;
        }
        Ok(())
    }
    pub fn output(&mut self, payload: usize) -> Result<(), String> {
        self.output_size(u64::try_from(payload).map_err(|_| OUTPUT_LIMIT_ERROR)?)
    }
}

#[cfg(test)]
mod materialization_tests {
    use super::*;
    #[test]
    fn failed_two_dimension_admission_and_size_overflow_never_partially_charge() {
        let mut budget = Budget::new(RuntimePolicy {
            materialization: MaterializationLimits {
                max_value_nodes: Some(1),
                max_text_bytes: Some(1),
            },
            ..RuntimePolicy::UNLIMITED
        });
        assert_eq!(budget.materialize(1, 2).unwrap_err(), TEXT_LIMIT_ERROR);
        assert!(budget.materialize(1, 1).is_ok());
        assert_eq!(budget.materialize(1, 1).unwrap_err(), NODE_LIMIT_ERROR);
        assert_eq!(add_bytes(u64::MAX, 1).unwrap_err(), SIZE_ERROR);
        let mut maximum = Budget::new(RuntimePolicy {
            materialization: MaterializationLimits {
                max_value_nodes: Some(u64::MAX),
                max_text_bytes: Some(u64::MAX),
            },
            ..RuntimePolicy::UNLIMITED
        });
        assert!(maximum.materialize(u64::MAX, u64::MAX).is_ok());
        assert!(maximum.materialize(0, 0).is_ok());
        assert_eq!(maximum.materialize(0, 1).unwrap_err(), TEXT_LIMIT_ERROR);
    }
}

#[cfg(test)]
mod control_tests {
    use super::*;
    #[test]
    fn exact_elapsed_boundary_and_large_durations_use_checked_comparison() {
        let settings = ExecutionControl {
            cancellation: None,
            max_elapsed: Some(Duration::from_nanos(10)),
        };
        assert!(settings
            .check_with_elapsed(|| Duration::from_nanos(9))
            .is_ok());
        assert_eq!(
            settings
                .check_with_elapsed(|| Duration::from_nanos(10))
                .unwrap_err(),
            ELAPSED_ERROR
        );
        assert_eq!(
            settings
                .check_with_elapsed(|| Duration::from_nanos(11))
                .unwrap_err(),
            ELAPSED_ERROR
        );
        let maximum = ExecutionControl {
            max_elapsed: Some(Duration::MAX),
            ..ExecutionControl::UNLIMITED
        };
        assert!(maximum
            .check_with_elapsed(|| Duration::from_secs(1))
            .is_ok());
        assert_eq!(
            maximum.check_with_elapsed(|| Duration::MAX).unwrap_err(),
            ELAPSED_ERROR
        );
    }
    #[test]
    fn cancellation_priority_and_disabled_control_do_not_read_the_clock() {
        assert!(ExecutionControl::UNLIMITED
            .check_with_elapsed(|| panic!("disabled clock read"))
            .is_ok());
        let token = CancellationToken::new();
        let clone = token.clone();
        std::thread::spawn(move || {
            clone.cancel();
            clone.cancel();
        })
        .join()
        .unwrap();
        assert!(token.is_cancelled());
        let settings = ExecutionControl {
            cancellation: Some(token),
            max_elapsed: Some(Duration::ZERO),
        };
        assert_eq!(
            settings
                .check_with_elapsed(|| panic!("cancelled clock read"))
                .unwrap_err(),
            CANCELLED_ERROR
        );
    }
}
