//! The reader on real files, one small pair per language: what weeder sees when
//! it reads a module and the test file that exercises it.
//!
//! The fixtures live in `fixtures/reader/<lang>/` and every number here is read
//! off them, so a change in what tilth-core returns shows up as a failure with
//! the line it disagrees about rather than as a rule that quietly stops firing.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use weeder::core::classify::Lang;
use weeder::core::read::DefinitionKind;
use weeder::seams::reader;

fn fixture(lang: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/reader")
        .join(lang)
}

fn source_of(path: &Path) -> String {
    std::fs::read_to_string(path).expect("a fixture file should be readable")
}

fn targets(name: &str) -> BTreeSet<String> {
    [name.to_string()].into_iter().collect()
}

/// The outline, flattened to `kind name start-end`, which is the whole of what
/// a rule reads out of a file's structure.
fn shape_of(path: &Path, source: &str) -> Vec<String> {
    reader::outline(path, source)
        .flatten()
        .iter()
        .map(|definition| {
            format!(
                "{:?} {} {}-{}",
                definition.kind, definition.name, definition.start_line, definition.end_line
            )
        })
        .collect()
}

/// The test shape, flattened to `kind name start-end depth`.
fn tests_of(path: &Path, source: &str) -> Vec<String> {
    reader::test_shape(path, source)
        .units
        .iter()
        .map(|unit| {
            format!(
                "{:?} {} {}-{} @{}",
                unit.kind, unit.name, unit.start_line, unit.end_line, unit.depth
            )
        })
        .collect()
}

/// The imports, flattened to `line source external`.
fn imports_of(path: &Path, source: &str) -> Vec<String> {
    reader::imports(path, source)
        .iter()
        .map(|import| {
            format!(
                "{} {} {}",
                import.start_line,
                import.source,
                if import.external { "external" } else { "local" }
            )
        })
        .collect()
}

/// The call sites of `name` under `scope`, flattened to `file:line in caller`.
fn callers_of(name: &str, scope: &Path) -> Vec<String> {
    reader::callers(&targets(name), scope)
        .expect("a directory of fixtures is searchable")
        .iter()
        .map(|site| {
            // The seam names a site relative to the scope it searched, joined
            // with `/` on every platform; the test reads the spelling as given.
            assert!(
                site.path.is_relative(),
                "a call site is named relative to the scope: {}",
                site.path.display()
            );
            format!(
                "{}:{} in {}",
                site.path.display(),
                site.line,
                site.calling_function
            )
        })
        .collect()
}

#[test]
fn typescript_reads_as_a_module_and_the_spec_beside_it() {
    let dir = fixture("ts");
    let module = dir.join("greet.ts");
    let spec = dir.join("greet.test.ts");
    let module_source = source_of(&module);
    let spec_source = source_of(&spec);

    assert_eq!(reader::language(&module), Lang::TypeScript);
    assert_eq!(
        shape_of(&module, &module_source),
        [
            "Import import { formatName } from \"./name\"; 1-1",
            "Function greet 3-5",
        ]
    );
    let outline = reader::outline(&module, &module_source);
    let greet = outline.find("greet").expect("the outline names the export");
    assert_eq!(greet.kind, DefinitionKind::Function);
    assert_eq!(
        greet.signature.as_deref(),
        Some("export function greet(name: string): string")
    );
    assert!(greet.spans(4), "the body of greet is inside greet");

    assert_eq!(imports_of(&module, &module_source), ["1 ./name local"]);
    assert_eq!(
        reader::related_files(&module, &module_source),
        vec![dir.join("name.ts")],
        "a relative import resolves to the file it names"
    );

    assert_eq!(
        tests_of(&spec, &spec_source),
        ["Suite greet 3-7 @0", "Case greets by name 4-6 @1"]
    );
    assert!(
        tests_of(&module, &module_source).is_empty(),
        "a module that declares no tests has no test shape"
    );
    assert_eq!(
        reader::test_shape(&spec, &spec_source)
            .cases()
            .map(|case| case.name.as_str())
            .collect::<Vec<_>>(),
        ["greets by name"]
    );

    // The call sits in an arrow function passed to `it`, which has no name of
    // its own; tilth spells that context `<top-level>`.
    assert_eq!(
        callers_of("greet", &dir),
        ["greet.test.ts:5 in <top-level>"]
    );
}

#[test]
fn python_reads_as_a_module_and_its_pytest_file() {
    let dir = fixture("py");
    let module = dir.join("greet.py");
    let spec = dir.join("test_greet.py");
    let module_source = source_of(&module);
    let spec_source = source_of(&spec);

    assert_eq!(reader::language(&module), Lang::Python);
    assert_eq!(
        shape_of(&module, &module_source),
        ["Import import textwrap 1-1", "Function greet 4-5"]
    );
    assert_eq!(
        reader::outline(&module, &module_source)
            .find("greet")
            .and_then(|definition| definition.signature.clone()),
        Some("def greet(name)".to_string())
    );
    assert_eq!(imports_of(&module, &module_source), ["1 textwrap external"]);

    // Python's absolute imports need the interpreter's path to resolve, so
    // tilth calls a bare module name external even where a sibling file
    // answers to it.
    assert_eq!(imports_of(&spec, &spec_source), ["1 greet external"]);

    assert_eq!(
        tests_of(&spec, &spec_source),
        [
            "Suite TestGreet 4-6 @0",
            "Case test_greets_by_name 5-6 @1",
            "Case test_greets_the_world 9-10 @0",
        ]
    );
    assert!(tests_of(&module, &module_source).is_empty());

    assert_eq!(
        callers_of("greet", &dir),
        [
            "test_greet.py:6 in TestGreet.test_greets_by_name",
            "test_greet.py:10 in test_greets_the_world",
        ]
    );
}

#[test]
fn rust_reads_as_a_module_with_its_tests_inside_it() {
    let dir = fixture("rs");
    let module = dir.join("greet.rs");
    let caller = dir.join("main.rs");
    let module_source = source_of(&module);
    let caller_source = source_of(&caller);

    assert_eq!(reader::language(&module), Lang::Rust);
    assert_eq!(
        shape_of(&module, &module_source),
        [
            "Import use std::fmt::Write; 1-1",
            "Function greet 3-7",
            "Module tests 10-17",
            "Import use super::greet; 11-11",
            "Function greets_by_name 14-16",
        ]
    );
    assert_eq!(
        reader::outline(&module, &module_source)
            .find("greets_by_name")
            .map(|definition| definition.start_line),
        Some(14),
        "a definition inside a module is still found by name"
    );

    assert_eq!(
        imports_of(&module, &module_source),
        ["1 std::fmt::Write external", "11 super::greet local"]
    );
    assert_eq!(
        imports_of(&caller, &caller_source),
        ["1 crate::greet::greet local"]
    );

    // Rust marks its tests with an attribute rather than a call, and the
    // module that holds them with `#[cfg(test)]`.
    assert_eq!(
        tests_of(&module, &module_source),
        ["Suite tests 10-17 @0", "Case greets_by_name 14-16 @1"]
    );
    assert!(tests_of(&caller, &caller_source).is_empty());

    assert_eq!(callers_of("greet", &dir), ["main.rs:4 in main"]);
}

#[test]
fn go_reads_as_a_package_and_its_test_file() {
    let dir = fixture("go");
    let module = dir.join("greet.go");
    let spec = dir.join("greet_test.go");
    let module_source = source_of(&module);
    let spec_source = source_of(&spec);

    assert_eq!(reader::language(&module), Lang::Go);
    assert_eq!(
        shape_of(&module, &module_source),
        ["Import import \"strings\" 3-3", "Function Greet 6-8"]
    );
    assert_eq!(
        reader::outline(&module, &module_source)
            .find("Greet")
            .and_then(|definition| definition.signature.clone()),
        Some("func Greet(name string) string".to_string())
    );

    // Go's module paths need the build system to resolve, so every import
    // reads as external.
    assert_eq!(imports_of(&module, &module_source), ["3 strings external"]);
    assert_eq!(imports_of(&spec, &spec_source), ["3 testing external"]);

    // A Go test is a function named for what it tests, taking the testing
    // handle; there is no suite around it.
    assert_eq!(tests_of(&spec, &spec_source), ["Case TestGreet 5-9 @0"]);
    assert!(tests_of(&module, &module_source).is_empty());

    assert_eq!(
        callers_of("Greet", &dir),
        [
            "greet_test.go:6 in TestGreet",
            "greet_test.go:7 in TestGreet"
        ]
    );
}
