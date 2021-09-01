#[time_graph::instrument]
fn function_a(repeat: bool) {
    std::thread::sleep(std::time::Duration::from_millis(1));
    if repeat {
        function_b();
    }
}

#[time_graph::instrument]
fn function_b() {
    std::thread::sleep(std::time::Duration::from_millis(1));
    function_a(false);
}

#[time_graph::instrument]
fn recursive(mut count: usize) {
    std::thread::sleep(std::time::Duration::from_millis(1));
    count -= 1;
    if count > 0 {
        recursive(count);
    }
}

fn main() {
    time_graph::enable_data_collection(true);

    recursive(4);
    function_a(true);

    let graph = time_graph::get_full_graph();

    println!("{}", graph.as_dot());

    #[cfg(feature = "json")]
    println!("{}", graph.as_json());

    #[cfg(feature = "table")]
    println!("{}", graph.as_table());

    #[cfg(feature = "table")]
    println!("{}", graph.as_short_table());
}
