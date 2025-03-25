/* Licensed under the AGPL-3.0 License: https://www.gnu.org/licenses/agpl-3.0.html */

use criterion::{black_box, criterion_group, criterion_main, Criterion, Bencher};
use dynamic_message::{string_split_whitespace, string_split_whitespace_regex, LONG_STRING, SHORT_STRING};

fn benchmark_split_functions_short(c: &mut Criterion) {
    let mut group = c.benchmark_group("String split functions - short");

    group.bench_with_input("split_whitespace", &SHORT_STRING, |b, &input | {
        b.iter(|| string_split_whitespace(black_box(input)));
    });

    group.bench_with_input("split_whitespace_regex", &SHORT_STRING, |b, &input | {
        b.iter(|| string_split_whitespace_regex(black_box(input)));
    });

    group.finish();
}

fn benchmark_split_functions_long(c: &mut Criterion) {
    let mut group = c.benchmark_group("String split functions - long");

    group.bench_with_input("split_whitespace", &LONG_STRING, |b, &input | {
        b.iter(|| string_split_whitespace(black_box(input)));
    });

    group.bench_with_input("split_whitespace_regex", &LONG_STRING, |b, &input | {
        b.iter(|| string_split_whitespace_regex(black_box(input)));
    });

    group.finish();
}

criterion_group!(benches, benchmark_split_functions_short, benchmark_split_functions_long);
criterion_main!(benches);