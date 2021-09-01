#[time_graph::instrument]
fn run_computation(max: u64) {
    for i in 0..max {
        compute(i)
    }

    time_graph::spanned!("another span", {
        details::bottom_5us();
    });

    for _ in 0..(max * max) {
        details::bottom_5us();
    }
}

#[time_graph::instrument]
pub fn compute(count: u64) {
    for _ in 0..count {
        details::bottom_5us();
    }
}

mod details {
    #[time_graph::instrument]
    pub fn bottom_5us() {
        std::thread::sleep(std::time::Duration::from_micros(5));
    }
}

#[time_graph::instrument]
fn run_other_5ms() {
    std::thread::sleep(std::time::Duration::from_millis(5));
}

fn main() {
    time_graph::enable_data_collection(true);

    run_other_5ms();
    run_computation(10);

    let graph = time_graph::get_full_graph();

    println!("{}", graph.as_dot());

    #[cfg(feature = "json")]
    println!("{}", graph.as_json());

    #[cfg(feature = "table")]
    println!("{}", graph.as_table());

    #[cfg(feature = "table")]
    println!("{}", graph.as_short_table());
}
