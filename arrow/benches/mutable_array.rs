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

#[macro_use]
extern crate criterion;
use criterion::Criterion;

use rand::RngExt;

use arrow::array::*;
use arrow::datatypes::Int32Type;
use arrow::util::bench_util::{
    create_dict_from_values, create_string_array, create_string_view_array,
};
use arrow::util::test_util::seedable_rng;

fn create_slices(size: usize) -> Vec<(usize, usize)> {
    let rng = &mut seedable_rng();

    (0..size)
        .map(|_| {
            let start = rng.random_range(0..size / 2);
            let end = rng.random_range(start + 1..size);
            (start, end)
        })
        .collect()
}

/// Slices of 1 to 8 rows, like the runs that `zip` copies for a `CASE`
/// expression whose condition changes often.
fn create_short_slices(size: usize) -> Vec<(usize, usize)> {
    let rng = &mut seedable_rng();

    (0..size)
        .map(|_| {
            let start = rng.random_range(0..size - 8);
            let len = rng.random_range(1..=8);
            (start, start + len)
        })
        .collect()
}

/// Total number of rows in `slices`
fn total_len(slices: &[(usize, usize)]) -> usize {
    slices.iter().map(|(start, end)| end - start).sum()
}

/// Copies `slices` of `v1`, which hold `len` rows in all, into a
/// `MutableArrayData` that starts with room for all of them.
fn bench<T: Array>(v1: &T, slices: &[(usize, usize)], len: usize) {
    let data = v1.to_data();
    let mut mutable = MutableArrayData::new(vec![&data], false, len);
    for (start, end) in slices {
        mutable.try_extend(0, *start, *end).unwrap();
    }
    mutable.freeze();
}

fn add_benchmark(c: &mut Criterion) {
    let v1 = create_string_array::<i32>(1024, 0.0);
    let v2 = create_slices(1024);
    let v2_len = total_len(&v2);
    c.bench_function("mutable str 1024", |b| b.iter(|| bench(&v1, &v2, v2_len)));

    let v1 = create_string_array::<i32>(1024, 0.5);
    c.bench_function("mutable str nulls 1024", |b| {
        b.iter(|| bench(&v1, &v2, v2_len))
    });

    let short_slices = create_short_slices(1024);
    let short_len = total_len(&short_slices);

    let v1 = create_string_view_array(1024, 0.0);
    c.bench_function("mutable str_view 1024", |b| {
        b.iter(|| bench(&v1, &v2, v2_len))
    });
    c.bench_function("mutable str_view short slices 1024", |b| {
        b.iter(|| bench(&v1, &short_slices, short_len))
    });

    let values = create_string_array::<i32>(16, 0.0);
    let v1 = create_dict_from_values::<Int32Type>(1024, 0.0, &values);
    c.bench_function("mutable dict 1024", |b| b.iter(|| bench(&v1, &v2, v2_len)));
    c.bench_function("mutable dict short slices 1024", |b| {
        b.iter(|| bench(&v1, &short_slices, short_len))
    });
}

criterion_group!(benches, add_benchmark);
criterion_main!(benches);
