//! Explicit bounded external policy dispatch; no grants are loaded from program images.
use super::{
    parse_json, CompiledGenericPackage, GenericVmOutcome, GenericVmPolicy, HostError, HostV3,
    JsonLimits, JsonValue as J, NetworkHostV4, NetworkLimits,
};
use crate::compiler::diagnostics::Diagnostics;
use std::{fs::OpenOptions, io::Read, path::Path};
fn error(message: &'static str) -> HostError {
    HostError {
        code: "policy",
        message: message.into(),
    }
}
/// Explicit execution authority selected by an external trusted policy version.
#[derive(Debug)]
pub enum DataHostPolicy {
    /// Existing version-three filesystem/process/time authority.
    V3(HostV3),
    /// Version-four execution-scoped loopback authority, separate from v3 services.
    Network(NetworkHostV4),
}
impl DataHostPolicy {
    /// Load one regular policy file, refusing oversized input before parsing.
    /// Linux opens are nonblocking/no-follow; Windows opens reject reparse files.
    pub fn load_policy(path: &Path) -> Result<Self, HostError> {
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(
                0x800
                    | if cfg!(target_arch = "aarch64") {
                        0x8000
                    } else {
                        0x20000
                    },
            );
        }
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(0x00200000);
        }
        let file = options
            .open(path)
            .map_err(|_| error("cannot open host policy"))?;
        let metadata = file
            .metadata()
            .map_err(|_| error("cannot inspect host policy"))?;
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(error("policy reparse points refuse"));
            }
        }
        if !metadata.is_file() || metadata.len() > 65536 {
            return Err(error(
                "host policy must be a regular file of at most 64 KiB",
            ));
        }
        let mut bytes = Vec::new();
        file.take(65537)
            .read_to_end(&mut bytes)
            .map_err(|_| error("cannot read host policy"))?;
        Self::from_policy(&bytes)
    }
    /// Validate a complete policy document; versions never select a fallback.
    pub fn from_policy(bytes: &[u8]) -> Result<Self, HostError> {
        if bytes.len() > 65536 {
            return Err(error("host policy exceeds 64 KiB"));
        }
        let value = parse_json(
            bytes,
            JsonLimits {
                bytes: 65536,
                values: 4096,
                ..JsonLimits::DEFAULT
            },
        )
        .map_err(|_| error("invalid host policy JSON"))?;
        let fields = match &value {
            J::Object(fields) => fields,
            _ => return Err(error("host policy must be an object")),
        };
        let version = fields.iter().find(|(k, _)| k == "version").map(|(_, v)| v);
        match version {
            Some(J::Int(3)) => HostV3::from_policy(bytes).map(Self::V3),
            Some(J::Int(4)) => network_policy(fields).map(Self::Network),
            _ => Err(error("host policy version must be 3 or 4")),
        }
    }
    /// Execute using exactly this external authority; network resources close on exit.
    pub fn execute(
        self,
        program: &CompiledGenericPackage,
        policy: GenericVmPolicy,
    ) -> Result<GenericVmOutcome, Diagnostics> {
        match self {
            Self::V3(host) => super::execute_generic_data_package_with_host(program, policy, &host),
            Self::Network(host) => {
                super::execute_generic_data_package_with_network_host(program, policy, host)
            }
        }
    }
}
fn number(value: &J) -> Result<u64, HostError> {
    match value {
        J::Int(n) => {
            u64::try_from(*n).map_err(|_| error("network policy limits must be nonnegative"))
        }
        _ => Err(error("network policy limits must be Int")),
    }
}
fn network_policy(fields: &[(String, J)]) -> Result<NetworkHostV4, HostError> {
    let mut limits = NetworkLimits::DEFAULT;
    let mut ports = Vec::new();
    let mut http = super::HttpExchangeLimits::DEFAULT;
    for (key, value) in fields {
        match key.as_str() {
            "version" => {}
            "listen_ports" => {
                let entries = match value {
                    J::Array(entries) if entries.len() <= 16 => entries,
                    _ => return Err(error("listen_ports must be an array of at most 16 ports")),
                };
                for entry in entries {
                    let port = u16::try_from(number(entry)?)
                        .map_err(|_| error("network port must be in 0..65535"))?;
                    if ports.contains(&port) {
                        return Err(error("duplicate network port grant"));
                    }
                    ports.push(port);
                }
            }
            "http" => {
                let entries = match value {
                    J::Object(entries) if entries.len() <= 8 => entries,
                    _ => return Err(error("HTTP limits must be a bounded object")),
                };
                for (key, value) in entries {
                    let n = number(value)?;
                    if key == "milliseconds" {
                        http.milliseconds = n;
                        continue;
                    }
                    let n = usize::try_from(n).map_err(|_| error("HTTP limit overflow"))?;
                    match key.as_str() {
                        "io_operations" => http.io_operations = n,
                        "head_bytes" => {
                            http.request.head_bytes = n;
                            http.response.head_bytes = n;
                        }
                        "line_bytes" => {
                            http.request.line_bytes = n;
                            http.response.line_bytes = n;
                        }
                        "headers" => {
                            http.request.headers = n;
                            http.response.headers = n;
                        }
                        "body_bytes" => {
                            http.request.body_bytes = n;
                            http.response.body_bytes = n;
                        }
                        "response_bytes" => http.response.output_bytes = n,
                        "work" => {
                            http.request.work = n;
                            http.response.work = n;
                        }
                        _ => return Err(error("unknown HTTP limit")),
                    }
                }
            }
            "limits" => {
                let entries = match value {
                    J::Object(entries) if entries.len() <= 4 => entries,
                    _ => return Err(error("network limits must be a bounded object")),
                };
                for (key, value) in entries {
                    let n = number(value)?;
                    match key.as_str() {
                        "listeners" => {
                            limits.listeners =
                                usize::try_from(n).map_err(|_| error("listener limit overflow"))?
                        }
                        "streams" => {
                            limits.streams =
                                usize::try_from(n).map_err(|_| error("stream limit overflow"))?
                        }
                        "bytes" => {
                            limits.bytes =
                                usize::try_from(n).map_err(|_| error("byte limit overflow"))?
                        }
                        "milliseconds" => limits.milliseconds = n,
                        _ => return Err(error("unknown network limit")),
                    }
                }
            }
            _ => return Err(error("unknown version-four policy field")),
        }
    }
    let mut host = NetworkHostV4::new(limits)?;
    host.set_http_limits(http)?;
    for port in ports {
        host.grant_port(port)?;
    }
    Ok(host)
}
