// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Benchmarks for appending iterators to a [`MutableBuffer`] with `extend`.
//!
//! Each case appends `LEN` items in calls of `chunk` items, into a buffer
//! that already has enough capacity.

use arrow_buffer::{ArrowNativeType, MutableBuffer};
use criterion::*;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use std::hint;
use std::ops::Range;

const LEN: usize = 1 << 15;

/// Benchmarks a case whose calls append the iterators `make(range)` for the
/// consecutive `chunk`-item ranges of `0..LEN`.
fn bench_extend<I: Iterator<Item: ArrowNativeType>>(
    group: &mut BenchmarkGroup<'_, measurement::WallTime>,
    name: &str,
    chunk: usize,
    make: impl Fn(Range<usize>) -> I,
) {
    let mut buffer = MutableBuffer::new(LEN * size_of::<I::Item>());
    group.bench_function(BenchmarkId::new(name, chunk), |b| {
        b.iter(|| {
            buffer.clear();
            for start in (0..LEN).step_by(chunk) {
                buffer.extend(make(start..start + chunk));
            }
            hint::black_box(buffer.as_slice());
        })
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(42);
    let bytes: Vec<u8> = (0..LEN).map(|_| rng.random_range(b'a'..=b'z')).collect();
    let keys: Vec<i32> = (0..LEN).map(|_| rng.random_range(0..1000)).collect();
    let wide: Vec<u128> = (0..LEN).map(|_| rng.random_range(0..1 << 100)).collect();

    // Read at run time so the compiler can't specialize the maps for them.
    let (from, to) = hint::black_box((b'a', b'z'));
    let key_offset: i32 = hint::black_box(1000);
    let wide_offset: u128 = hint::black_box(1000);

    let mut group = c.benchmark_group("MutableBuffer extend");
    for chunk in [16, 1024] {
        bench_extend(&mut group, "copied u8", chunk, |r| bytes[r].iter().copied());
        // Like replacing one byte with another in a string.
        bench_extend(&mut group, "map u8", chunk, |r| {
            bytes[r]
                .iter()
                .map(move |&b| if b == from { to } else { b })
        });
        // Like copying dictionary keys in `MutableArrayData`.
        bench_extend(&mut group, "map i32", chunk, |r| {
            keys[r].iter().map(move |k| k + key_offset)
        });
        bench_extend(&mut group, "map u128", chunk, |r| {
            wide[r].iter().map(move |v| v + wide_offset)
        });
        // Like dense union offsets in `MutableArrayData`.
        bench_extend(&mut group, "range i32", chunk, |r| {
            r.start as i32..r.end as i32
        });
        // Like `BufferBuilder::append_n`.
        bench_extend(&mut group, "repeat_n u32", chunk, |r| {
            std::iter::repeat_n(r.start as u32, chunk)
        });
        bench_extend(&mut group, "chain i32", chunk, |r| {
            let (lo, hi) = keys[r].split_at(chunk / 2);
            lo.iter().chain(hi).copied()
        });
        // The size hint is not exact. The predicate always passes, so the
        // branch is predictable.
        bench_extend(&mut group, "filter i32", chunk, |r| {
            keys[r].iter().copied().filter(|&k| k >= 0)
        });
    }
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
