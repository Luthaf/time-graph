use criterion::{Criterion, criterion_group, criterion_main, black_box};

#[time_graph::instrument]
fn do_nothing(value: usize) -> usize {
    value
}

fn do_nothing_no_instrument(value: usize) -> usize {
    value
}


#[time_graph::instrument]
fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

#[time_graph::instrument]
fn fibonacci_single(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci_no_instrument(n - 1) + fibonacci_no_instrument(n - 2),
    }
}

fn fibonacci_no_instrument(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci_no_instrument(n - 1) + fibonacci_no_instrument(n - 2),
    }
}
fn empty_function(c: &mut Criterion) {
    c.bench_function("empty functions, not instrumented", |b| b.iter(|| do_nothing_no_instrument(black_box(44))));

    time_graph::enable_data_collection(false);
    c.bench_function("empty functions, no collection", |b| b.iter(|| do_nothing(black_box(44))));

    time_graph::enable_data_collection(true);
    c.bench_function("empty functions, collection", |b| b.iter(|| do_nothing(black_box(44))));
}

fn basic_calculation(c: &mut Criterion) {
    c.bench_function("fibonacci, not instrumented", |b| b.iter(|| fibonacci_no_instrument(black_box(20))));

    time_graph::enable_data_collection(false);
    c.bench_function("fibonacci, no collection", |b| b.iter(|| fibonacci(black_box(20))));
    c.bench_function("fibonacci outer, no collection", |b| b.iter(|| fibonacci_single(black_box(20))));

    time_graph::enable_data_collection(true);
    c.bench_function("fibonacci, collection", |b| b.iter(|| fibonacci(black_box(20))));
    c.bench_function("fibonacci outer, collection", |b| b.iter(|| fibonacci_single(black_box(20))));
}

criterion_group!(benches, empty_function, basic_calculation);
criterion_main!(benches);
