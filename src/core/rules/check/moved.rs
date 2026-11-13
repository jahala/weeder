//! Tests that followed their code into another file.
//!
//! A function that moves from one file to another takes its tests with it, and
//! on the way they are often renamed after their new home and rewritten around
//! it. T1 follows a case by its name or by what it holds, and T2 follows a
//! claim by its case or by its line, so a move that changes all of those reads
//! to both as a loss. What is left to follow it by is what the tests were
//! about: the production function they called.
//!
//! So a case that left is read as moved when three things hold. The function
//! it calls is defined in production code on both sides of the change, in a
//! file the change touched, so it moved or stayed under the eyes of the diff.
//! The change adds cases to another test file that call the same function.
//! And those cases make at least as many assertions as the ones that left,
//! counted the way T2 counts them. Each added assertion pays for one that left
//! and for nothing else, so an added case stands in for no more than it
//! asserts, and a case that left asserting more than what arrived is still a
//! case gone.
//!
//! Where weeder cannot tell a move from a loss, the rule keeps the finding it
//! would have made. A function weeder cannot see on both sides, because the
//! file that defines it is not in the change, follows nothing. A case that
//! asserted nothing has no count to hold an arrival to, so T1 follows it by
//! its name or its body alone.

use std::collections::BTreeSet;

use crate::core::change::{Change, Side};
use crate::core::classify::FileKind;
use crate::core::read::{DefinitionKind, TestUnit};
use crate::core::syntax::words;

/// The production functions the change carries on both sides of it: defined
/// before it in a file it touched, and after it in a file it touched, the same
/// one or another. A function a test module declares is a test, not the code
/// under it.
#[must_use]
pub fn kept(changes: &[Change]) -> BTreeSet<String> {
    let before: BTreeSet<&str> = changes
        .iter()
        .flat_map(|change| functions(&change.before))
        .collect();
    changes
        .iter()
        .flat_map(|change| functions(&change.after))
        .filter(|name| before.contains(name))
        .map(str::to_string)
        .collect()
}

/// The functions a production file defines outside its tests.
fn functions(side: &Side) -> Vec<&str> {
    if !side.is(FileKind::Prod) {
        return Vec::new();
    }
    side.outline
        .flatten()
        .into_iter()
        .filter(|definition| definition.kind == DefinitionKind::Function)
        .filter(|definition| !definition.name.is_empty())
        .filter(|definition| {
            !side.tests.units.iter().any(|unit| {
                unit.start_line <= definition.start_line && definition.end_line <= unit.end_line
            })
        })
        .map(|definition| definition.name.as_str())
        .collect()
}

/// The functions among `kept` that a case calls, anywhere inside it.
#[must_use]
pub fn exercised(side: &Side, case: &TestUnit, kept: &BTreeSet<String>) -> BTreeSet<String> {
    if kept.is_empty() {
        return BTreeSet::new();
    }
    let mask = side.mask();
    (case.start_line..=case.end_line)
        .flat_map(|line| {
            words(&mask.code(line))
                .into_iter()
                .filter(|word| word.is_called() && kept.contains(word.text))
                .map(|word| word.text.to_string())
                .collect::<Vec<String>>()
        })
        .collect()
}

/// The assertions a case makes on its own lines, out of a side's assertions
/// as T2 weighs them, each with the line it is made on: a line inside a
/// narrower case written within this one is that case's.
#[must_use]
pub fn weight(side: &Side, case: &TestUnit, weights: &[(u32, usize)]) -> usize {
    weights
        .iter()
        .filter(|(line, _)| side.tests.case_at(*line) == Some(case))
        .fold(0, |sum, (_, weight)| sum.saturating_add(*weight))
}

/// Assertions a change added to one test file, in cases that call the
/// functions named.
#[derive(Debug, Clone)]
pub struct Offer {
    pub path: String,
    pub calls: BTreeSet<String>,
    pub weight: usize,
}

/// What a payment took, and from which files.
#[derive(Debug, Default)]
pub struct Paid {
    pub amount: usize,
    pub from: BTreeSet<String>,
}

/// The added assertions a change offers, each spent once, so a case that
/// arrived pays for no more than it asserts however many cases left.
#[derive(Debug, Default)]
pub struct Ledger {
    offers: Vec<Offer>,
}

impl Ledger {
    #[must_use]
    pub fn new(offers: Vec<Offer>) -> Ledger {
        Ledger { offers }
    }

    /// Pays as much of `owed` as the offers outside `from` that call one of
    /// `calls` hold, and takes it from them.
    pub fn pay(&mut self, from: &str, calls: &BTreeSet<String>, owed: usize) -> Paid {
        let mut paid = Paid::default();
        for offer in self.payers(from, calls) {
            if paid.amount == owed {
                break;
            }
            let amount = offer.weight.min(owed - paid.amount);
            if amount == 0 {
                continue;
            }
            offer.weight -= amount;
            paid.amount += amount;
            paid.from.insert(offer.path.clone());
        }
        paid
    }

    /// Pays the whole of `owed` or none of it: a case is moved or it is not.
    /// A case that asserted nothing has no count to hold what arrived to, and
    /// any number of them would ride on one case that calls the function, so
    /// it is paid for by nothing and stays gone.
    pub fn pay_whole(&mut self, from: &str, calls: &BTreeSet<String>, owed: usize) -> Option<Paid> {
        if owed == 0 {
            return None;
        }
        let held = self
            .payers(from, calls)
            .fold(0_usize, |sum, offer| sum.saturating_add(offer.weight));
        (held >= owed).then(|| self.pay(from, calls, owed))
    }

    fn payers<'a>(
        &'a mut self,
        from: &'a str,
        calls: &'a BTreeSet<String>,
    ) -> impl Iterator<Item = &'a mut Offer> {
        self.offers.iter_mut().filter(move |offer| {
            offer.path != from && !calls.is_empty() && !offer.calls.is_disjoint(calls)
        })
    }
}
