//! Private versioned source image for a Node-free v2 runner.
//!
//! This is a source-backed encoding, not an executable IR codec or stable ABI.
//! The bounded compiler validates it again before invocation; no host authority
//! is encoded, inherited from v1, or supplied by the embedded source.
use super::*;
use crate::compiler::execution::{
    ExecutionControl, ExecutionLimits, MaterializationLimits, RuntimePolicy,
};

const MAGIC: &[u8] = b"SVRV2\0";
const HEADER: usize = 16;
const VERSION: u8 = 1;
const SOURCE_FEATURE: u32 = 1;
/// Maximum embedded UTF-8 source, independently of the frontend's own quotas.
pub const NATIVE_SOURCE_LIMIT: usize = 1024 * 1024;

/// Identify the v2 image family; decoding still validates every header field.
pub fn is_native_image(image: &[u8]) -> bool {
    image.starts_with(b"SVRV2")
}
/// Fixed finite command/standalone-runner policy; not process isolation.
pub fn command_runtime_policy() -> RuntimeVmPolicy {
    RuntimeVmPolicy {
        runtime: RuntimePolicy {
            execution: ExecutionLimits {
                max_instructions: Some(1_000_000),
                max_output_bytes: Some(1_048_576),
            },
            materialization: MaterializationLimits {
                max_value_nodes: Some(1_000_000),
                max_text_bytes: Some(16_777_216),
            },
        },
        control: ExecutionControl {
            cancellation: None,
            max_elapsed: Some(std::time::Duration::from_secs(30)),
        },
        ..RuntimeVmPolicy::DEFAULT
    }
}
fn entry(program: &CheckedProgramSource) -> Result<ProgramFunctionId, String> {
    let id = program
        .function_id("main")
        .ok_or("v2 executable requires main")?;
    let syntax = program
        .function(id)
        .ok_or("v2 main identity unavailable")?
        .typed()
        .callable()
        .function()
        .syntax();
    if !syntax.parameters.is_empty() || syntax.return_type.as_deref().unwrap_or("Unit") != "Unit" {
        return Err("v2 executable requires a parameterless Unit main".into());
    }
    Ok(id)
}
impl CheckedProgramSource {
    /// Validate all linked lowering before encoding this immutable checked source.
    /// The executable contains source and repeats checking/lowering at startup.
    /// No source path, host grants, environment or unchecked compiler options persist.
    pub fn encode_native_image(&self) -> Result<Vec<u8>, String> {
        if self.source().len() > NATIVE_SOURCE_LIMIT {
            return Err("v2 native source exceeds 1 MiB".into());
        }
        self.with_linked_ir(entry(self)?, ProgramLoweringLimits::DEFAULT, |_, _, _| {
            Ok(())
        })
        .map_err(|error| error.to_string())?;
        let mut image = Vec::with_capacity(HEADER + self.source().len());
        image.extend_from_slice(MAGIC);
        image.push(VERSION);
        image.push(VALUE_MODEL_V2 as u8);
        image.extend_from_slice(&SOURCE_FEATURE.to_le_bytes());
        image.extend_from_slice(&(self.source().len() as u32).to_le_bytes());
        image.extend_from_slice(self.source().as_bytes());
        Ok(image)
    }
}
/// Decode exact, bounded UTF-8 source with explicit version/model/feature refusal.
/// Recheck every body with finite compiler quotas; return no partial program.
/// The header detects representation mistakes, not malicious source tampering.
pub fn decode_native_image(image: &[u8]) -> Result<CheckedProgramSource, String> {
    if image.len() < HEADER || image.len() > HEADER + NATIVE_SOURCE_LIMIT {
        return Err("invalid v2 native image length".into());
    }
    if &image[..MAGIC.len()] != MAGIC || image[6] != VERSION {
        return Err("unsupported v2 native image version".into());
    }
    if image[7] != VALUE_MODEL_V2 as u8 {
        return Err("unsupported v2 native value model".into());
    }
    let features = u32::from_le_bytes(image[8..12].try_into().expect("fixed header"));
    if features != SOURCE_FEATURE {
        return Err("unsupported v2 native image features".into());
    }
    let length = u32::from_le_bytes(image[12..16].try_into().expect("fixed header")) as usize;
    if length != image.len() - HEADER {
        return Err("v2 native source length mismatch".into());
    }
    let source = std::str::from_utf8(&image[HEADER..]).map_err(|_| "invalid v2 native UTF-8")?;
    let program = check_program_source("<embedded-v2>", source, ProgramSourceLimits::DEFAULT)
        .map_err(|error| error.to_string())?;
    entry(&program)?;
    Ok(program)
}
/// Execute a decoded source image without input/file services, sharing the CLI's
/// finite profile and captured output/status. Arguments are caller-supplied data.
pub fn execute_native_image(
    image: &[u8],
    arguments: &[String],
) -> Result<RuntimeVmCommandOutcome, String> {
    if arguments.len() > 1024
        || arguments
            .iter()
            .try_fold(0usize, |sum, arg| sum.checked_add(arg.len()))
            .filter(|sum| *sum <= 65_536)
            .is_none()
    {
        return Err("v2 native arguments exceed 1024 entries or 64 KiB UTF-8".into());
    }
    let program = decode_native_image(image)?;
    program
        .execute_with_services(
            entry(&program)?,
            &[],
            arguments,
            RuntimeVmServices::default(),
            ProgramLoweringLimits::DEFAULT,
            command_runtime_policy(),
        )
        .map_err(|error| error.to_string())
}
