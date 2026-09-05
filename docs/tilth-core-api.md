# tilth-core — the surface weed needs

Sent to almaty (the tilth agent) on 2026-09-05 as the request for the `tilth-core` crate. Signatures name tilth's own items on origin/main v0.10.1 (f5c0afa); "make pub" means the item exists and only its visibility changes. Two items are additive and marked so. Nothing here asks for a behaviour change in tilth.

## 1. File type and language

```rust
pub fn detect_file_type(path: &Path) -> FileType;      // src/lang/mod.rs, already pub
pub enum FileType { Code(Lang), Markdown, StructuredData, Tabular, Log, Other }
pub enum Lang { Rust, TypeScript, Tsx, JavaScript, Python, Go, ... }   // src/types.rs
```

weed matches on `FileType::Code(Lang::{TypeScript, Tsx, JavaScript, Python, Rust, Go})` and treats everything else as a language it has no rules for.

## 2. Outline entries

```rust
pub fn get_outline_entries(content: &str, lang: Lang) -> Vec<OutlineEntry>;   // src/lang/outline.rs, already pub
pub struct OutlineEntry { pub kind: OutlineKind, pub name: String, pub start_line: u32, pub end_line: u32, pub signature: Option<String>, pub children: Vec<OutlineEntry>, pub doc: Option<String> }
pub enum OutlineKind { Import, Function, Class, Struct, Interface, TypeAlias, Enum, Constant, Variable, ImmutableVariable, Export, Property, Module, TestSuite, TestCase }
```

Used for: counting and naming definitions in a changed file (T1 case counts in py/rs/go, S1 body-only stubs, R2 dead exports, X2 blast radius). The `doc` and `signature` fields are welcome but not required.

## 3. Test-file classification

```rust
pub fn is_test_file(path: &Path) -> bool;   // src/types.rs, make pub
```

Today it matches `.test.`, `.spec.` and `__tests__/`. That is the JavaScript convention; weed layers `test_*.py`, `*_test.py`, `*_test.go`, `tests/` and `#[cfg(test)]` on top in its own classifier, so no change is requested here.

## 4. Test structure (additive)

`src/read/outline/test_file.rs` already walks `describe`/`context`/`it`/`test`/`specify` calls and renders them as indented text. weed needs the same walk as data:

```rust
pub struct TestEntry { pub name: String, pub kind: TestKind, pub start_line: u32, pub end_line: u32, pub depth: u8 }
pub enum TestKind { Suite, Case }
pub fn test_entries(content: &str, lang: Lang) -> Vec<TestEntry>;   // new, additive; the existing `outline` can be expressed over it
```

For Python, Rust and Go weed derives cases from outline entries (functions named `test_*`, functions preceded by `#[test]`, functions named `Test*` taking `*testing.T`), so `test_entries` may return empty for those languages without harm. If `#[test]` attributes are not visible through `OutlineEntry`, say so; weed will read the lines above a function's `start_line` itself.

## 5. Imports

```rust
pub fn is_import_line(line: &str, lang: Lang) -> bool;                                   // src/read/imports.rs, make pub
pub fn is_external(source: &str, lang: Lang) -> bool;                                    // src/read/imports.rs, make pub
pub fn extract_import_source(text: &str, lang: Option<Lang>) -> String;                  // src/lang/outline.rs, make pub
pub fn resolve_related_files_with_content(file_path: &Path, content: &str) -> Vec<PathBuf>;  // src/read/imports.rs, already pub
```

Used for: D2 dependency direction (which layer a changed file imports from), M1 (which module a mock specifier resolves to), R1 (a cited path that no longer resolves).

## 6. Callers and dependents

```rust
pub struct CallerMatch { pub path: PathBuf, pub line: u32, pub calling_function: String, pub call_text: String, pub caller_range: Option<(u32, u32)>, pub content: Arc<String> }
pub fn find_callers_batch(targets: &HashSet<String>, scope: &Path, bloom: &BloomFilterCache, glob: Option<&str>, early_quit_threshold: usize) -> Result<Vec<(String, CallerMatch)>, TilthError>;   // src/search/callers.rs, make pub, and make BloomFilterCache constructible from outside (`BloomFilterCache::new()` pub)
pub fn analyze_deps(path: &Path, scope: &Path, bloom: &BloomFilterCache) -> Result<DepsResult, TilthError>;   // src/search/deps.rs, already pub
pub struct DepsResult { pub target: PathBuf, pub uses_local: Vec<LocalDep>, pub uses_external: Vec<String>, pub used_by: Vec<Dependent>, pub total_dependents: usize, pub exported_count: usize, pub searched_count: usize }
```

Used for: X2 naming the blast radius of an out-of-scope change, R2 dead exports (a definition with zero callers in scope), D2 through `uses_local`. `TilthError` needs to be public and implement `std::error::Error` so weed can wrap it.

## 7. Error type

`TilthError` (src/error.rs) as the crate's error, public, `Error + Send + Sync + 'static`.

## Acceptance test weed will run against the crate

`crates/tilth-core/tests/api.rs` in tilth (or `tests/reader_langs.rs` in weed if you prefer to keep tilth's tests to its own concerns): for each of the four languages, a 20-line inline sample with one function, one import, and one test case; assert `detect_file_type` names the language, `get_outline_entries` contains the function by name with a plausible line range, `is_import_line` is true for the import line and `extract_import_source` returns its source, and for TypeScript `test_entries` returns one suite containing one case with the right lines. For callers: a two-file temp dir where `b` calls `a::f`; `find_callers_batch({"f"}, dir, ...)` returns one match in `b` with the right line.

## How weed depends on it

`Cargo.toml`: `tilth-core = { git = "https://github.com/jahala/tilth", rev = "<the commit you name>" }`. Until that commit is on GitHub, `.cargo/config.toml` in weed patches the dependency to a local checkout of your branch. Version policy: lockstep with tilth is fine; weed pins a rev either way.
