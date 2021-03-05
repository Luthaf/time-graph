use time_graph::{callsite, CallSite};

#[test]
fn callsite_macro() {
    let cs_1: &'static CallSite = callsite!("cs_1");
    let cs_2 = callsite!("cs_2");
    let cs_3 = callsite!("test"); let cs_4 = callsite!("test");

    assert_eq!(cs_1.name(), "cs_1");
    assert_eq!(cs_2.name(), "cs_2");
    assert_eq!(cs_3.name(), "test");
    assert_eq!(cs_4.name(), "test");

    let all = [cs_1, cs_2, cs_3, cs_4];

    for cs in &all {
        assert_eq!(cs.file(), "time-graph/tests/callsite.rs");
        assert_eq!(cs.module_path(), "callsite");
    }

    assert_eq!(cs_3.line(), cs_4.line());
    assert_eq!(cs_1.line() + 1, cs_2.line());
}
