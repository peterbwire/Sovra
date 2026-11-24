//! Independent Sovra association-list snapshots; not opaque intrinsic std Map/Set.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use sovra::compiler::{project::packages::*, v2::*};
use std::{
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "sovra snapshot collections λ {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("app")).unwrap();
        std::fs::create_dir_all(root.join("collections")).unwrap();
        for name in ["sovra.toml", "main.svr", "maps.svr", "sets.svr"] {
            std::fs::copy(
                std::path::Path::new("stdlib/snapshot-collections").join(name),
                root.join("collections").join(name),
            )
            .unwrap();
        }
        std::fs::write(root.join("app/sovra.toml"),"[project]\nname=\"consumer\"\nentry=\"main.svr\"\n[dependencies.collections]\npath=\"../collections\"\n").unwrap();
        Self(root)
    }
    fn source(&self, source: &str) {
        std::fs::write(
            self.0.join("app/main.svr"),
            format!("use collections::maps;use collections::sets;{source}"),
        )
        .unwrap();
    }
    fn compile(&self) -> Result<CompiledGenericPackage, sovra::compiler::diagnostics::Diagnostics> {
        compile_generic_package(
            capture_package_sources(self.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            "main",
            GenericPackageCompilationLimits::DEFAULT,
        )
    }
    fn result(&self, source: &str) -> GenericVmOutcome {
        self.source(source);
        execute_generic_package(&self.compile().unwrap(), GenericVmPolicy::DEFAULT).unwrap()
    }
    fn ok(&self, body: &str) {
        let source = format!("fn main()->std::Result<Bool,std::Error>{{{body}}}");
        let out = self.result(&source);
        assert_eq!(out.value().enum_tag(), Some(0), "{out:?}");
        assert_eq!(out.value().payload().unwrap().as_bool(), Some(true));
    }
    fn cli(&self, command: &str, flags: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_svr"))
            .arg(command)
            .arg("--experimental-generics")
            .args(flags)
            .arg(self.0.join("app"))
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn insertion_replacement_and_ordered_snapshots_preserve_old_maps() {
    let f = Fixture::new();
    f.ok("let empty=collections::maps::map_new::<String,Int>();let a=collections::maps::map_insert::<String,Int>(empty,\"a\",1)?;let b=collections::maps::map_insert::<String,Int>(a,\"b\",2)?;let c=collections::maps::map_insert::<String,Int>(b,\"a\",3)?;let keys=collections::maps::map_keys::<String,Int>(c)?;let values=collections::maps::map_values::<String,Int>(c)?;let wanted_keys:[String]=[\"a\",\"b\"];let wanted_values:[Int]=[3,2];let size=collections::maps::map_len::<String,Int>(c)?;let old_size=collections::maps::map_len::<String,Int>(empty)?;let contains=collections::maps::map_contains::<String,Int>(c,\"a\")?;let missing=collections::maps::map_contains::<String,Int>(c,\"x\")?;return std::Result::<Bool,std::Error>::Ok(keys==wanted_keys&&values==wanted_values&&size==2&&old_size==0&&contains&&missing==false);");
}
#[test]
fn missing_get_remove_and_reinsertion_have_explicit_option_outcomes() {
    let f = Fixture::new();
    f.ok("let empty=collections::maps::map_new::<Int,String>();let a=collections::maps::map_insert::<Int,String>(empty,1,\"one\")?;let b=collections::maps::map_insert::<Int,String>(a,2,\"two\")?;let removed=collections::maps::map_remove::<Int,String>(b,1)?;match removed {std::Option::Some(change)=>{let again=collections::maps::map_insert::<Int,String>(change.map,1,\"new\")?;let keys=collections::maps::map_keys::<Int,String>(again)?;let missing=collections::maps::map_get::<Int,String>(again,99)?;let no_remove=collections::maps::map_remove::<Int,String>(empty,99)?;match missing {std::Option::Some(value)=>{return std::Result::<Bool,std::Error>::Ok(false);},std::Option::None()=>{match no_remove {std::Option::Some(value)=>{return std::Result::<Bool,std::Error>::Ok(false);},std::Option::None()=>{return std::Result::<Bool,std::Error>::Ok(keys[0]==2&&keys[1]==1&&change.value==\"one\");}}}}},std::Option::None()=>{return std::Result::<Bool,std::Error>::Ok(false);}}");
}
#[test]
fn logical_equality_ignores_insertion_order_but_raw_record_equality_remains_structural() {
    let f = Fixture::new();
    f.ok("let e=collections::maps::map_new::<String,Int>();let a1=collections::maps::map_insert::<String,Int>(e,\"x\",1)?;let a=collections::maps::map_insert::<String,Int>(a1,\"y\",2)?;let b1=collections::maps::map_insert::<String,Int>(e,\"y\",2)?;let b=collections::maps::map_insert::<String,Int>(b1,\"x\",1)?;let c=collections::maps::map_insert::<String,Int>(b,\"x\",3)?;let same=collections::maps::map_equal::<String,Int>(a,b)?;let different=collections::maps::map_equal::<String,Int>(a,c)?;return std::Result::<Bool,std::Error>::Ok(same&&different==false&&(a==b)==false);");
}
#[test]
fn nested_values_and_returned_entries_are_independent_snapshots() {
    let f = Fixture::new();
    f.ok("let mut original:[Int]=[1];let e=collections::maps::map_new::<String,[Int]>();let m=collections::maps::map_insert::<String,[Int]>(e,\"k\",original)?;original[0]=9;let entries=collections::maps::map_entries::<String,[Int]>(m)?;let mut copy=entries;copy[0]=collections::maps::Entry::<String,[Int]>{key:\"k\",value:original};let got=collections::maps::map_get::<String,[Int]>(m,\"k\")?;match got {std::Option::Some(value)=>{let mut v=value;v[0]=8;let second=collections::maps::map_get::<String,[Int]>(m,\"k\")?;match second {std::Option::Some(still)=>{return std::Result::<Bool,std::Error>::Ok(still[0]==1&&v[0]==8&&copy[0].value[0]==9);},std::Option::None()=>{return std::Result::<Bool,std::Error>::Ok(false);}}},std::Option::None()=>{return std::Result::<Bool,std::Error>::Ok(false);}}");
}
#[test]
fn sets_keep_duplicate_positions_and_use_membership_equality() {
    let f = Fixture::new();
    f.ok("let e=collections::sets::set_new::<Int>();let a=collections::sets::set_insert::<Int>(e,1)?;let b=collections::sets::set_insert::<Int>(a,2)?;let twice=collections::sets::set_insert::<Int>(b,1)?;let reverse1=collections::sets::set_insert::<Int>(e,2)?;let reverse=collections::sets::set_insert::<Int>(reverse1,1)?;let equal=collections::sets::set_equal::<Int>(twice,reverse)?;let size=collections::sets::set_len::<Int>(twice)?;let values=collections::sets::set_values::<Int>(twice)?;let removed=collections::sets::set_remove::<Int>(twice,1)?;match removed {std::Option::Some(after)=>{let again=collections::sets::set_insert::<Int>(after,1)?;let order=collections::sets::set_values::<Int>(again)?;let absent=collections::sets::set_get::<Int>(again,99)?;match absent {std::Option::Some(value)=>{return std::Result::<Bool,std::Error>::Ok(false);},std::Option::None()=>{return std::Result::<Bool,std::Error>::Ok(equal&&size==2&&values[0]==1&&values[1]==2&&order[0]==2&&order[1]==1);}}},std::Option::None()=>{return std::Result::<Bool,std::Error>::Ok(false);}}");
}
#[test]
fn string_keys_compare_exact_unicode_and_bool_keys_are_supported() {
    let f = Fixture::new();
    f.ok("let e=collections::maps::map_new::<String,Int>();let a=collections::maps::map_insert::<String,Int>(e,\"é\",1)?;let b=collections::maps::map_insert::<String,Int>(a,\"é\",2)?;let c=collections::maps::map_insert::<String,Int>(b,\"\",3)?;let length=collections::maps::map_len::<String,Int>(c)?;let s=collections::sets::set_new::<Bool>();let yes=collections::sets::set_insert::<Bool>(s,true)?;let both=collections::sets::set_insert::<Bool>(yes,false)?;let count=collections::sets::set_len::<Bool>(both)?;let contains=collections::sets::set_contains::<Bool>(both,true)?;return std::Result::<Bool,std::Error>::Ok(length==3&&count==2&&contains);");
}
#[test]
fn malformed_user_constructed_records_return_structured_errors_without_arbitrary_first_wins() {
    let f = Fixture::new();
    let code="fn code<T>(r:std::Result<T,std::Error>)->String{match r {std::Result::Ok(value)=>{return \"\";},std::Result::Err(error)=>{return error.code;}}}";
    let bad="let one=collections::maps::Entry::<Int,Int>{key:1,value:1};let two=collections::maps::Entry::<Int,Int>{key:1,value:2};let entries:[collections::maps::Entry<Int,Int>]=[one,two];let bad=collections::maps::SnapshotMap::<Int,Int>{entries:entries};let good=collections::maps::map_new::<Int,Int>();";
    for (kind, expression) in [
        ("Int", "collections::maps::map_len::<Int,Int>(bad)"),
        ("Bool", "collections::maps::map_contains::<Int,Int>(bad,1)"),
        (
            "std::Option<Int>",
            "collections::maps::map_get::<Int,Int>(bad,1)",
        ),
        (
            "collections::maps::SnapshotMap<Int,Int>",
            "collections::maps::map_insert::<Int,Int>(bad,1,3)",
        ),
        (
            "std::Option<collections::maps::Removal<Int,Int>>",
            "collections::maps::map_remove::<Int,Int>(bad,1)",
        ),
        (
            "[collections::maps::Entry<Int,Int>]",
            "collections::maps::map_entries::<Int,Int>(bad)",
        ),
        ("[Int]", "collections::maps::map_keys::<Int,Int>(bad)"),
        ("[Int]", "collections::maps::map_values::<Int,Int>(bad)"),
        ("Bool", "collections::maps::map_equal::<Int,Int>(good,bad)"),
    ] {
        let source=format!("{code}fn main()->Bool{{{bad}return code::<{kind}>({expression})==\"invalid_snapshot_map\";}}");
        assert_eq!(
            f.result(&source).value().as_bool(),
            Some(true),
            "{expression}"
        );
    }
    let out=f.result("fn main()->std::Result<Int,std::Error>{let items:[Int]=[1,1];let bad=collections::sets::SnapshotSet::<Int>{items:items};return collections::sets::set_len::<Int>(bad);}");
    assert_eq!(out.value().enum_tag(), Some(1));
    let error = out.value().payload().unwrap();
    assert_eq!(
        error.field(0).unwrap().as_str(),
        Some("invalid_snapshot_set")
    );
    assert!(error.field(2).unwrap().payload().is_none());
}
#[test]
fn float_and_compound_keys_and_mixed_nominal_arguments_are_rejected() {
    let f = Fixture::new();
    for source in ["fn main(){collections::maps::map_new::<Float,Int>();}","fn main(){collections::sets::set_new::<[Int]>();}","fn main(){let m=collections::maps::map_new::<Int,Int>();collections::maps::map_insert::<Int,Float>(m,1,1.0);}"] {f.source(source);assert!(f.compile().is_err());}
}
#[test]
fn package_type_argument_allowance_is_cumulative_bounded_and_keeps_standalone_defaults() {
    let f = Fixture::new();
    std::fs::write(
        f.0.join("app/sovra.toml"),
        "[project]\nname=\"quota\"\nentry=\"main.svr\"\n",
    )
    .unwrap();
    let source = format!(
        "fn id<T>(x:T)->T{{return x;}}fn main(){{{}}}",
        "id::<Int>(1);".repeat(200)
    );
    std::fs::write(f.0.join("app/main.svr"), &source).unwrap();
    assert!(f.compile().is_ok());
    assert!(
        compile_generic_array_source(&source, "main", GenericPropagationSourceLimits::DEFAULT)
            .is_err()
    );
    let mut limits = GenericPackageCompilationLimits::DEFAULT;
    limits.bodies.calls.arguments.max_arguments = 200;
    assert!(compile_generic_package(
        capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        limits
    )
    .is_ok());
    limits.bodies.calls.arguments.max_arguments = 199;
    assert!(compile_generic_package(
        capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        limits
    )
    .is_err());
    assert_eq!(
        GenericCopyBodyLimits::PACKAGE.calls.syntax.max_arguments,
        GenericCopyBodyLimits::DEFAULT.calls.syntax.max_arguments
    );
}
#[test]
fn work_and_snapshot_materialization_refuse_without_modifying_checked_program() {
    let f = Fixture::new();
    f.source("fn main()->std::Result<Bool,std::Error>{let e=collections::sets::set_new::<Int>();let a=collections::sets::set_insert::<Int>(e,1)?;let b=collections::sets::set_insert::<Int>(a,2)?;return std::Result::<Bool,std::Error>::Ok((collections::sets::set_len::<Int>(b)?)==2);}");
    let p = f.compile().unwrap();
    let mut tiny = GenericVmPolicy::DEFAULT;
    tiny.max_work = 100;
    assert!(execute_generic_package(&p, tiny).is_err());
    let mut memory = GenericVmPolicy::DEFAULT;
    memory.runtime.materialization.max_value_nodes = Some(1);
    assert!(execute_generic_package(&p, memory).is_err());
    let out = execute_generic_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(out.value().payload().unwrap().as_bool(), Some(true));
}
#[test]
fn independent_collection_consumer_passes_locked_cli_suite() {
    let f = Fixture::new();
    let source = std::fs::read_to_string("examples/snapshot-collections/main.svr").unwrap();
    std::fs::write(f.0.join("app/main.svr"), source).unwrap();
    assert!(f.cli("lock", &[]).status.success());
    let lock = std::fs::read(f.0.join("app/sovra.lock")).unwrap();
    for command in ["check", "run", "test"] {
        let out = f.cli(command, &["--locked"]);
        assert!(
            out.status.success(),
            "{} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    assert_eq!(std::fs::read(f.0.join("app/sovra.lock")).unwrap(), lock);
}

#[test]
fn native_collection_consumer_runs_after_sources_are_removed_without_node() {
    let f = Fixture::new();
    std::fs::copy(
        "examples/snapshot-collections/main.svr",
        f.0.join("app/main.svr"),
    )
    .unwrap();
    assert!(f.cli("lock", &[]).status.success());
    let output = f.0.join("application");
    let built = f.cli(
        "build",
        &[
            "--locked",
            "--emit",
            "exe",
            "--output",
            output.to_str().unwrap(),
        ],
    );
    assert!(
        built.status.success(),
        "{} {}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );
    std::fs::remove_dir_all(f.0.join("app")).unwrap();
    std::fs::remove_dir_all(f.0.join("collections")).unwrap();
    let out = Command::new(&output)
        .current_dir(&f.0)
        .env_clear()
        .env("PATH", "/nonexistent")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty());
    assert!(out.stderr.is_empty());
}
