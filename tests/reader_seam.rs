//! The seam holds: one file in weeder names `tilth_core`, and everything weeder
//! decides with is plain data that compiles without a parser behind it.
//!
//! The source tree is the subject here, so these cases read it. A rule that
//! reached for tilth directly, or a core type that carried a tree-sitter node,
//! would still pass every other test in this repository and quietly cost weeder
//! the ability to swap the dependency.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use weeder::core::read::{
    CallerSite, Definition, DefinitionKind, Import, Outline, TestShape, TestUnit, TestUnitKind,
};
use weeder::seams::reader;

/// The one module allowed to name the parsing substrate.
const SEAM: &str = "src/seams/reader.rs";

/// Every Rust source file weeder ships, as a path relative to the repository root.
fn sources(under: &str) -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    let mut stack = vec![root.join(under)];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("a source directory weeder ships should be readable")
        {
            let path = entry.expect("a source entry should be readable").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                let relative = path
                    .strip_prefix(root)
                    .expect("every source sits under the repository root");
                found.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    found.sort();
    found
}

fn contents(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    fs::read_to_string(&path).expect("a source file weeder ships should be readable")
}

#[test]
fn the_reader_is_the_only_module_that_names_tilth_core() {
    let naming: Vec<String> = sources("src")
        .into_iter()
        .filter(|relative| {
            let source = contents(relative);
            source.contains("tilth_core") || source.contains("tilth-core")
        })
        .collect();

    assert_eq!(
        naming,
        vec![SEAM.to_string()],
        "the parsing substrate is named in one place, so an upgrade is one file"
    );
}

#[test]
fn the_core_carries_no_parser() {
    for relative in sources("src/core") {
        let source = contents(&relative);
        for forbidden in ["tilth_core", "tilth-core", "tree_sitter", "tree-sitter"] {
            assert!(
                !source.contains(forbidden),
                "{relative} names `{forbidden}`; the core decides on data the reader hands it"
            );
        }
    }
}

#[test]
fn the_readers_data_types_need_nothing_but_themselves() {
    let outline = Outline {
        definitions: vec![Definition {
            kind: DefinitionKind::Class,
            name: "Basket".to_string(),
            start_line: 4,
            end_line: 20,
            signature: None,
            children: vec![Definition {
                kind: DefinitionKind::Function,
                name: "total".to_string(),
                start_line: 8,
                end_line: 12,
                signature: Some("def total(self)".to_string()),
                children: Vec::new(),
            }],
        }],
    };

    let total = outline
        .find("total")
        .expect("find reaches a definition nested inside another");
    assert_eq!(total.kind, DefinitionKind::Function);
    assert!(total.spans(8) && total.spans(12) && !total.spans(13));
    assert_eq!(
        outline
            .flatten()
            .iter()
            .map(|definition| definition.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Basket", "total"],
        "flatten walks the tree in source order, parents before children"
    );

    let shape = TestShape {
        units: vec![
            TestUnit {
                kind: TestUnitKind::Suite,
                name: "basket".to_string(),
                start_line: 1,
                end_line: 9,
                depth: 0,
                attributes: Vec::new(),
            },
            TestUnit {
                kind: TestUnitKind::Case,
                name: "adds up".to_string(),
                start_line: 2,
                end_line: 8,
                depth: 1,
                attributes: Vec::new(),
            },
        ],
    };
    assert_eq!(
        shape
            .cases()
            .map(|case| case.name.as_str())
            .collect::<Vec<_>>(),
        vec!["adds up"],
        "a suite is not a case: a rule that counts tests counts the cases"
    );

    let import = Import {
        start_line: 1,
        end_line: 1,
        text: "import { basket } from \"./basket\";".to_string(),
        source: "./basket".to_string(),
        external: false,
    };
    assert_eq!(import.source, "./basket");

    let site = CallerSite {
        symbol: "total".to_string(),
        path: PathBuf::from("src/checkout.ts"),
        line: 12,
        calling_function: "checkout".to_string(),
        call_text: "const owed = total(basket);".to_string(),
    };
    assert_eq!(site.line, 12);
}

#[test]
fn what_the_reader_returns_is_that_same_data() {
    // Typed on the core's own shapes: were the seam to hand back anything
    // tilth-shaped, this would not compile.
    fn definitions(outline: Outline) -> usize {
        outline.flatten().len()
    }
    fn cases(shape: TestShape) -> usize {
        shape.cases().count()
    }
    fn first_source(imports: Vec<Import>) -> String {
        imports
            .first()
            .map(|import| import.source.clone())
            .unwrap_or_default()
    }
    fn lines(sites: Vec<CallerSite>) -> Vec<u32> {
        sites.iter().map(|site| site.line).collect()
    }

    let path = Path::new("basket.ts");
    let source = "import { price } from \"./price\";\n\nexport function total(items: string[]): number {\n  return items.length * price;\n}\n\ndescribe(\"total\", () => {\n  it(\"counts the items\", () => {\n    expect(total([\"a\"])).toBe(price);\n  });\n});\n";

    assert_eq!(definitions(reader::outline(path, source)), 2);
    assert_eq!(cases(reader::test_shape(path, source)), 1);
    assert_eq!(first_source(reader::imports(path, source)), "./price");

    let scope = tempfile::tempdir().expect("a directory to search");
    fs::write(scope.path().join("price.ts"), "export const price = 2;\n")
        .expect("the fixture writes");
    fs::write(
        scope.path().join("basket.ts"),
        "import { price } from \"./price\";\n\nexport function total(n: number): number {\n  return n * price;\n}\n",
    )
    .expect("the fixture writes");
    let targets: BTreeSet<String> = ["total".to_string()].into_iter().collect();
    assert!(
        lines(reader::callers(&targets, scope.path()).expect("a searchable directory")).is_empty(),
        "nothing calls total here; a definition is not a call site"
    );
}

#[test]
fn a_statement_naming_several_modules_is_one_import_per_module() {
    // A statement is read for every module it names, each on the line that
    // names it: the source at the end of a TypeScript clause, every member of a
    // Rust group, every path in a Go block (#124).
    let typescript = reader::imports(
        Path::new("basket.ts"),
        "import {\n  price,\n  tax,\n} from \"./price\";\n\nexport const rate = tax;\n",
    );
    let read: Vec<(u32, &str, bool)> = typescript
        .iter()
        .map(|import| (import.start_line, import.source.as_str(), import.external))
        .collect();
    assert_eq!(read, [(4, "./price", false)]);

    let rust = reader::imports(
        Path::new("basket.rs"),
        "use crate::core::{\n    price::Price,\n    tax::Tax,\n};\n\npub fn rate() -> Tax {\n    Tax::default()\n}\n",
    );
    let read: Vec<(u32, &str, bool)> = rust
        .iter()
        .map(|import| (import.start_line, import.source.as_str(), import.external))
        .collect();
    assert_eq!(
        read,
        [
            (2, "crate::core::price::Price", false),
            (3, "crate::core::tax::Tax", false)
        ]
    );

    // Go's module paths read as external, which is all a file alone says
    // about them; the module each line names is its own import all the same.
    let go = reader::imports(
        Path::new("basket.go"),
        "package basket\n\nimport (\n\t\"fmt\"\n\t\"strings\"\n)\n\nfunc Rate() string {\n\treturn fmt.Sprint(strings.TrimSpace(\" x \"))\n}\n",
    );
    let read: Vec<(u32, &str, bool)> = go
        .iter()
        .map(|import| (import.start_line, import.source.as_str(), import.external))
        .collect();
    assert_eq!(read, [(4, "fmt", true), (5, "strings", true)]);
    assert!(go.iter().all(|import| import.start_line == import.end_line));
    assert_eq!(go[0].text, "\"fmt\"");
}

#[test]
fn a_test_is_told_from_a_helper_that_shares_its_prefix() {
    // Go: `go test` runs a function named for a test that takes the handle
    // that runs it. A helper whose name starts the same way is not a test.
    let go = reader::test_shape(
        Path::new("basket_test.go"),
        "package basket\n\nimport \"testing\"\n\nfunc TestData() string {\n\treturn \"x\"\n}\n\nfunc TestRate(t *testing.T) {\n\tif TestData() == \"\" {\n\t\tt.Fatal(\"no data\")\n\t}\n}\n",
    );
    assert_eq!(
        go.cases()
            .map(|case| case.name.as_str())
            .collect::<Vec<_>>(),
        ["TestRate"]
    );

    // Rust: the attribute is what makes a test, not the name or the module.
    let rust = reader::test_shape(
        Path::new("basket.rs"),
        "#[cfg(test)]\nmod tests {\n    fn test_helper() -> u8 {\n        1\n    }\n\n    #[test]\n    fn adds_up() {\n        assert_eq!(test_helper(), 1);\n    }\n}\n",
    );
    assert_eq!(
        rust.units
            .iter()
            .map(|unit| (unit.kind, unit.name.as_str(), unit.depth))
            .collect::<Vec<_>>(),
        [
            (TestUnitKind::Suite, "tests", 0),
            (TestUnitKind::Case, "adds_up", 1)
        ]
    );

    // The attributes above a Rust test travel with it, so a rule can read a
    // case declared to expect a failure.
    let panicking = reader::test_shape(
        Path::new("basket.rs"),
        "#[cfg(test)]\nmod tests {\n    #[test]\n    // the empty basket is refused\n    #[should_panic(expected = \"empty\")]\n    fn refuses_nothing() {\n        total(&[]);\n    }\n}\n",
    );
    assert_eq!(
        panicking
            .units
            .iter()
            .map(|unit| (unit.name.as_str(), unit.attributes.clone()))
            .collect::<Vec<_>>(),
        [
            ("tests", vec!["cfg(test)".to_string()]),
            (
                "refuses_nothing",
                vec![
                    "should_panic(expected = \"empty\")".to_string(),
                    "test".to_string()
                ]
            )
        ]
    );
}
