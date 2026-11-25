//! Explicit Linux handle-rooted host authority. No policy is installed by default.
use super::{parse_json, JsonLimits, JsonValue as J};
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use super::{RuntimeBytesLimits, RuntimeBytesV2, RuntimeDataWork, RuntimeValueFuel};
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use crate::compiler::execution::MaterializationLimits;
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Read,
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Component, PathBuf},
};
/// Stable host refusal. Effects already performed are not rolled back.
#[derive(Debug, Clone)]
pub struct HostError {
    /// Stable authority/input/io/unsupported/limit/cleanup category.
    pub code: &'static str,
    /// Bounded reason; cleanup failures remain explicit.
    pub message: String,
}
fn err(code: &'static str, message: impl Into<String>) -> HostError {
    HostError {
        code,
        message: message.into(),
    }
}
fn io(e: std::io::Error) -> HostError {
    err("io", e.to_string())
}
/// Lowerable independent host limits; logical VM counters do not sandbox host memory/processes.
#[derive(Debug, Clone, Copy)]
pub struct HostLimits {
    /// Bytes per file or stream.
    pub bytes: usize,
    /// Directory entries.
    pub entries: usize,
    /// Argument count.
    pub arguments: usize,
    /// Aggregate argument/environment bytes.
    pub argument_bytes: usize,
    /// Owned child/timer duration.
    pub milliseconds: u64,
}
impl HostLimits {
    /// Finite approved initial host profile.
    pub const DEFAULT: Self = Self {
        bytes: 16 * 1024 * 1024,
        entries: 4096,
        arguments: 1024,
        argument_bytes: 64 * 1024,
        milliseconds: 30000,
    };
    fn validate(self) -> Result<(), HostError> {
        if self.bytes > Self::DEFAULT.bytes
            || self.entries > 4096
            || self.arguments > 1024
            || self.argument_bytes > 65536
            || self.milliseconds > 30000
        {
            return Err(err("policy", "host limits may only narrow defaults"));
        }
        Ok(())
    }
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
#[derive(Debug)]
struct Program {
    executable: File,
    cwd: File,
    environment: BTreeMap<String, String>,
}
/// Version-three host grants, owned by the trusted embedding caller. Default denies all services.
#[derive(Debug)]
pub struct HostV3 {
    read: BTreeMap<String, File>,
    write: BTreeMap<String, File>,
    #[cfg(all(
        target_os = "linux",
        any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
    ))]
    programs: BTreeMap<String, Program>,
    /// Explicit time authority.
    pub clocks: bool,
    /// Explicit timer authority.
    pub timers: bool,
    /// Lowerable host resource policy.
    pub limits: HostLimits,
    epoch: Instant,
}
impl Default for HostV3 {
    fn default() -> Self {
        Self {
            read: BTreeMap::new(),
            write: BTreeMap::new(),
            #[cfg(all(
                target_os = "linux",
                any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
            ))]
            programs: BTreeMap::new(),
            clocks: false,
            timers: false,
            limits: HostLimits::DEFAULT,
            epoch: Instant::now(),
        }
    }
}
fn object(v: &J) -> Result<&[(String, J)], HostError> {
    if let J::Object(v) = v {
        Ok(v)
    } else {
        Err(err("input", "expected object"))
    }
}
fn field<'a>(v: &'a J, name: &str) -> Result<&'a J, HostError> {
    object(v)?
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v)
        .ok_or_else(|| err("input", format!("missing {name}")))
}
fn text(v: &J) -> Result<&str, HostError> {
    if let J::String(v) = v {
        if v.contains('\0') {
            return Err(err("input", "NUL is forbidden"));
        }
        Ok(v)
    } else {
        Err(err("input", "expected String"))
    }
}
fn int(v: &J) -> Result<i64, HostError> {
    if let J::Int(v) = v {
        Ok(*v)
    } else {
        Err(err("input", "expected Int"))
    }
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
fn obj(items: Vec<(&str, J)>) -> J {
    J::Object(items.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
fn encode(bytes: &[u8]) -> Result<J, HostError> {
    let mut fuel = RuntimeValueFuel::new(MaterializationLimits::UNLIMITED);
    let mut work = RuntimeDataWork::new(128 * 1024 * 1024);
    let b = RuntimeBytesV2::from_slice(bytes, RuntimeBytesLimits::DEFAULT, &mut fuel, &mut work)
        .map_err(|_| err("limit", "binary output"))?;
    Ok(J::String(
        b.to_base64(RuntimeBytesLimits::DEFAULT, &mut fuel, &mut work)
            .map_err(|_| err("limit", "base64 output"))?,
    ))
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
fn decode(v: &J, limit: usize) -> Result<Vec<u8>, HostError> {
    let mut fuel = RuntimeValueFuel::new(MaterializationLimits::UNLIMITED);
    let mut work = RuntimeDataWork::new(128 * 1024 * 1024);
    let mut limits = RuntimeBytesLimits::DEFAULT;
    limits.max_bytes = limit;
    let b = RuntimeBytesV2::from_base64(text(v)?, limits, &mut fuel, &mut work)
        .map_err(|_| err("input", "strict Base64 required"))?;
    Ok(b.as_slice().to_vec())
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
fn open_flags(directory: bool) -> i32 {
    // Linux arm64 overrides asm-generic's directory/no-follow flag values.
    let (nofollow, directory_flag) = if cfg!(target_arch = "aarch64") {
        (0x8000, 0x4000)
    } else {
        (0x20000, 0x10000)
    };
    0x800 | nofollow | if directory { directory_flag } else { 0 }
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
fn dir(path: &Path) -> Result<File, HostError> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .custom_flags(open_flags(true))
        .open(path)
        .map_err(io)
}
#[cfg(not(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
)))]
fn dir(_: &Path) -> Result<File, HostError> {
    Err(err(
        "unsupported",
        "handle-rooted filesystem requires Linux",
    ))
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
fn fd(file: &File) -> PathBuf {
    use std::os::fd::AsRawFd;
    PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()))
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
fn target(root: &File, path: &str) -> Result<(File, PathBuf), HostError> {
    if path.len() > 4096 || path.contains('\0') {
        return Err(err("input", "invalid path"));
    }
    let mut parts = Vec::new();
    for c in Path::new(path).components() {
        match c {
            Component::Normal(n) => parts.push(n.to_owned()),
            Component::CurDir => {}
            _ => return Err(err("authority", "relative path cannot escape root")),
        }
    }
    let mut current = root.try_clone().map_err(io)?;
    for part in parts.iter().take(parts.len().saturating_sub(1)) {
        current = dir(&fd(&current).join(part))?;
    }
    let path = parts
        .last()
        .map_or_else(|| fd(&current), |name| fd(&current).join(name));
    Ok((current, path))
}
impl HostV3 {
    pub(super) fn with_limits(limits: HostLimits) -> Result<Self, HostError> {
        limits.validate()?;
        Ok(Self {
            limits,
            ..Self::default()
        })
    }

    /// Grant a captured read directory; no source API can call this method.
    pub fn grant_read(&mut self, name: &str, path: &Path) -> Result<(), HostError> {
        self.read
            .insert(name.into(), dir(&fs::canonicalize(path).map_err(io)?)?);
        Ok(())
    }
    /// Grant a separate captured write directory.
    pub fn grant_write(&mut self, name: &str, path: &Path) -> Result<(), HostError> {
        self.write
            .insert(name.into(), dir(&fs::canonicalize(path).map_err(io)?)?);
        Ok(())
    }
    /// Grant an exact native executable, fixed cwd and explicit environment; no shell or inheritance.
    pub fn grant_program(
        &mut self,
        name: &str,
        executable: &Path,
        cwd: &Path,
        environment: BTreeMap<String, String>,
    ) -> Result<(), HostError> {
        #[cfg(not(all(
            target_os = "linux",
            any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
        )))]
        {
            let _ = (name, executable, cwd, environment);
            Err(err("unsupported", "owned process adapter requires Linux"))
        }
        #[cfg(all(
            target_os = "linux",
            any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
        ))]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let executable = OpenOptions::new()
                .read(true)
                .custom_flags(open_flags(false))
                .open(fs::canonicalize(executable).map_err(io)?)
                .map_err(io)?;
            let mut magic = [0; 4];
            (&executable).read_exact(&mut magic).map_err(io)?;
            if magic != *b"\x7fELF" {
                return Err(err(
                    "unsupported",
                    "executable grant requires a native ELF image",
                ));
            }
            let mut total = 0usize;
            for (k, v) in &environment {
                if k.is_empty() || k.contains(['\0', '=']) || v.contains('\0') {
                    return Err(err("input", "invalid environment"));
                }
                total = total
                    .checked_add(k.len() + v.len())
                    .ok_or_else(|| err("limit", "environment size"))?;
            }
            if total > self.limits.argument_bytes {
                return Err(err("limit", "environment size"));
            }
            self.programs.insert(
                name.into(),
                Program {
                    executable,
                    cwd: dir(&fs::canonicalize(cwd).map_err(io)?)?,
                    environment,
                },
            );
            Ok(())
        }
    }
    /// Load at most 64 KiB of explicit trusted CLI policy; never read policy from source code.
    pub fn load_policy(path: &Path) -> Result<Self, HostError> {
        let file = File::open(path).map_err(io)?;
        let mut bytes = Vec::new();
        file.take(65537).read_to_end(&mut bytes).map_err(io)?;
        if bytes.len() > 65536 {
            return Err(err("policy", "host policy exceeds 64 KiB"));
        }
        Self::from_policy(&bytes)
    }
    /// Read an explicit versioned policy file. Paths and executable grants are trusted caller inputs.
    pub fn from_policy(bytes: &[u8]) -> Result<Self, HostError> {
        let mut limits = JsonLimits::DEFAULT;
        limits.bytes = 65536;
        limits.values = 4096;
        let v = parse_json(bytes, limits).map_err(|_| err("policy", "invalid host policy JSON"))?;
        if int(field(&v, "version")?)? != 3 {
            return Err(err("policy", "host policy version must be 3"));
        }
        let mut host = Self::default();
        for (k, value) in object(&v)? {
            match k.as_str() {
                "version" => {}
                "read_roots" | "write_roots" => {
                    for (name, path) in object(value)? {
                        if k == "read_roots" {
                            host.grant_read(name, Path::new(text(path)?))?;
                        } else {
                            host.grant_write(name, Path::new(text(path)?))?;
                        }
                    }
                }
                "clocks" => {
                    if let J::Bool(v) = value {
                        host.clocks = *v;
                    } else {
                        return Err(err("policy", "clocks must be Bool"));
                    }
                }
                "timers" => {
                    if let J::Bool(v) = value {
                        host.timers = *v;
                    } else {
                        return Err(err("policy", "timers must be Bool"));
                    }
                }
                "limits" => {
                    for (k, v) in object(value)? {
                        let n = u64::try_from(int(v)?)
                            .map_err(|_| err("policy", "negative host limit"))?;
                        match k.as_str() {
                            "bytes" => {
                                host.limits.bytes =
                                    usize::try_from(n).map_err(|_| err("policy", "byte limit"))?
                            }
                            "entries" => {
                                host.limits.entries =
                                    usize::try_from(n).map_err(|_| err("policy", "entry limit"))?
                            }
                            "arguments" => {
                                host.limits.arguments = usize::try_from(n)
                                    .map_err(|_| err("policy", "argument limit"))?
                            }
                            "argument_bytes" => {
                                host.limits.argument_bytes = usize::try_from(n)
                                    .map_err(|_| err("policy", "argument byte limit"))?
                            }
                            "milliseconds" => host.limits.milliseconds = n,
                            _ => return Err(err("policy", "unknown host limit")),
                        }
                    }
                }
                "processes" => {
                    for (name, p) in object(value)? {
                        if object(p)?.iter().any(|(key, _)| {
                            !["executable", "cwd", "environment"].contains(&key.as_str())
                        }) {
                            return Err(err("policy", "unknown process grant field"));
                        }
                        let mut env = BTreeMap::new();
                        for (k, v) in object(field(p, "environment")?)? {
                            env.insert(k.clone(), text(v)?.to_owned());
                        }
                        host.grant_program(
                            name,
                            Path::new(text(field(p, "executable")?)?),
                            Path::new(text(field(p, "cwd")?)?),
                            env,
                        )?;
                    }
                }
                _ => return Err(err("policy", "unknown host policy field")),
            }
        }
        host.limits.validate()?;
        Ok(host)
    }
    /// Invoke a versioned, explicitly typed JSON request. Output binary streams are standard Base64.
    /// The checkpoint is called during timers, stream polling and bounded file I/O.
    pub fn call<E>(
        &self,
        request: &J,
        mut checkpoint: impl FnMut() -> Result<(), E>,
    ) -> Result<Result<J, HostError>, E> {
        checkpoint()?;
        self.call_inner(request, &mut checkpoint)
    }
    fn call_inner<E>(
        &self,
        r: &J,
        tick: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<Result<J, HostError>, E> {
        let setup = (|| {
            self.limits.validate()?;
            if int(field(r, "version")?)? != 3 {
                return Err(err("input", "request version must be 3"));
            }
            text(field(r, "op")?).map(str::to_owned)
        })();
        let op = match setup {
            Ok(op) => op,
            Err(e) => return Ok(Err(e)),
        };
        let allowed: &[&str] = match op.as_str() {
            "path.normalize" | "path.basename" => &["version", "op", "path"],
            "path.join" => &["version", "op", "left", "right"],
            "date.parse" => &["version", "op", "text"],
            "date.format" => &["version", "op", "milliseconds", "offset_minutes"],
            "date.add" => &["version", "op", "milliseconds", "delta"],
            "clock.wall" | "clock.monotonic" => &["version", "op"],
            "timer.sleep" => &["version", "op", "milliseconds"],
            "process.run" => &["version", "op", "program", "arguments", "input"],
            "fs.write" => &["version", "op", "root", "path", "data"],
            "fs.rename" => &["version", "op", "root", "path", "to"],
            "fs.read" | "fs.list" | "fs.metadata" | "fs.mkdir" | "fs.remove" => {
                &["version", "op", "root", "path"]
            }
            _ => return Ok(Err(err("unsupported", "unknown version-three operation"))),
        };
        let fields = match object(r) {
            Ok(fields) => fields,
            Err(e) => return Ok(Err(e)),
        };
        for (i, (key, _)) in fields.iter().enumerate() {
            if !allowed.contains(&key.as_str()) || fields[..i].iter().any(|(k, _)| k == key) {
                return Ok(Err(err("input", "unknown or duplicate request field")));
            }
        }
        if op.starts_with("path.") {
            return Ok((|| {
                let normalize = |path: &str| -> Result<String, HostError> {
                    let mut parts = Vec::new();
                    for part in path.split('/') {
                        match part {
                            "" | "." => {}
                            ".." => {
                                return Err(err(
                                    "input",
                                    "parent components are not portable relative paths",
                                ))
                            }
                            part => parts.push(part),
                        }
                    }
                    if path.starts_with('/') {
                        return Err(err("input", "relative path required"));
                    }
                    Ok(parts.join("/"))
                };
                match op.as_str() {
                    "path.normalize" => normalize(text(field(r, "path")?)?).map(J::String),
                    "path.basename" => {
                        let path = normalize(text(field(r, "path")?)?)?;
                        Ok(J::String(path.rsplit('/').next().unwrap_or("").into()))
                    }
                    "path.join" => {
                        let a = normalize(text(field(r, "left")?)?)?;
                        let b = normalize(text(field(r, "right")?)?)?;
                        Ok(J::String(if a.is_empty() {
                            b
                        } else if b.is_empty() {
                            a
                        } else {
                            format!("{a}/{b}")
                        }))
                    }
                    _ => Err(err("unsupported", "path operation")),
                }
            })());
        }
        if op.starts_with("date.") {
            return Ok((|| match op.as_str() {
                "date.parse" => super::parse_date(text(field(r, "text")?)?).map(J::Int),
                "date.format" => super::format_date(
                    int(field(r, "milliseconds")?)?,
                    int(field(r, "offset_minutes")?)?,
                )
                .map(J::String),
                "date.add" => {
                    super::add_date(int(field(r, "milliseconds")?)?, int(field(r, "delta")?)?)
                        .map(J::Int)
                }
                _ => Err(err("unsupported", "date operation")),
            })());
        }
        if op == "timer.sleep" {
            let ms = match field(r, "milliseconds").and_then(int) {
                Ok(n) if n >= 0 && n as u64 <= self.limits.milliseconds => n as u64,
                _ => return Ok(Err(err("limit", "timer duration"))),
            };
            if !self.timers {
                return Ok(Err(err("authority", "timers denied")));
            }
            let start = Instant::now();
            while start.elapsed() < Duration::from_millis(ms) {
                tick()?;
                std::thread::sleep(
                    Duration::from_millis(2)
                        .min(Duration::from_millis(ms).saturating_sub(start.elapsed())),
                );
            }
            return Ok(Ok(J::Null));
        }
        if op == "clock.wall" || op == "clock.monotonic" {
            if !self.clocks {
                return Ok(Err(err("authority", "clocks denied")));
            }
            let n = if op == "clock.wall" {
                match SystemTime::now().duration_since(UNIX_EPOCH) {
                    Ok(d) => i64::try_from(d.as_millis()).ok(),
                    Err(e) => i64::try_from(e.duration().as_millis())
                        .ok()
                        .and_then(i64::checked_neg),
                }
            } else {
                i64::try_from(self.epoch.elapsed().as_millis()).ok()
            };
            return Ok(n.map(J::Int).ok_or_else(|| err("limit", "clock range")));
        }
        if op == "process.run" {
            return self.process(r, tick);
        }
        if op.starts_with("fs.") {
            #[cfg(not(all(
                target_os = "linux",
                any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
            )))]
            {
                return Ok(Err(err(
                    "unsupported",
                    "handle-rooted filesystem requires Linux",
                )));
            }
            #[cfg(all(
                target_os = "linux",
                any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
            ))]
            {
                return self.files(r, &op, tick);
            }
        }
        Ok(Err(err(
            "unsupported",
            "unknown version-three host operation",
        )))
    }
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
impl HostV3 {
    fn files<E>(
        &self,
        r: &J,
        op: &str,
        tick: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<Result<J, HostError>, E> {
        use std::os::unix::fs::OpenOptionsExt;
        let args = (|| {
            let name = text(field(r, "root")?)?;
            let path = text(field(r, "path")?)?;
            let write = matches!(op, "fs.write" | "fs.mkdir" | "fs.remove" | "fs.rename");
            let root = if write {
                self.write.get(name)
            } else {
                self.read.get(name)
            }
            .ok_or_else(|| err("authority", "root grant denied"))?;
            target(root, path)
        })();
        let (_parent, path) = match args {
            Ok(v) => v,
            Err(e) => return Ok(Err(e)),
        };
        if op == "fs.read" {
            let opened = (|| {
                let f = OpenOptions::new()
                    .read(true)
                    .custom_flags(open_flags(false))
                    .open(path)
                    .map_err(io)?;
                if !f.metadata().map_err(io)?.is_file() {
                    return Err(err("unsupported", "regular file required"));
                }
                Ok(f)
            })();
            let mut f = match opened {
                Ok(f) => f,
                Err(e) => return Ok(Err(e)),
            };
            let mut out = Vec::new();
            let mut chunk = [0u8; 65536];
            loop {
                tick()?;
                let n = match f.read(&mut chunk) {
                    Ok(n) => n,
                    Err(e) => return Ok(Err(io(e))),
                };
                if n == 0 {
                    break;
                }
                if n > self.limits.bytes.saturating_sub(out.len()) {
                    return Ok(Err(err("limit", "file byte ceiling")));
                }
                out.extend_from_slice(&chunk[..n]);
            }
            return Ok(encode(&out));
        }
        if op == "fs.write" {
            let data = match field(r, "data").and_then(|v| decode(v, self.limits.bytes)) {
                Ok(v) => v,
                Err(e) => return Ok(Err(e)),
            };
            let opened = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .custom_flags(open_flags(false))
                .open(path);
            let mut f = match opened {
                Ok(f) => f,
                Err(e) => return Ok(Err(io(e))),
            };
            let regular = match f.metadata() {
                Ok(m) => m.is_file(),
                Err(e) => return Ok(Err(io(e))),
            };
            if !regular {
                return Ok(Err(err("unsupported", "regular file required")));
            }
            if let Err(e) = f.set_len(0) {
                return Ok(Err(io(e)));
            }
            for bytes in data.chunks(65536) {
                tick()?;
                if let Err(e) = f.write_all(bytes) {
                    return Ok(Err(err("partial", format!("file created/truncated; {e}"))));
                }
            }
            return Ok(f
                .sync_all()
                .map(|_| J::Int(data.len() as i64))
                .map_err(|e| err("partial", format!("file changed; {e}"))));
        }
        if op == "fs.list" {
            let opened = if path == fd(&_parent) {
                _parent.try_clone().map_err(io)
            } else {
                dir(&path)
            };
            let d = match opened {
                Ok(d) => d,
                Err(e) => return Ok(Err(e)),
            };
            let entries = match fs::read_dir(fd(&d)) {
                Ok(v) => v,
                Err(e) => return Ok(Err(io(e))),
            };
            let mut names = Vec::new();
            for entry in entries {
                tick()?;
                if names.len() >= self.limits.entries {
                    return Ok(Err(err("limit", "directory entry ceiling")));
                }
                let entry = match entry {
                    Ok(v) => v,
                    Err(e) => return Ok(Err(io(e))),
                };
                let name = match entry.file_name().into_string() {
                    Ok(v) => v,
                    Err(_) => return Ok(Err(err("unsupported", "directory name is not UTF-8"))),
                };
                names.push(J::String(name));
            }
            names.sort_by(|a, b| match (a, b) {
                (J::String(a), J::String(b)) => a.cmp(b),
                _ => std::cmp::Ordering::Equal,
            });
            return Ok(Ok(J::Array(names)));
        }
        let out = (|| {
            let metadata = if path == fd(&_parent) {
                _parent.metadata()
            } else {
                fs::symlink_metadata(&path)
            }
            .map_err(io)?;
            if metadata.file_type().is_symlink() {
                return Err(err("authority", "symlink target refused"));
            }
            match op {
                "fs.metadata" => Ok(obj(vec![
                    (
                        "bytes",
                        J::Int(
                            i64::try_from(metadata.len()).map_err(|_| err("limit", "file size"))?,
                        ),
                    ),
                    ("directory", J::Bool(metadata.is_dir())),
                    ("readonly", J::Bool(metadata.permissions().readonly())),
                ])),
                "fs.remove" => {
                    if metadata.is_dir() {
                        fs::remove_dir(&path)
                    } else {
                        fs::remove_file(&path)
                    }
                    .map_err(io)?;
                    Ok(J::Null)
                }
                "fs.rename" => {
                    let name = text(field(r, "root")?)?;
                    let root = self
                        .write
                        .get(name)
                        .ok_or_else(|| err("authority", "write root denied"))?;
                    let (_dest, dest) = target(root, text(field(r, "to")?)?)?;
                    if fs::symlink_metadata(&dest).is_ok_and(|m| m.file_type().is_symlink()) {
                        return Err(err("authority", "rename symlink destination refused"));
                    }
                    fs::rename(&path, dest).map_err(io)?;
                    Ok(J::Null)
                }
                _ => Err(err("unsupported", "filesystem operation")),
            }
        })();
        if op == "fs.mkdir" {
            return Ok(fs::create_dir(path).map(|_| J::Null).map_err(io));
        }
        Ok(out)
    }
}
#[cfg(not(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
)))]
impl HostV3 {
    fn process<E>(
        &self,
        _: &J,
        _: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<Result<J, HostError>, E> {
        Ok(Err(err(
            "unsupported",
            "owned process adapter requires Linux",
        )))
    }
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
impl HostV3 {
    pub(super) fn start_process(
        &self,
        r: &J,
    ) -> Result<process_exchange::OwnedProcessExchange, HostError> {
        use std::{
            os::{fd::OwnedFd, unix::net::UnixStream},
            process::{Command, Stdio},
        };
        (|| {
            let grant = self
                .programs
                .get(text(field(r, "program")?)?)
                .ok_or_else(|| err("authority", "executable grant denied"))?;
            let J::Array(args) = field(r, "arguments")? else {
                return Err(err("input", "arguments must be array"));
            };
            if args.len() > self.limits.arguments {
                return Err(err("limit", "argument count"));
            }
            let mut total = grant.environment.iter().try_fold(0usize, |n, (k, v)| {
                n.checked_add(k.len())
                    .and_then(|n| n.checked_add(v.len() + 1))
                    .ok_or_else(|| err("limit", "environment bytes"))
            })?;
            if total > self.limits.argument_bytes {
                return Err(err("limit", "environment bytes"));
            }
            let mut argv = Vec::new();
            for arg in args {
                let arg = text(arg)?;
                total = total
                    .checked_add(arg.len())
                    .and_then(|n| n.checked_add(1))
                    .ok_or_else(|| err("limit", "argument bytes"))?;
                if total > self.limits.argument_bytes {
                    return Err(err("limit", "argument bytes"));
                }
                argv.push(arg.to_owned());
            }
            let input = decode(field(r, "input")?, self.limits.bytes)?;
            let (a, child_in) = UnixStream::pair().map_err(io)?;
            let (b, child_out) = UnixStream::pair().map_err(io)?;
            let (c, child_err) = UnixStream::pair().map_err(io)?;
            a.set_nonblocking(true).map_err(io)?;
            b.set_nonblocking(true).map_err(io)?;
            c.set_nonblocking(true).map_err(io)?;
            let child = Command::new(fd(&grant.executable))
                .args(argv)
                .current_dir(fd(&grant.cwd))
                .env_clear()
                .envs(&grant.environment)
                .stdin(Stdio::from(OwnedFd::from(child_in)))
                .stdout(Stdio::from(OwnedFd::from(child_out)))
                .stderr(Stdio::from(OwnedFd::from(child_err)))
                .spawn()
                .map_err(io)?;
            Ok((child, a, b, c, input))
        })()
        .map(|(child, input_pipe, stdout, stderr, input)| {
            process_exchange::OwnedProcessExchange::new(
                child,
                input_pipe,
                stdout,
                stderr,
                input,
                self.limits,
            )
        })
    }
    fn process<E>(
        &self,
        r: &J,
        tick: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<Result<J, HostError>, E> {
        let mut pending = match self.start_process(r) {
            Ok(pending) => pending,
            Err(e) => return Ok(Err(e)),
        };
        let run = loop {
            match pending.poll(tick) {
                Ok(Ok(None)) => std::thread::sleep(Duration::from_millis(2)),
                Ok(Ok(Some(value))) => break Ok(Ok(value)),
                Ok(Err(e)) => break Ok(Err(e)),
                Err(e) => break Err(e),
            }
        };
        if let Err(e) = pending.cleanup() {
            return Ok(Err(e));
        }
        run
    }
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
#[derive(Debug)]
struct OwnedChild(std::process::Child);
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
impl std::ops::Deref for OwnedChild {
    type Target = std::process::Child;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
impl std::ops::DerefMut for OwnedChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
#[path = "host_process_exchange_v3.rs"]
mod process_exchange;

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) use process_exchange::OwnedProcessExchange;
