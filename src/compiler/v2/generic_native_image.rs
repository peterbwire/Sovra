//! Versioned generic source images for host-free native runners.
use super::*;
use crate::compiler::diagnostics::diagnostic_message;
const MAGIC: &[u8] = b"SVRG2\0";
const HEADER: usize = 16;
const VERSION: u8 = 1;
const BASE_FEATURES: u32 = 1;
const ENUM_FEATURES: u32 = 3;
const FEATURES: u32 = 7;
const PROPAGATION_FEATURES: u32 = 15;
const ADAPTER_FEATURES: u32 = 31;
const BUILTIN_FEATURES: u32 = 63;
const STRING_FEATURES: u32 = 127;
const VALUE_FEATURES: u32 = 255;
const ARRAY_FEATURES: u32 = 511;
/// Independent generic native source ceiling, before UTF-8 decoding or compilation.
pub const GENERIC_NATIVE_SOURCE_LIMIT: usize = 1024 * 1024;
/// Recognize this image family; decoding still checks the complete header.
pub fn is_generic_native_image(image: &[u8]) -> bool {
    image.starts_with(b"SVRG2")
}
/// Shared finite generic command/native policy, with cooperative elapsed control.
pub fn generic_command_runtime_policy() -> GenericVmPolicy {
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.control.max_elapsed = Some(std::time::Duration::from_secs(30));
    policy
}
impl CompiledGenericSource {
    /// Require the command entry contract without executing any instruction.
    pub fn validate_command_entry(&self) -> Result<(), String> {
        if self.entry() != "main" {
            return Err("generic executable entry must be main".into());
        }
        let root = &self.ir().functions()[0];
        let layouts = self.ir().plans().layouts();
        if !matches!(
            layouts.layouts()[root.return_layout()].kind(),
            GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Unit)
        ) {
            return Err("generic execution requires a parameterless Unit main function".into());
        }
        Ok(())
    }
    /// Encode checked source for rechecking at startup, not a machine-code/IR codec.
    /// No source pathname, host grant, argument or caller-selected policy is retained.
    pub fn encode_generic_native_image(&self) -> Result<Vec<u8>, String> {
        if self.source().len() > GENERIC_NATIVE_SOURCE_LIMIT {
            return Err("generic native source exceeds 1 MiB".into());
        }
        self.validate_command_entry()?;
        let mut image = Vec::with_capacity(HEADER + self.source().len());
        image.extend_from_slice(MAGIC);
        image.push(VERSION);
        image.push(VALUE_MODEL_V2 as u8);
        let features = if self.has_standard_arrays() {
            ARRAY_FEATURES
        } else if self.has_standard_values() {
            VALUE_FEATURES
        } else if self.has_standard_strings() {
            STRING_FEATURES
        } else if self.has_standard_builtins() {
            BUILTIN_FEATURES
        } else if self.has_standard_results() {
            ADAPTER_FEATURES
        } else if self.propagation_ir().is_some() {
            PROPAGATION_FEATURES
        } else {
            FEATURES
        };
        image.extend_from_slice(&features.to_le_bytes());
        image.extend_from_slice(&(self.source().len() as u32).to_le_bytes());
        image.extend_from_slice(self.source().as_bytes());
        Ok(image)
    }
}
/// Validate exact bounded header/source and repeat the complete finite compiler pipeline.
/// The format is private and its header is not authentication of embedded source.
pub fn decode_generic_native_image(image: &[u8]) -> Result<CompiledGenericSource, String> {
    if image.len() < HEADER || image.len() > HEADER + GENERIC_NATIVE_SOURCE_LIMIT {
        return Err("invalid generic native image length".into());
    }
    if &image[..MAGIC.len()] != MAGIC || image[6] != VERSION {
        return Err("unsupported generic native image version".into());
    }
    if image[7] != VALUE_MODEL_V2 as u8 {
        return Err("unsupported generic native value model".into());
    }
    let features = u32::from_le_bytes(image[8..12].try_into().expect("fixed header"));
    if !matches!(
        features,
        BASE_FEATURES
            | ENUM_FEATURES
            | FEATURES
            | PROPAGATION_FEATURES
            | ADAPTER_FEATURES
            | BUILTIN_FEATURES
            | STRING_FEATURES
            | VALUE_FEATURES
            | ARRAY_FEATURES
    ) {
        return Err("unsupported generic native image features".into());
    }
    let length = u32::from_le_bytes(image[12..16].try_into().expect("fixed header")) as usize;
    if length != image.len() - HEADER {
        return Err("generic native source length mismatch".into());
    }
    let source =
        std::str::from_utf8(&image[HEADER..]).map_err(|_| "invalid generic native UTF-8")?;
    let checked = if features == ARRAY_FEATURES {
        compile_generic_array_source(source, "main", GenericPropagationSourceLimits::DEFAULT)
    } else if features == VALUE_FEATURES {
        compile_generic_value_source(source, "main", GenericPropagationSourceLimits::DEFAULT)
    } else if features == STRING_FEATURES {
        compile_generic_string_source(source, "main", GenericPropagationSourceLimits::DEFAULT)
    } else if features == BUILTIN_FEATURES {
        compile_generic_builtin_source(source, "main", GenericPropagationSourceLimits::DEFAULT)
    } else if features == ADAPTER_FEATURES {
        compile_generic_adapter_source(source, "main", GenericPropagationSourceLimits::DEFAULT)
    } else if features == PROPAGATION_FEATURES {
        compile_generic_propagation_source(source, "main", GenericPropagationSourceLimits::DEFAULT)
    } else {
        super::generic_source::compile_generic_source_profile(
            source,
            "main",
            GenericSourceLimits::DEFAULT,
            features & 2 != 0,
            features & 4 != 0,
        )
    }
    .map_err(|error| diagnostic_message!("{error}"))?;
    checked.validate_command_entry()?;
    Ok(checked)
}
/// Recheck and execute an embedded generic program under the fixed host-free policy.
/// There is deliberately no host-service or argument transport.
pub fn execute_generic_native_image(image: &[u8]) -> Result<GenericSourceOutcome, String> {
    let program = decode_generic_native_image(image)?;
    execute_generic_source(&program, generic_command_runtime_policy())
        .map_err(|error| diagnostic_message!("{error}"))
}
