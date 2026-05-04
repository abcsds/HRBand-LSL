use crate::heart_rate::IntervalKind;
use std::collections::HashMap;
use std::sync::Arc;

/// HookPoint represents the different lifecycle points in the BLE-LSL system
/// where hooks can be registered and executed
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum HookPoint {
    /// Before BLE device scanning starts
    PreScan,
    /// After BLE device scanning completes
    PostScan,
    /// Before connecting to a BLE device
    PreConnect,
    /// After successfully connecting to a BLE device
    PostConnect,
    /// When data is received from the device
    DataReceived,
    /// Before streaming data to LSL
    PreStream,
    /// After streaming data to LSL
    PostStream,
    /// Before disconnecting from the device
    PreDisconnect,
}

/// HookContext contains information that is passed to hooks during execution
#[derive(Debug, Clone, Default)]
pub struct HookContext {
    /// Optional device name
    pub device_name: Option<String>,
    /// Optional device BLE address
    pub device_address: Option<String>,
    /// Optional current heart rate value
    pub heart_rate: Option<u8>,
    /// R-R interval values in **milliseconds**, populated only when streaming
    /// from an ECG-style band via the standard 0x2A37 RR-bit payload.
    pub rr_intervals: Vec<u16>,
    /// Peak-to-peak interval values in **milliseconds**, populated only when
    /// streaming from a PPG-style band via Polar PMD/PPI. PP is an
    /// approximation of RR, not the same physiological measurement —
    /// hooks that care about HRV should branch on `interval_kind`.
    pub pp_intervals: Vec<u16>,
    /// Which interval characteristic the active connection produced. `None`
    /// before the first frame; `Some(Rr)` or `Some(Pp)` once protocol
    /// selection commits.
    pub interval_kind: Option<IntervalKind>,
    /// Additional metadata as key-value pairs
    pub metadata: HashMap<String, String>,
}

/// Hook trait defines the interface for extensible hooks in the system
/// Implementations must be object-safe (Send + Sync)
pub trait Hook: Send + Sync {
    /// Execute the hook with the given lifecycle point and context
    ///
    /// # Arguments
    /// * `point` - The lifecycle point where this hook is executing
    /// * `context` - The context information for this execution
    ///
    /// # Errors
    /// Returns an error if hook execution fails
    fn execute(&self, point: HookPoint, context: &HookContext) -> anyhow::Result<()>;
}

/// HookRegistry manages registration and execution of multiple hooks.
///
/// Cloning is cheap — each hook is held behind an [`Arc`] — so the registry can
/// be shared into async closures by cloning it without copying the hook trait
/// objects themselves.
#[derive(Clone)]
pub struct HookRegistry {
    hooks: Vec<Arc<dyn Hook>>,
}

impl HookRegistry {
    /// Create a new empty hook registry
    pub fn new() -> Self {
        HookRegistry { hooks: Vec::new() }
    }

    /// Register a new hook
    ///
    /// # Arguments
    /// * `hook` - The hook to register (as an Arc<dyn Hook>)
    pub fn register(&mut self, hook: Arc<dyn Hook>) {
        self.hooks.push(hook);
    }

    /// Execute all registered hooks at the given hook point with the provided context
    ///
    /// Hooks are executed in the order they were registered. If any hook returns an error,
    /// the error is returned immediately and remaining hooks are not executed.
    ///
    /// # Arguments
    /// * `point` - The lifecycle point to execute hooks for
    /// * `context` - The context information to pass to hooks
    ///
    /// # Errors
    /// Returns an error if any hook execution fails
    pub fn execute(&self, point: HookPoint, context: &HookContext) -> anyhow::Result<()> {
        for hook in &self.hooks {
            hook.execute(point, context)?;
        }
        Ok(())
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_point_copy() {
        let point = HookPoint::PreScan;
        let point2 = point;
        assert_eq!(point, point2);
    }

    #[test]
    fn test_hook_context_default() {
        let context = HookContext::default();
        assert!(context.device_name.is_none());
        assert!(context.device_address.is_none());
        assert!(context.heart_rate.is_none());
        assert!(context.rr_intervals.is_empty());
        assert!(context.pp_intervals.is_empty());
        assert!(context.interval_kind.is_none());
        assert!(context.metadata.is_empty());
    }
}
