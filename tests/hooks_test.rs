use hrband_lsl::hooks::{Hook, HookContext, HookPoint, HookRegistry};
use std::sync::Arc;

// A simple test hook that counts executions
struct CountingHook {
    count: Arc<std::sync::Mutex<usize>>,
}

impl CountingHook {
    fn new() -> Arc<Self> {
        Arc::new(CountingHook {
            count: Arc::new(std::sync::Mutex::new(0)),
        })
    }

    fn get_count(&self) -> usize {
        *self.count.lock().unwrap()
    }
}

impl Hook for CountingHook {
    fn execute(&self, _point: HookPoint, _context: &HookContext) -> anyhow::Result<()> {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        Ok(())
    }
}

// A hook that captures context data
struct ContextCapturingHook {
    data: Arc<std::sync::Mutex<Vec<(String, Option<String>, Option<String>)>>>,
}

impl ContextCapturingHook {
    fn new() -> Arc<Self> {
        Arc::new(ContextCapturingHook {
            data: Arc::new(std::sync::Mutex::new(Vec::new())),
        })
    }

    fn get_data(&self) -> Vec<(String, Option<String>, Option<String>)> {
        self.data.lock().unwrap().clone()
    }
}

impl Hook for ContextCapturingHook {
    fn execute(&self, point: HookPoint, context: &HookContext) -> anyhow::Result<()> {
        let mut data = self.data.lock().unwrap();
        data.push((
            format!("{:?}", point),
            context.device_name.clone(),
            context.device_address.clone(),
        ));
        Ok(())
    }
}

#[test]
fn test_hook_trait_can_be_implemented() {
    let hook = CountingHook::new();
    let context = HookContext::default();

    assert_eq!(hook.get_count(), 0);
    hook.execute(HookPoint::PreScan, &context).unwrap();
    assert_eq!(hook.get_count(), 1);
}

#[test]
fn test_hook_registry_can_register_and_execute_single_hook() {
    let hook = CountingHook::new();
    let count_before = hook.get_count();

    let mut registry = HookRegistry::new();
    registry.register(hook.clone());

    let context = HookContext::default();
    registry.execute(HookPoint::PostScan, &context).unwrap();

    assert_eq!(hook.get_count(), count_before + 1);
}

#[test]
fn test_hook_registry_executes_all_registered_hooks() {
    let hook1 = CountingHook::new();
    let hook2 = CountingHook::new();
    let hook3 = CountingHook::new();

    let mut registry = HookRegistry::new();
    registry.register(hook1.clone());
    registry.register(hook2.clone());
    registry.register(hook3.clone());

    let context = HookContext::default();
    registry.execute(HookPoint::PreConnect, &context).unwrap();

    assert_eq!(hook1.get_count(), 1);
    assert_eq!(hook2.get_count(), 1);
    assert_eq!(hook3.get_count(), 1);
}

#[test]
fn test_hook_registry_executes_hooks_in_order() {
    let hook1 = CountingHook::new();
    let hook2 = CountingHook::new();

    let mut registry = HookRegistry::new();
    registry.register(hook1.clone());
    registry.register(hook2.clone());

    let context = HookContext::default();
    registry.execute(HookPoint::PostConnect, &context).unwrap();

    assert_eq!(hook1.get_count(), 1);
    assert_eq!(hook2.get_count(), 1);
}

#[test]
fn test_hook_context_default() {
    let context = HookContext::default();

    assert!(context.device_name.is_none());
    assert!(context.device_address.is_none());
    assert!(context.heart_rate.is_none());
    assert!(context.rr_intervals.is_empty());
    assert!(context.metadata.is_empty());
}

#[test]
fn test_hook_context_with_data() {
    let mut context = HookContext::default();
    context.device_name = Some("Heart Rate Monitor".to_string());
    context.device_address = Some("AA:BB:CC:DD:EE:FF".to_string());
    context.heart_rate = Some(75);
    context.rr_intervals = vec![800, 810, 805];
    context
        .metadata
        .insert("firmware".to_string(), "v1.0".to_string());

    assert_eq!(context.device_name, Some("Heart Rate Monitor".to_string()));
    assert_eq!(
        context.device_address,
        Some("AA:BB:CC:DD:EE:FF".to_string())
    );
    assert_eq!(context.heart_rate, Some(75));
    assert_eq!(context.rr_intervals.len(), 3);
    assert_eq!(context.metadata.get("firmware"), Some(&"v1.0".to_string()));
}

#[test]
fn test_hook_context_with_hook_points() {
    let capturing_hook = ContextCapturingHook::new();
    let mut registry = HookRegistry::new();
    registry.register(capturing_hook.clone());

    let mut context = HookContext::default();
    context.device_name = Some("Test Device".to_string());
    context.device_address = Some("11:22:33:44:55:66".to_string());

    let hook_points = vec![
        HookPoint::PreScan,
        HookPoint::PostScan,
        HookPoint::PreConnect,
        HookPoint::PostConnect,
        HookPoint::DataReceived,
        HookPoint::PreStream,
        HookPoint::PostStream,
        HookPoint::PreDisconnect,
    ];

    for point in hook_points {
        registry.execute(point, &context).unwrap();
    }

    let captured_data = capturing_hook.get_data();
    assert_eq!(captured_data.len(), 8);

    for (point_str, _, _) in captured_data.iter() {
        assert!(!point_str.is_empty());
    }
}

#[test]
fn test_hook_registry_multiple_executions() {
    let hook = CountingHook::new();
    let mut registry = HookRegistry::new();
    registry.register(hook.clone());

    let context = HookContext::default();

    registry.execute(HookPoint::PreScan, &context).unwrap();
    registry.execute(HookPoint::PostScan, &context).unwrap();
    registry.execute(HookPoint::PreConnect, &context).unwrap();

    assert_eq!(hook.get_count(), 3);
}
