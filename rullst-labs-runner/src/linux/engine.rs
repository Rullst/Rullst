use rullst_labs::{
    CaseOutput, ContentHash, ExecutionFailure, ExecutionLimits, MAX_WASM_BYTES, TrapKind,
    WorkerOutcome,
};
use wasmi::{
    CompilationMode, Config, EnforcedLimits, Engine, Linker, Module, Store, StoreLimitsBuilder,
};

/// Called only after the worker's OS probes and hard limits have succeeded.
/// Compilation/validation are inside that boundary too, before engine fuel can
/// govern execution. No unchecked module/native-cache deserialization is used.
pub(super) fn evaluate(
    bytes: &[u8],
    inputs: &[[i64; 2]],
    limits: &ExecutionLimits,
) -> WorkerOutcome {
    match run(bytes, inputs, limits) {
        Ok(cases) => WorkerOutcome::Executed {
            artifact: ContentHash::of(bytes),
            cases,
        },
        Err(failure) => WorkerOutcome::Rejected(failure),
    }
}
fn run(
    bytes: &[u8],
    inputs: &[[i64; 2]],
    limits: &ExecutionLimits,
) -> Result<Vec<CaseOutput>, ExecutionFailure> {
    if bytes.len() > MAX_WASM_BYTES || !bytes.starts_with(b"\0asm\x01\0\0\0") {
        return Err(ExecutionFailure::InvalidModule);
    }
    super::structure::validate(bytes, limits)?;
    let config = configuration();
    let engine = Engine::new(&config);
    let module = Module::new(&engine, bytes).map_err(|_| ExecutionFailure::InvalidModule)?;
    if module.imports().next().is_some() {
        return Err(ExecutionFailure::InvalidModule);
    }
    let memory = usize::try_from(limits.memory_pages())
        .map_err(|_| ExecutionFailure::InvalidModule)?
        .checked_mul(65536)
        .ok_or(ExecutionFailure::InvalidModule)?;
    let mut results = Vec::with_capacity(inputs.len());
    for input in inputs {
        let resources = StoreLimitsBuilder::new()
            .memory_size(memory)
            .table_elements(4096)
            .instances(1)
            .tables(1)
            .memories(1)
            .trap_on_grow_failure(true)
            .build();
        let mut store = Store::new(&engine, resources);
        store.limiter(|resources| resources);
        store
            .set_fuel(limits.fuel_per_case())
            .map_err(|_| ExecutionFailure::InvalidModule)?;
        let linker = Linker::new(&engine);
        let instance = linker
            .instantiate_and_start(&mut store, &module)
            .map_err(|_| ExecutionFailure::InvalidModule)?;
        let solve = instance
            .get_typed_func::<(i64, i64), i64>(&store, "solve")
            .map_err(|_| ExecutionFailure::InvalidModule)?;
        let result = match solve.call(&mut store, (input[0], input[1])) {
            Ok(value) => CaseOutput::Value(value),
            Err(error) => CaseOutput::Trap(match error.as_trap_code() {
                Some(wasmi::TrapCode::OutOfFuel) => TrapKind::Fuel,
                Some(wasmi::TrapCode::StackOverflow) => TrapKind::Stack,
                Some(
                    wasmi::TrapCode::MemoryOutOfBounds
                    | wasmi::TrapCode::GrowthOperationLimited
                    | wasmi::TrapCode::OutOfSystemMemory,
                ) => TrapKind::Memory,
                _ => TrapKind::Guest,
            }),
        };
        results.push(result);
    }
    Ok(results)
}

pub(super) fn configuration() -> Config {
    let mut config = Config::default();
    // Fixed constants satisfy the pinned engine's min/max stack precondition.
    config
        .set_min_stack_height(0)
        .set_max_stack_height(262144)
        .set_max_recursion_depth(128)
        .set_max_cached_stacks(0)
        .consume_fuel(true)
        .allow_start_fn(false)
        .ignore_custom_sections(true)
        .compilation_mode(CompilationMode::Eager)
        .enforced_limits(EnforcedLimits::strict())
        .wasm_multi_memory(false)
        .wasm_multi_value(false)
        .wasm_reference_types(true)
        .wasm_tail_call(false)
        .wasm_extended_const(false)
        .wasm_custom_page_sizes(false)
        .wasm_wide_arithmetic(false);
    config
}
