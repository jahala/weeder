//! What weeder knows about a file once it has been read: the shapes the reader
//! seam returns and the rules take as arguments.
//!
//! Every type here is plain data with no parser behind it. A rule that counts
//! test cases, names a blast radius or follows an import reads these values and
//! nothing else, so the substrate that produced them can be upgraded or
//! replaced without a rule noticing.

use std::path::PathBuf;

/// What a definition is, as far as a rule needs to care. Languages spell these
/// differently and the reader maps each language's word onto one of them;
/// anything weeder has no name for arrives as `Other` rather than being dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionKind {
    Import,
    Function,
    Class,
    Struct,
    Interface,
    TypeAlias,
    Enum,
    Constant,
    Variable,
    Export,
    Property,
    Module,
    Other,
}

/// One named thing a file declares, with the lines it occupies and whatever it
/// declares inside itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
    pub kind: DefinitionKind,
    pub name: String,
    /// 1-based, and inclusive at both ends.
    pub start_line: u32,
    pub end_line: u32,
    /// The declaration as it is written, where the language has one line for it.
    pub signature: Option<String>,
    pub children: Vec<Definition>,
}

impl Definition {
    /// Whether `line` falls inside this definition. This is how a changed line
    /// finds the thing it changed.
    #[must_use]
    pub fn spans(&self, line: u32) -> bool {
        self.start_line <= line && line <= self.end_line
    }
}

/// Everything a file declares, in source order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Outline {
    pub definitions: Vec<Definition>,
}

impl Outline {
    /// The first definition of that name at any depth. Names repeat across
    /// scopes in every language weeder reads, so a caller that needs all of them
    /// walks [`Outline::flatten`] instead.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&Definition> {
        fn search<'a>(definitions: &'a [Definition], name: &str) -> Option<&'a Definition> {
            for definition in definitions {
                if definition.name == name {
                    return Some(definition);
                }
                if let Some(found) = search(&definition.children, name) {
                    return Some(found);
                }
            }
            None
        }
        search(&self.definitions, name)
    }

    /// Every definition the file declares, parents before their children, in
    /// source order.
    #[must_use]
    pub fn flatten(&self) -> Vec<&Definition> {
        fn walk<'a>(definitions: &'a [Definition], into: &mut Vec<&'a Definition>) {
            for definition in definitions {
                into.push(definition);
                walk(&definition.children, into);
            }
        }
        let mut found = Vec::new();
        walk(&self.definitions, &mut found);
        found
    }
}

/// A grouping of tests, or a test itself. Deleting a suite deletes every case
/// under it, which is why a rule counting tests has to tell them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestUnitKind {
    Suite,
    Case,
}

/// One suite or case, with the lines it occupies and how deeply it nests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestUnit {
    pub kind: TestUnitKind,
    /// The title the test carries: the string a suite or case was given, or the
    /// name of the function that declares it.
    pub name: String,
    /// 1-based, and inclusive at both ends.
    pub start_line: u32,
    pub end_line: u32,
    /// 0 at the top of the file, one more inside each enclosing suite.
    pub depth: u8,
}

/// The tests a file declares, in source order, suites before what they hold.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TestShape {
    pub units: Vec<TestUnit>,
}

impl TestShape {
    /// The cases alone: the tests that can pass or fail.
    pub fn cases(&self) -> impl Iterator<Item = &TestUnit> {
        self.units
            .iter()
            .filter(|unit| unit.kind == TestUnitKind::Case)
    }
}

/// One import statement: where it is, what it says, and what it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    /// 1-based, and inclusive at both ends. A statement written across several
    /// lines carries the range it spans.
    pub start_line: u32,
    pub end_line: u32,
    /// The statement as it is written.
    pub text: String,
    /// The module the statement names: a package, a crate path, a relative path.
    pub source: String,
    /// Whether the source lies outside the project. A language whose module
    /// paths need a build system to resolve, Go, Java, Kotlin, reads every
    /// source as external, because from the file alone that is all that is known.
    pub external: bool,
}

/// One call site of a symbol: a file, a line, and the function the call sits in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallerSite {
    /// The symbol that was searched for and found called here.
    pub symbol: String,
    /// Relative to the scope the search was given, joined with `/` on every
    /// platform, so a finding that names it reads the same wherever the checkout
    /// sits.
    pub path: PathBuf,
    /// 1-based.
    pub line: u32,
    /// The function the call is written inside, or the language's word for a
    /// call that sits in no function at all.
    pub calling_function: String,
    /// The line of source the call is written on, trimmed.
    pub call_text: String,
}
