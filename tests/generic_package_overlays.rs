//! Unsaved generic graphs check registered buffers without publishing executable authority.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use sovra::compiler::{project::packages::*, v2::*};
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "svr generic overlays {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(p.join("app")).unwrap();
        std::fs::create_dir(p.join("helper")).unwrap();
        std::fs::write(p.join("app/sovra.toml"),"[project]\nname=\"app\"\nentry=\"main.svr\"\n[dependencies.helper]\npath=\"../helper\"\n").unwrap();
        std::fs::write(
            p.join("helper/sovra.toml"),
            "[project]\nname=\"helper\"\nentry=\"main.svr\"\n[modules]\nvalues=\"values.svr\"\n",
        )
        .unwrap();
        std::fs::write(
            p.join("app/main.svr"),
            "use helper::values;fn main(){let x=helper::values::id::<Int>(1);let never_run=1/0;}",
        )
        .unwrap();
        std::fs::write(p.join("helper/main.svr"), "fn main(){}").unwrap();
        std::fs::write(
            p.join("helper/values.svr"),
            "mod values {export fn id<T>(x:T)->T{return x;}}",
        )
        .unwrap();
        Self(p)
    }
    fn overlay(&self, path: &str, source: &str) -> SourceOverlay {
        SourceOverlay {
            entry: self.0.join(path),
            source: source.into(),
        }
    }
    fn check(
        &self,
        overlays: &[SourceOverlay],
        limits: SourceOverlayLimits,
    ) -> Result<(), sovra::compiler::diagnostics::Diagnostics> {
        check_generic_package_with_overlays(self.0.join("app"), overlays, limits)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn unsaved_module_errors_have_original_file_ranges_and_do_not_mutate_disk_or_locks() {
    let f = Fixture::new();
    let path = f.0.join("helper/values.svr");
    let original = std::fs::read(&path).unwrap();
    let bad = "// λ😀\r\nmod values {export fn id<T>(x:T)->T{return true;}}";
    let error = f
        .check(
            &[f.overlay("helper/values.svr", bad)],
            SourceOverlayLimits::DEFAULT,
        )
        .unwrap_err();
    assert!(error
        .items
        .iter()
        .any(|e| e.source_file.as_deref() == path.to_str()
            && e.span.line == 1
            && bad.get(e.span.start..e.span.end).is_some()));
    f.check(
        &[f.overlay(
            "helper/values.svr",
            "mod values {export fn id<T>(x:T)->T{return x;}}",
        )],
        SourceOverlayLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(std::fs::read(path).unwrap(), original);
    assert!(!f.0.join("app/sovra.lock").exists());
}
#[test]
fn foreign_duplicate_manifest_and_nonexistent_overlay_paths_refuse() {
    let f = Fixture::new();
    std::fs::write(f.0.join("notes.svr"), "fn main(){}").unwrap();
    for items in [
        vec![f.overlay("notes.svr", "fn main(){}")],
        vec![f.overlay("app/sovra.toml", "")],
        vec![f.overlay("missing.svr", "")],
        vec![
            f.overlay("app/main.svr", "fn main(){}"),
            f.overlay("app/./main.svr", "fn main(){}"),
        ],
    ] {
        assert!(f.check(&items, SourceOverlayLimits::DEFAULT).is_err());
    }
}
#[test]
fn lowerable_overlay_count_and_bytes_are_exact_and_never_fall_back_to_disk() {
    let f = Fixture::new();
    let text = "fn main(){}";
    let item = f.overlay("app/main.svr", text);
    let mut limits = SourceOverlayLimits {
        max_files: 1,
        max_source_bytes: text.len(),
        max_total_source_bytes: text.len(),
    };
    f.check(std::slice::from_ref(&item), limits).unwrap();
    limits.max_source_bytes -= 1;
    assert!(f.check(std::slice::from_ref(&item), limits).is_err());
    limits.max_source_bytes += 1;
    limits.max_total_source_bytes -= 1;
    assert!(f.check(std::slice::from_ref(&item), limits).is_err());
    limits.max_total_source_bytes += 1;
    limits.max_files = 0;
    assert!(f.check(&[item], limits).is_err());
}
#[test]
fn cli_overlay_wire_echoes_profile_and_id_and_rejects_truncation_and_mode_mixups() {
    let f = Fixture::new();
    let path = std::fs::canonicalize(f.0.join("app/main.svr")).unwrap();
    let path = path.to_str().unwrap().as_bytes();
    let text = b"fn main(){let never_run=1/0;}";
    let mut wire = SOURCE_OVERLAY_MAGIC.to_vec();
    for n in [71u32, 1, path.len() as u32, text.len() as u32] {
        wire.extend(n.to_be_bytes());
    }
    wire.extend(path);
    wire.extend(text);
    let execute = |args: &[&str], input: &[u8]| {
        let mut child = Command::new(env!("CARGO_BIN_EXE_svr"))
            .args(args)
            .arg(f.0.join("app"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let _ = child.stdin.take().unwrap().write_all(input);
        child.wait_with_output().unwrap()
    };
    let flags = [
        "check",
        "--experimental-generics",
        "--format",
        "json",
        "--overlays-stdin",
    ];
    let out = execute(&flags, &wire);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let value = parse_json(&out.stdout, JsonLimits::DEFAULT).unwrap();
    let JsonValue::Object(fields) = value else {
        panic!()
    };
    assert!(fields.contains(&(
        "profile".into(),
        JsonValue::String("experimental-generics".into())
    )));
    assert!(String::from_utf8_lossy(&out.stdout).contains("\"request_id\":71"));
    assert!(!execute(&flags, &wire[..wire.len() - 1]).status.success());
    for extra in [
        "--locked",
        "--symbols",
        "--package-context",
        "--experimental-data",
        "--experimental-collections",
        "--overlays-stdin",
    ] {
        let mut args = flags.to_vec();
        args.push(extra);
        assert_eq!(execute(&args, &[]).status.code(), Some(2));
    }
}

fn field<'a>(j: &'a JsonValue, key: &str) -> &'a JsonValue {
    let JsonValue::Object(v) = j else { panic!() };
    &v.iter().find(|(k, _)| k == key).unwrap().1
}
fn array(j: &JsonValue) -> &[JsonValue] {
    let JsonValue::Array(v) = j else { panic!() };
    v
}
fn number(j: &JsonValue) -> usize {
    let JsonValue::Int(v) = j else { panic!() };
    *v as usize
}
#[test]
fn checked_calls_with_identical_names_keep_distinct_cross_file_targets() {
    let f = Fixture::new();
    std::fs::write(f.0.join("helper/sovra.toml"),"[project]\nname=\"helper\"\nentry=\"main.svr\"\n[modules]\nvalues=\"values.svr\"\nother=\"other.svr\"\n").unwrap();
    std::fs::write(
        f.0.join("helper/other.svr"),
        "// λ😀\r\nmod other {export fn id<T>(x:T)->T{return x;}}",
    )
    .unwrap();
    let text="use helper::values;use helper::other;fn main(){let a=helper :: values :: id::<Int>(1);let b=helper::other::id::<Int>(2);}";
    let overlay = f.overlay("app/main.svr", text);
    let symbols = check_generic_package_overlays_with_symbols(
        f.0.join("app"),
        &[overlay],
        SourceOverlayLimits::DEFAULT,
        GenericSymbolLimits::DEFAULT,
    )
    .unwrap();
    let meta = parse_json(symbols.as_bytes(), JsonLimits::DEFAULT).unwrap();
    assert_eq!(field(&meta, "unavailable"), &JsonValue::Bool(false));
    let declarations = array(field(&meta, "declarations"));
    let refs = array(field(&meta, "references"));
    let files = array(field(&meta, "files"));
    assert_eq!(refs.len(), 2);
    let targets: Vec<_> = refs
        .iter()
        .map(|r| number(field(r, "definition")))
        .collect();
    assert_ne!(targets[0], targets[1]);
    for (r, expected) in refs.iter().zip(["helper/values.svr", "helper/other.svr"]) {
        let target = &declarations[number(field(r, "definition"))];
        assert_eq!(field(target, "name"), &JsonValue::String("id".into()));
        assert_eq!(
            field(&files[number(field(target, "file"))], "path"),
            &JsonValue::String(
                std::fs::canonicalize(f.0.join(expected))
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .into()
            )
        );
        assert!(text
            .get(number(field(r, "start"))..number(field(r, "end")))
            .unwrap()
            .ends_with("id"));
    }
}
#[test]
fn optional_navigation_quotas_refuse_whole_projection_without_invalidating_checks() {
    let f = Fixture::new();
    for reduce in 0..6 {
        let mut limits = GenericSymbolLimits::DEFAULT;
        match reduce {
            0 => limits.max_declarations = 0,
            1 => limits.max_references = 0,
            2 => limits.max_bytes = 1,
            3 => limits.max_work = 0,
            4 => limits.max_locals = 0,
            _ => limits.max_local_references = 0,
        }
        let metadata = check_generic_package_overlays_with_symbols(
            f.0.join("app"),
            &[],
            SourceOverlayLimits::DEFAULT,
            limits,
        )
        .unwrap();
        assert_eq!(metadata, "{\"schema_version\":1,\"unavailable\":true}");
    }
    std::fs::write(
        f.0.join("helper/values.svr"),
        "mod values {fn private()->Int{return 7;}export fn id<T>(x:T)->T{return x;}}",
    )
    .unwrap();
    let invalid = f.overlay(
        "app/main.svr",
        "use helper::values;fn main(){let x=helper::values::private();}",
    );
    assert!(check_generic_package_overlays_with_symbols(
        f.0.join("app"),
        &[invalid],
        SourceOverlayLimits::DEFAULT,
        GenericSymbolLimits::DEFAULT
    )
    .is_err());
}

#[test]
fn package_type_annotations_preserve_aliases_qualification_and_definition_scopes() {
    let f = Fixture::new();
    let provider="// λ😀\r\nmod values {export struct Box<T>{value:T}export enum Choice<T>{One(Box<T>),End}export fn id<T>(x:T)->T{return x;}}";
    std::fs::write(f.0.join("helper/values.svr"), provider).unwrap();
    let source="use helper::values;struct Box<T>{local:T}type Alias<T>=helper :: values :: Box<T>;struct Holder<T>{item:helper::values::Box<T>}enum Local<T>{One(helper::values::Choice<T>),End}fn keep<T>(x:Alias<T>)->helper::values::Box<T>{let saved:Alias<T>=x;let items:[Alias<T>]=[saved];let qualified:helper :: values :: Box<T>=saved;return qualified;}fn local<T>(x:Box<T>)->Box<T>{return x;}fn build<T>(x:T){let own=Box::<T>{local:x};let foreign=helper :: values :: Box::<T>{value:x};let choice=helper::values::Choice::<T>::One(foreign);let copied=helper::values::id::<Alias<T>>(foreign);let array=helper::values::id::<[Alias<T>]>([copied]);}fn main(){}";
    let overlay = f.overlay("app/main.svr", source);
    let meta = check_generic_package_overlays_with_symbols(
        f.0.join("app"),
        &[overlay],
        SourceOverlayLimits::DEFAULT,
        GenericSymbolLimits::DEFAULT,
    )
    .unwrap();
    let meta = parse_json(meta.as_bytes(), JsonLimits::DEFAULT).unwrap();
    assert_eq!(field(&meta, "unavailable"), &JsonValue::Bool(false));
    let files = array(field(&meta, "files"));
    let declarations = array(field(&meta, "declarations"));
    let refs = array(field(&meta, "references"));
    for (file, text, start, expected_file, name, kind) in [
        (
            "app/main.svr",
            source,
            source.find("Box::<T>{local").unwrap(),
            "app/main.svr",
            "Box",
            23,
        ),
        (
            "app/main.svr",
            source,
            source.find("helper :: values :: Box::<T>").unwrap() + "helper :: values :: ".len(),
            "helper/values.svr",
            "Box",
            23,
        ),
        (
            "app/main.svr",
            source,
            source.find("Choice::<T>").unwrap(),
            "helper/values.svr",
            "Choice",
            10,
        ),
        (
            "app/main.svr",
            source,
            source.find("id::<Alias").unwrap() + 5,
            "app/main.svr",
            "Alias",
            26,
        ),
        (
            "app/main.svr",
            source,
            source.find("id::<[Alias").unwrap() + 6,
            "app/main.svr",
            "Alias",
            26,
        ),
        (
            "app/main.svr",
            source,
            source.find("items:[Alias").unwrap() + 7,
            "app/main.svr",
            "Alias",
            26,
        ),
        (
            "app/main.svr",
            source,
            source.find("saved:Alias").unwrap() + 6,
            "app/main.svr",
            "Alias",
            26,
        ),
        (
            "app/main.svr",
            source,
            source.find("qualified:helper :: values :: Box").unwrap()
                + "qualified:helper :: values :: ".len(),
            "helper/values.svr",
            "Box",
            23,
        ),
        (
            "app/main.svr",
            source,
            source.find("x:Alias").unwrap() + 2,
            "app/main.svr",
            "Alias",
            26,
        ),
        (
            "app/main.svr",
            source,
            source.find("helper :: values :: Box").unwrap() + "helper :: values :: ".len(),
            "helper/values.svr",
            "Box",
            23,
        ),
        (
            "app/main.svr",
            source,
            source.find("item:helper::values::Box").unwrap() + "item:helper::values::".len(),
            "helper/values.svr",
            "Box",
            23,
        ),
        (
            "app/main.svr",
            source,
            source.find("x:Box").unwrap() + 2,
            "app/main.svr",
            "Box",
            23,
        ),
        (
            "app/main.svr",
            source,
            source.find("One(helper::values::Choice").unwrap() + "One(helper::values::".len(),
            "helper/values.svr",
            "Choice",
            10,
        ),
        (
            "helper/values.svr",
            provider,
            provider.find("One(Box").unwrap() + 4,
            "helper/values.svr",
            "Box",
            23,
        ),
    ] {
        let path = JsonValue::String(
            std::fs::canonicalize(f.0.join(file))
                .unwrap()
                .to_str()
                .unwrap()
                .into(),
        );
        let r = refs
            .iter()
            .find(|r| {
                number(field(r, "start")) == start
                    && field(&files[number(field(r, "file"))], "path") == &path
            })
            .unwrap();
        assert_eq!(
            &text[number(field(r, "start"))..number(field(r, "end"))],
            name
        );
        let d = &declarations[number(field(r, "definition"))];
        assert_eq!(field(d, "kind"), &JsonValue::Int(kind));
        assert_eq!(field(d, "name"), &JsonValue::String(name.into()));
        assert_eq!(
            field(&files[number(field(d, "file"))], "path"),
            &JsonValue::String(
                std::fs::canonicalize(f.0.join(expected_file))
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .into()
            )
        );
    }
}
#[test]
fn inaccessible_or_unimported_type_annotations_produce_no_navigation() {
    let f = Fixture::new();
    std::fs::write(
        f.0.join("helper/values.svr"),
        "mod values {export struct Box<T>{value:T}struct Secret<T>{value:T}}",
    )
    .unwrap();
    for source in [
        "use helper::values;fn hidden<T>(x:helper::values::Secret<T>){}fn main(){}",
        "fn hidden<T>(x:helper::values::Box<T>){}fn main(){}",
        "use helper::values;fn hidden<T>(x:T){let y:helper::values::Secret<T>=x;}fn main(){}",
        "fn hidden<T>(x:T){let y:helper::values::Box<T>=x;}fn main(){}",
    ] {
        assert!(check_generic_package_overlays_with_symbols(
            f.0.join("app"),
            &[f.overlay("app/main.svr", source)],
            SourceOverlayLimits::DEFAULT,
            GenericSymbolLimits::DEFAULT
        )
        .is_err());
    }
}

#[test]
fn package_locals_preserve_shadowing_writes_and_function_ownership() {
    let f = Fixture::new();
    let source = "// λ😀\r\nfn work(x:Int)->Int{let mut value=x;if true{let value=true;let inner=value;}value=value+1;return value;}fn other(x:Int)->Int{return x;}fn main(){let result=work(1);}";
    let meta = check_generic_package_overlays_with_symbols(
        f.0.join("app"),
        &[f.overlay("app/main.svr", source)],
        SourceOverlayLimits::DEFAULT,
        GenericSymbolLimits::DEFAULT,
    )
    .unwrap();
    let meta = parse_json(meta.as_bytes(), JsonLimits::DEFAULT).unwrap();
    let locals = array(field(&meta, "locals"));
    let refs = array(field(&meta, "local_references"));
    let files = array(field(&meta, "files"));
    let path = JsonValue::String(
        std::fs::canonicalize(f.0.join("app/main.svr"))
            .unwrap()
            .to_str()
            .unwrap()
            .into(),
    );
    for (marker, offset, declaration, label) in [
        ("value=x", 6, "work(x", "parameter x: Int"),
        ("inner=value", 6, "value=true", "let value: Bool"),
        ("value=value+1", 0, "value=x", "let mut value: Int"),
        ("value=value+1", 6, "value=x", "let mut value: Int"),
        ("return value", 7, "value=x", "let mut value: Int"),
        ("return x", 7, "other(x", "parameter x: Int"),
    ] {
        let start = source.find(marker).unwrap() + offset;
        let reference = refs
            .iter()
            .find(|r| {
                number(field(r, "start")) == start
                    && field(&files[number(field(r, "file"))], "path") == &path
            })
            .unwrap();
        let target = &locals[number(field(reference, "definition"))];
        let expected = source.find(declaration).unwrap()
            + if declaration.ends_with("(x") {
                declaration.len() - 1
            } else {
                0
            };
        assert_eq!(number(field(target, "start")), expected);
        assert_eq!(field(target, "signature"), &JsonValue::String(label.into()));
        assert_eq!(field(reference, "owner"), field(target, "owner"));
        assert_eq!(field(reference, "file"), field(target, "file"));
    }
}

#[test]
fn package_field_reads_retain_nominal_identity_through_groups_and_chains() {
    let f = Fixture::new();
    let provider = "// λ😀\r\nmod values {export struct Box<T>{value:T}}";
    std::fs::write(f.0.join("helper/values.svr"), provider).unwrap();
    let source = "// λ😀\r\nuse helper::values;struct Box<T>{value:T}fn inspect<T>(a:Box<T>,b:helper::values::Box<T>){let own=a.value;let other=(b.value);let nested=helper::values::Box::<Box<T>>{value:a};let chain=nested.value.value;}fn main(){}";
    let metadata = check_generic_package_overlays_with_symbols(
        f.0.join("app"),
        &[f.overlay("app/main.svr", source)],
        SourceOverlayLimits::DEFAULT,
        GenericSymbolLimits::DEFAULT,
    )
    .unwrap();
    let metadata = parse_json(metadata.as_bytes(), JsonLimits::DEFAULT).unwrap();
    assert_eq!(field(&metadata, "unavailable"), &JsonValue::Bool(false));
    let declarations = array(field(&metadata, "declarations"));
    let references = array(field(&metadata, "references"));
    let files = array(field(&metadata, "files"));
    for (marker, offset, target_file, target_source) in [
        ("a.value", 2, "app/main.svr", source),
        ("{value:a}", 1, "helper/values.svr", provider),
        ("b.value", 2, "helper/values.svr", provider),
        ("nested.value.value", 7, "helper/values.svr", provider),
        ("nested.value.value", 13, "app/main.svr", source),
    ] {
        let at = source.find(marker).unwrap() + offset;
        let reference = references
            .iter()
            .find(|r| {
                number(field(r, "start")) == at
                    && field(&files[number(field(r, "file"))], "path")
                        == &JsonValue::String(
                            std::fs::canonicalize(f.0.join("app/main.svr"))
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .into(),
                        )
            })
            .unwrap();
        let target = &declarations[number(field(reference, "definition"))];
        assert_eq!(field(target, "kind"), &JsonValue::Int(8));
        assert_eq!(
            number(field(target, "name_start")),
            target_source.find("value:T").unwrap()
        );
        assert_eq!(
            field(target, "signature"),
            &JsonValue::String("value:T".into())
        );
        assert_eq!(
            field(&files[number(field(target, "file"))], "path"),
            &JsonValue::String(
                std::fs::canonicalize(f.0.join(target_file))
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .into()
            )
        );
    }
}

#[test]
fn reordered_constructor_labels_and_field_quotas_preserve_complete_projection() {
    let f = Fixture::new();
    let source = "struct Pair<T>{left:T,right:T}fn main(){let pair=Pair::<Int>{right // λ\n :(1),left:((2))};let x=pair.left;}";
    let project = |limits| {
        check_generic_package_overlays_with_symbols(
            f.0.join("app"),
            &[f.overlay("app/main.svr", source)],
            SourceOverlayLimits::DEFAULT,
            limits,
        )
        .unwrap()
    };
    let metadata = project(GenericSymbolLimits::DEFAULT);
    let metadata = parse_json(metadata.as_bytes(), JsonLimits::DEFAULT).unwrap();
    assert_eq!(field(&metadata, "unavailable"), &JsonValue::Bool(false));
    let declarations = array(field(&metadata, "declarations"));
    let references = array(field(&metadata, "references"));
    for (marker, offset, name) in [
        ("right //", 0, "right"),
        ("left:((", 0, "left"),
        ("pair.left", 5, "left"),
    ] {
        let reference = references
            .iter()
            .find(|r| number(field(r, "start")) == source.find(marker).unwrap() + offset)
            .unwrap();
        let target = &declarations[number(field(reference, "definition"))];
        assert_eq!(field(target, "kind"), &JsonValue::Int(8));
        assert_eq!(field(target, "name"), &JsonValue::String(name.into()));
        assert_eq!(
            number(field(target, "name_start")),
            source.find(&format!("{name}:T")).unwrap()
        );
    }
    for reduced in [
        GenericSymbolLimits {
            max_declarations: declarations.len() - 1,
            ..GenericSymbolLimits::DEFAULT
        },
        GenericSymbolLimits {
            max_references: references.len() - 1,
            ..GenericSymbolLimits::DEFAULT
        },
    ] {
        assert_eq!(
            project(reduced),
            "{\"schema_version\":1,\"unavailable\":true}"
        );
    }
}
