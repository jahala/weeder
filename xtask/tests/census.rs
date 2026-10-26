//! A population under twenty is re-graded whole, and a census may stand.
//!
//! The re-grade draws a seeded sample of twenty from each population the report
//! holds. When a population is smaller than that, there is no draw to make: the
//! sample is every case there is, and a re-grade of all of them is a census. A
//! census stands at the same bar as a draw, ninety percent of the cases
//! re-graded. A sample short of twenty that is also short of its population is
//! a draw too small to rest on, and it still never stands.
//!
//! The population is read from the measurement itself, the blocked commits the
//! run found, and never from the re-grade's own account of it.

mod common;

use common::cleared_population;

const PROVISIONAL: &str = "weeder ships as a gate, pending the independent re-grade:";
const CONFIRMED: &str = "weeder ships as a gate:";

#[test]
fn a_census_of_nineteen_with_eighteen_agreeing_stands() {
    let (repo, bench) = cleared_population(19);
    bench.write_audit(&[("blocked commits", 19, 18)]);

    bench.calibrate("probe", &repo, &[]).succeeded();

    assert!(
        bench.verdict().starts_with(CONFIRMED),
        "every blocked commit the report holds was re-graded, and 18 of 19 is over the bar: {}",
        bench.verdict()
    );
    assert!(
        bench.report().contains(
            "blocked commits at 94.7 percent of 19 cases, every blocked commit the report holds"
        ),
        "the report says the sample is a census:\n{}",
        bench.report()
    );
}

#[test]
fn a_census_of_nineteen_with_seventeen_agreeing_does_not_stand() {
    let (repo, bench) = cleared_population(19);
    bench.write_audit(&[("blocked commits", 19, 17)]);

    bench.calibrate("probe", &repo, &[]).succeeded();

    assert!(
        bench.verdict().starts_with(PROVISIONAL),
        "a census is held to the same bar, and 17 of 19 is under it: {}",
        bench.verdict()
    );
    assert!(
        bench.report().contains(
            "blocked commits agrees on 89.5 percent of 19 cases, every blocked commit the report holds"
        ),
        "the report names the census that fell short:\n{}",
        bench.report()
    );
}

#[test]
fn a_sample_of_nineteen_from_a_population_of_twenty_five_does_not_stand() {
    let (repo, bench) = cleared_population(25);
    bench.write_audit(&[("blocked commits", 19, 19)]);

    bench.calibrate("probe", &repo, &[]).succeeded();

    assert!(
        bench.verdict().starts_with(PROVISIONAL),
        "nineteen of twenty-five is a short draw, not a census: {}",
        bench.verdict()
    );
    assert!(
        bench.report().contains("the sample floor is 20"),
        "the report says the sample was too small:\n{}",
        bench.report()
    );
}

#[test]
fn a_population_of_twenty_five_re_graded_twenty_still_stands() {
    let (repo, bench) = cleared_population(25);
    bench.write_audit(&[("blocked commits", 20, 18)]);

    bench.calibrate("probe", &repo, &[]).succeeded();

    assert!(
        bench.verdict().starts_with(CONFIRMED),
        "a seeded draw of twenty is what a population of twenty or more is held to: {}",
        bench.verdict()
    );
    assert!(
        bench
            .report()
            .contains("blocked commits at 90.0 percent of 20 cases."),
        "a draw is not called a census:\n{}",
        bench.report()
    );
}
