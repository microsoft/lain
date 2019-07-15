#![feature(specialization)]

#[macro_use]
extern crate criterion;
#[macro_use]
extern crate lain;

use criterion::*;

use lain::byteorder::BigEndian;
use lain::prelude::*;

#[derive(Debug, Default, Clone, Mutatable, BinarySerialize)]
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

#[derive(Debug, Default, Clone, Mutatable, BinarySerialize)]
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

fn bench_serialize_1000(c: &mut Criterion) {
    let struct_size = std::mem::size_of::<NestedStruct>();
    let function_name = format!(
        "serialize BigEndian with struct of size 0x{:X}",
        struct_size
    );

    let nested = TestStruct {
        single_byte: 0,
        bitfield_1: 0,
        bitfield_2: 2,
        bitfield_3: 1,
        bitfield_4: 0,
        bitfield_5: 3,
        uint32: 0xFFEEDDCC,
        short: 0xAAFF,
        end_byte: 0x1,
    };

    let parent = NestedStruct {
        test1: 0xAABBCCDD,
        nested,
        test2: 0x00112233,
        ..Default::default()
    };

    c.bench(
        function_name.as_ref(),
        Benchmark::new("serialize", move |b| {
            let mut buffer = Vec::with_capacity(struct_size);
            b.iter(|| {
                parent.binary_serialize::<_, BigEndian>(&mut buffer);
                black_box(&buffer);
                buffer.clear();
            });
        })
        .throughput(Throughput::Bytes(struct_size as u32)),
    );
}

criterion_group!(benches, bench_serialize_1000);
criterion_main!(benches);
