use std::collections::HashSet;

use time_graph::{instrument, traverse_registered_callsite};

#[instrument(name = "renamed")]
fn named() {}

#[instrument("even with spaces")]
fn here_we_go() {}

#[instrument]
fn do_nothing() {
    named();
    here_we_go();
}

#[test]
fn check_created_callsite() {
    let mut names = HashSet::new();
    traverse_registered_callsite(|cs| {
        names.insert(cs.name());
    });
    assert!(names.is_empty());

    do_nothing();

    traverse_registered_callsite(|cs| {
        names.insert(cs.name());
    });
    let expected = ["do_nothing", "renamed", "even with spaces"].iter().cloned().collect::<HashSet<_>>();
    assert_eq!(names, expected);
}
