use std::time::Duration;

#[time_graph::instrument]
fn sleep_recursive(depth: u32) {
    if depth == 0 {
        return;
    }
    std::thread::sleep(Duration::from_millis(10));
    sleep_recursive(depth - 1);
}

#[test]
fn test_recursive() {
    time_graph::enable_data_collection(true);
    sleep_recursive(5);

    for span in time_graph::get_full_graph().spans() {
        if span.callsite.name() == "sleep_recursive" {
            // make sure we record the correct elapsed time (at least 50 ms,
            // likely a bit more since sleep does not guarantee exact timing).
            assert!(span.elapsed >= std::time::Duration::from_millis(50));
            assert!(span.elapsed <= std::time::Duration::from_millis(80));
            assert_eq!(span.called, 6);
            break;
        }
    }
}
