#![feature(specialization)]

extern crate criterion;
extern crate lain;

use criterion::*;

use lain::prelude::*;
use lain::rand::SeedableRng;

#[derive(Debug, Default, Clone, FuzzerObject, BinarySerialize)]
pub struct NestedStruct {
    test1: u32,
    nested: TestStruct,
    test2: u32,
    test3: u32,
    test4: u32,
    test5: u32,
    test6: u32,
    test7: u32,
    test8: u8,
    test9: u16,
    test10: u64,
    test11: [u8; 32],
}

#[derive(Debug, Default, Clone, FuzzerObject, BinarySerialize)]
pub struct TestStruct {
    single_byte: u8,

    #[bitfield(backing_type = "u8", bits = 1)]
    bitfield_1: u8,
    #[bitfield(backing_type = "u8", bits = 2)]
    bitfield_2: u8,
    #[bitfield(backing_type = "u8", bits = 1)]
    bitfield_3: u8,
    #[bitfield(backing_type = "u8", bits = 1)]
    bitfield_4: u8,
    #[bitfield(backing_type = "u8", bits = 3)]
    bitfield_5: u8,

    uint32: u32,

    short: u16,
    end_byte: u8,
}

fn bench_new_fuzzed_1000(c: &mut Criterion) {
    let struct_size = std::mem::size_of::<NestedStruct>();
    let function_name = format!("bench_new_fuzzed struct of size 0x{:X}", struct_size);

    c.bench(
        function_name.as_ref(),
        Benchmark::new("fuzz", move |b| {
            let mut mutator = Mutator::new(lain::rand::rngs::SmallRng::from_seed([0u8; 16]));
            b.iter(|| {
                let s = NestedStruct::new_fuzzed(&mut mutator, None);
                black_box(s);
            });
        })
        .throughput(Throughput::Bytes(struct_size as u32)),
    );
}

fn bench_in_place_mutation(c: &mut Criterion) {
    let struct_size = std::mem::size_of::<NestedStruct>();
    let function_name = format!("bench_in_place_mutation struct of size 0x{:X}", struct_size);

    c.bench(
        function_name.as_ref(),
        Benchmark::new("fuzz", move |b| {
            let mut mutator = Mutator::new(lain::rand::rngs::SmallRng::from_seed([0u8; 16]));
            let mut s = NestedStruct::new_fuzzed(&mut mutator, None);
            b.iter(|| {
                let mut s = s.clone();
                let state = mutator.get_corpus_state();
                s.mutate(&mut mutator, None);
                black_box(&s);
                mutator.set_corpus_state(state);
            });
        })
        .throughput(Throughput::Bytes(struct_size as u32)),
    );
}

fn bench_default_1000(c: &mut Criterion) {
    let struct_size = std::mem::size_of::<NestedStruct>();
    let function_name = format!("bench_default_1000 struct of size 0x{:X}", struct_size);

    c.bench(
        function_name.as_ref(),
        Benchmark::new("fuzz", move |b| {
            b.iter(|| {
                let s = NestedStruct::default();
                black_box(s);
            });
        })
        .throughput(Throughput::Bytes(struct_size as u32)),
    );
}

criterion_group!(
    benches,
    bench_new_fuzzed_1000,
    bench_default_1000,
    bench_in_place_mutation
);
criterion_main!(benches);
