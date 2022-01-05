#![feature(specialization)]

extern crate lain;

#[cfg(test)]
mod test {
    use lain::byteorder::{BigEndian, LittleEndian};
    use lain::hexdump;
    use lain::prelude::*;
    use lain::rand::rngs::SmallRng;
    use lain::rand::SeedableRng;
    use std::io::BufWriter;

    #[derive(Debug, NewFuzzed, Clone, BinarySerialize)]
    pub struct NestedStruct {
        test1: u32,
        nested: TestStruct,
        test2: u32,
    }

    #[derive(Debug, NewFuzzed, Clone, BinarySerialize)]
    pub struct TestStruct {
        single_byte: u8,

        #[lain(bits = 1)]
        bitfield_1: u8,
        #[lain(bits = 2)]
        bitfield_2: u8,
        #[lain(bits = 1)]
        bitfield_3: u8,
        #[lain(bits = 1)]
        bitfield_4: u8,
        #[lain(bits = 3)]
        bitfield_5: u8,

        uint32: u32,

        short: u16,
        end_byte: u8,
    }

    #[test]
    fn test_little_endian_serialization() {
        //let expected_data = vec![0x00u8, 0x6Cu8, 0x00u8, 0x00u8, 0xBBu8, 0xCCu8, 0xDDu8, 0xEEu8, 0xFFu8, 0xAAu8, 0x01u8, 0x00u8, ];
        let expected_data = vec![
            0x00u8, 0x6Cu8, 0xCCu8, 0xDDu8, 0xEEu8, 0xFFu8, 0xFFu8, 0xAAu8, 0x01u8,
        ];

        let test = TestStruct {
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

        let mut buffer = Vec::with_capacity(expected_data.len());
        test.binary_serialize::<_, LittleEndian>(&mut buffer);

        compare_slices(&expected_data, &buffer);
    }

    #[test]
    fn test_big_endian_serialization() {
        let expected_data = vec![
            0x00u8, 0x6Cu8, 0xFFu8, 0xEEu8, 0xDDu8, 0xCCu8, 0xAAu8, 0xFFu8, 0x01u8,
        ];

        let test = TestStruct {
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

        let mut buffer = Vec::with_capacity(expected_data.len());
        test.binary_serialize::<_, BigEndian>(&mut buffer);

        compare_slices(&expected_data, &buffer);
    }

    #[test]
    fn test_nested_struct_big_endian() {
        let expected_data = vec![
            0xAAu8, 0xBBu8, 0xCCu8, 0xDDu8, 0x00u8, 0x6Cu8, 0xFFu8, 0xEEu8, 0xDDu8, 0xCCu8, 0xAAu8,
            0xFFu8, 0x01u8, 0x00u8, 0x11u8, 0x22u8, 0x33u8,
        ];

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
        };

        let mut buffer = Vec::with_capacity(expected_data.len());
        parent.binary_serialize::<_, BigEndian>(&mut buffer);

        compare_slices(&expected_data, &buffer);
    }

    #[test]
    fn test_nested_struct_little_endian() {
        let expected_data = vec![
            0xDDu8, 0xCCu8, 0xBBu8, 0xAAu8, 0x00u8, 0x6Cu8, 0xCCu8, 0xDDu8, 0xEEu8, 0xFFu8, 0xFFu8,
            0xAAu8, 0x01u8, 0x33, 0x22, 0x11, 0x0,
        ];

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
        };

        let mut buffer = Vec::with_capacity(expected_data.len());
        parent.binary_serialize::<_, LittleEndian>(&mut buffer);

        compare_slices(&expected_data, &buffer);
    }

    #[test]
    fn test_boolean_field_can_be_randomized() {
        #[derive(Default, NewFuzzed, BinarySerialize, Clone)]
        struct StructWithBoolean {
            bool_field: bool,
        }

        let mut mutator = get_mutator();

        let initialized_struct: StructWithBoolean =
            StructWithBoolean::new_fuzzed(&mut mutator, None);

        // no assert or anything here since the concern is whether or not
        // the rng had bounds that cause a panic
        assert!(initialized_struct.bool_field == true || initialized_struct.bool_field == false);
    }

    #[test]
    fn test_ignored_fields() {
        #[derive(NewFuzzed, BinarySerialize, Clone)]
        struct IgnoredFieldsStruct {
            #[lain(ignore)]
            ignored: u8,
        }

        let mut mutator = get_mutator();

        let initialized_struct = IgnoredFieldsStruct::new_fuzzed(&mut mutator, None);

        assert_eq!(initialized_struct.ignored, 0);
    }

    #[test]
    fn test_initializer() {
        #[derive(Default, NewFuzzed, BinarySerialize, Clone)]
        struct InitializedFieldsStruct {
            #[lain(initializer = "0x41")]
            initialized: u8,
        }

        let mut mutator = get_mutator();

        let initialized_struct = InitializedFieldsStruct::new_fuzzed(&mut mutator, None);

        assert_eq!(initialized_struct.initialized, 0x41);
    }

    #[test]
    fn test_dynamic_array_limits() {
        #[derive(Default, NewFuzzed, Clone, BinarySerialize)]
        struct Foo {
            #[lain(min = 1, max = 10)]
            bar: Vec<u32>,
        }

        #[derive(Default, NewFuzzed, BinarySerialize, Clone)]
        struct InnerType {
            x: u32,
        }

        let mut mutator = get_mutator();

        let initialized_struct: Foo = Foo::new_fuzzed(&mut mutator, None);

        assert!(initialized_struct.bar.len() >= 1);
        assert!(initialized_struct.bar.len() <= 10);
    }

    #[test]
    fn generic_mutation_test() {
        #[derive(Default, Debug, NewFuzzed, BinarySerialize, Clone)]
        struct Foo {
            #[lain(min = 1, max = 10)]
            bar: Vec<u32>,
            baz: u64,
            x: u32,
            y: f32,
            z: f64,
            a: bool,
            b: u8,
        }

        let mut instance;
        let mut mutator = get_mutator();

        for _i in 0..10000 {
            instance = Foo::new_fuzzed(&mut mutator, None);
        }
    }

    #[test]
    fn test_filling_array() {
        let mut mutator = get_mutator();

        #[derive(Default, NewFuzzed, BinarySerialize, Clone)]
        struct StructWithArray {
            pub array: [u8; 16],
        }

        let initialized_struct = StructWithArray::new_fuzzed(&mut mutator, None);

        let mut result = 0u8;
        for &b in initialized_struct.array.iter() {
            result ^= b;
        }

        assert!(result != 0);
    }

    // #[test]
    // fn test_filling_vec() {
    //     let mut mutator = get_mutator();
    //     mutator.begin_new_corpus();
    //     let mut v: Vec<u8> = vec![1,2];
    //     v.mutate(&mut mutator);

    //     println!("VECIS: {:X?}", v);
    // }

    #[test]
    fn test_array_of_structs() {
        let mut mutator = get_mutator();

        #[derive(Default, NewFuzzed, BinarySerialize, Clone)]
        struct OtherStruct {
            field: u8,
        }

        #[derive(Default, NewFuzzed, BinarySerialize, Clone)]
        struct StructWithArray {
            pub array: [OtherStruct; 32],
        }

        let initialized_struct = StructWithArray::new_fuzzed(&mut mutator, None);

        let mut result = 0u8;
        for ref b in initialized_struct.array.iter() {
            result ^= b.field;
        }

        assert!(result != 0);
    }

    #[test]
    fn test_overriding_byteorder_parent_littleendian() {
        let expected: [u8; 8] = [0xDD, 0xCC, 0xBB, 0xAA, 0x00, 0x11, 0x22, 0x33];

        #[derive(BinarySerialize)]
        struct BigEndianStruct {
            big_endian_field: u32,
        }

        #[derive(BinarySerialize)]
        struct MyStruct {
            field1: u32,
            #[lain(big_endian)]
            field2: BigEndianStruct,
        }

        let s = MyStruct {
            field1: 0xAABBCCDD,
            field2: BigEndianStruct {
                big_endian_field: 0x00112233,
            },
        };

        let mut serialized_buffer = Vec::new();
        s.binary_serialize::<_, LittleEndian>(&mut serialized_buffer);

        compare_slices(&expected, &serialized_buffer);
    }

    #[test]
    fn test_overriding_byteorder_parent_bigendian() {
        let expected: [u8; 8] = [0xAA, 0xBB, 0xCC, 0xDD, 0x33, 0x22, 0x11, 0x00];

        #[derive(BinarySerialize)]
        struct LittleEndianStruct {
            little_endian_field: u32,
        }

        #[derive(BinarySerialize)]
        struct MyStruct {
            field1: u32,
            #[lain(little_endian)]
            field2: LittleEndianStruct,
        }

        let s = MyStruct {
            field1: 0xAABBCCDD,
            field2: LittleEndianStruct {
                little_endian_field: 0x00112233,
            },
        };

        let mut serialized_buffer = Vec::new();
        s.binary_serialize::<_, BigEndian>(&mut serialized_buffer);

        compare_slices(&expected, &serialized_buffer);
    }

    #[test]
    fn serializing_union_type() {
        let expected: [u8; 8] = [0xFF, 0x00, 0x00, 0x00, 0xAA, 0xBB, 0xCC, 0xDD];

        #[derive(BinarySerialize)]
        #[lain(serialized_size = 0x4)]
        enum MyEnum {
            MyOtherStruct(MyOtherStruct),
        }

        #[derive(BinarySerialize)]
        struct MyOtherStruct {
            foo: u8,
        }

        #[derive(BinarySerialize)]
        struct MyStruct {
            e: MyEnum,
            x: u32,
        }

        let s = MyStruct {
            e: MyEnum::MyOtherStruct(MyOtherStruct { foo: 0xFF }),
            x: 0xAABBCCDD,
        };

        let mut serialized_buffer = Vec::new();
        s.binary_serialize::<_, BigEndian>(&mut serialized_buffer);

        compare_slices(&expected, &serialized_buffer);
    }

    #[test]
    fn serializing_dynamic_array() {
        let expected: [u8; 4] = [0xAA, 0xBB, 0xCC, 0xDD];

        #[derive(Default, BinarySerialize)]
        struct MyStruct {
            dynamic_array: Vec<u16>,
        }

        let mut instance: MyStruct = Default::default();
        instance.dynamic_array.push(0xAABB);
        instance.dynamic_array.push(0xCCDD);

        let mut serialized_buffer = [0u8; 4];
        {
            let buffer_ref: &mut [u8] = &mut serialized_buffer;
            let mut writer = BufWriter::new(buffer_ref);
            instance.binary_serialize::<_, BigEndian>(&mut writer);
        }

        compare_slices(&expected, &serialized_buffer);
    }

    #[test]
    fn serializing_string() {
        let expected: [u8; 4] = [0x54, 0x45, 0x53, 0x54];

        const CHOICE_TEXT: &'static str = "TEST";

        #[derive(BinarySerialize)]
        struct MyStruct {
            choice: &'static str,
            choice2: String,
        }

        impl Default for MyStruct {
            fn default() -> Self {
                MyStruct {
                    choice: &CHOICE_TEXT,
                    choice2: String::from(CHOICE_TEXT),
                }
            }
        }

        let instance = MyStruct::default();

        let mut serialized_buffer = [0u8; 4];
        {
            let buffer_ref: &mut [u8] = &mut serialized_buffer;
            let mut writer = BufWriter::new(buffer_ref);
            instance.binary_serialize::<_, LittleEndian>(&mut writer);
        }

        compare_slices(&expected, &serialized_buffer);
    }

    #[test]
    fn mutating_string() {
        // TODO: Fix mutation methods

        // let mut my_string = String::from("Hello, world");
        // let mut mutator = get_mutator();

        // my_string.mutate(&mut mutator);

        // assert!(my_string != "Hello, world");
    }

    #[test]
    fn string_serialized_size() {
        // TODO: FIx

        // let my_string = String::from("Hello, world");

        // assert!(my_string.serialized_size() == my_string.as_bytes().len());
    }

    #[test]
    fn string_with_unicode_chars_serialized_size() {
        let my_string = String::from("🔥");

        assert!(my_string.serialized_size() == my_string.as_bytes().len());
    }

    #[test]
    fn driver_can_reproduce_mutations() {
        use lain::rand::Rng;
        use std::sync::{Arc, RwLock};

        #[derive(Debug, Default, NewFuzzed, Mutatable, Clone, PartialEq, BinarySerialize)]
        struct S {
            value: u32,
        }

        #[derive(Default)]
        struct LocalContext {}

        #[derive(Default, Debug)]
        struct GlobalContext {
            mutated_data: Vec<S>,
            iterations: usize,
        }

        fn fuzzer_routine<R: lain::rand::Rng>(
            mutator: &mut Mutator<R>,
            _ctx: &mut LocalContext,
            global_ctx: Option<Arc<RwLock<GlobalContext>>>,
        ) -> Result<(), ()> {
            let global_ctx = global_ctx.unwrap();
            let mut global_ctx = global_ctx.write().unwrap();

            let data = S::new_fuzzed(mutator, None);

            global_ctx.mutated_data.push(data);
            global_ctx.iterations += 1;

            Ok(())
        }

        // Do the first run

        let seed: u64 = lain::rand::thread_rng().gen();
        let mut driver = lain::driver::FuzzerDriver::<GlobalContext>::new(1);
        let global_context: Arc<RwLock<GlobalContext>> = Default::default();
        driver.set_global_context(global_context.clone());
        driver.set_seed(seed);

        let driver = Arc::new(driver);

        lain::driver::start_fuzzer(driver.clone(), fuzzer_routine);

        let one_milli = std::time::Duration::from_millis(1);
        loop {
            if driver.num_iterations() >= 20 {
                driver.signal_exit();
                break;
            }

            std::thread::sleep(one_milli);
        }

        driver.join_threads();

        // Recreate the driver for a reproduction run
        let mutated_data = &global_context.read().unwrap().mutated_data[10..15];
        let start_iteration: u64 = 10;
        let end_iteration: u64 = 15;

        let mut driver = lain::driver::FuzzerDriver::<GlobalContext>::new(1);
        let global_context: Arc<RwLock<GlobalContext>> = Default::default();

        driver.set_global_context(global_context.clone());
        driver.set_to_reproduce_mode(start_iteration, end_iteration);
        driver.set_seed(seed);

        let driver = Arc::new(driver);

        lain::driver::start_fuzzer(driver.clone(), fuzzer_routine);

        driver.join_threads();

        // Check for differences
        let reproduced_data = &global_context.read().unwrap().mutated_data;
        for i in 0..mutated_data.len() {
            let i = i as usize;
            assert_eq!(mutated_data[i], reproduced_data[i]);
        }

        //println!("{:?}", global_context.read().unwrap());
    }

    #[test]
    fn test_post_mutation_called() {
        #[derive(NewFuzzed, Clone, BinarySerialize)]
        struct S {
            #[lain(ignore)]
            pub post_mutation_called: bool,
        }

        impl Fixup for S {
            fn fixup<R: lain::rand::Rng>(&mut self, _mutator: &mut Mutator<R>) {
                println!("fixup called");
                self.post_mutation_called = true;
            }
        }

        let mut mutator = get_mutator();

        let instance = S::new_fuzzed(&mut mutator, None);

        assert!(instance.post_mutation_called);
    }

    #[test]
    fn test_string_mutation() {
        // this test mostly ensures that the string generation does not panic
        let mut mutator = get_mutator();

        let mut utf8_str = Utf8String::new_fuzzed(&mut mutator, None);
        println!("{:?}", utf8_str);

        utf8_str.mutate(&mut mutator, None);
        println!("{:?}", utf8_str);

        let mut ascii_str = AsciiString::new_fuzzed(&mut mutator, None);
        println!("{:?}", ascii_str);

        ascii_str.mutate(&mut mutator, None);
        println!("{:?}", ascii_str);
    }

    #[test]
    fn test_max_size_constraint_seems_to_work() {
        #[derive(NewFuzzed, BinarySerialize)]
        struct Foo {
            a: u8,
            b: u8,
            c: Bar,
        }

        #[derive(NewFuzzed, BinarySerialize)]
        struct Bar {
            #[lain(min = 0, max = 100, weight_to = "min")]
            c: Vec<u8>,
        }

        #[derive(NewFuzzed, BinarySerialize)]
        enum TestEnum {
            Foo(Foo),
            Bar(Bar),
        }

        let mut mutator = get_mutator();

        for _i in 0..100 {
            let f = TestEnum::new_fuzzed(&mut mutator, Some(&Constraints::new().max_size(5)));
            assert!(f.serialized_size() <= 5);
        }
    }

    #[test]
    fn max_size_constraint_seems_to_work_with_mutation() {
        #[derive(NewFuzzed, Mutatable, BinarySerialize)]
        struct Foo {
            a: u8,
            b: u8,
            c: Bar,
        }

        #[derive(NewFuzzed, Mutatable, BinarySerialize)]
        struct Bar {
            #[lain(min = 0, max = 100, weight_to = "min")]
            c: Vec<u8>,
        }

        #[derive(NewFuzzed, Mutatable, BinarySerialize)]
        enum TestEnum {
            Foo(Foo),
            Bar(Bar),
        }

        let mut mutator = get_mutator();
        let mut constraints = Constraints::new();
        constraints.max_size(5);

        let mut instance = TestEnum::new_fuzzed(&mut mutator, Some(&constraints));

        for _i in 0..100 {
            instance.mutate(&mut mutator, Some(&constraints));
            assert!(instance.serialized_size() <= 5);
        }
    }

    #[test]
    fn max_size_constraint_seems_to_work_with_real_failure_case() {
        #[derive(NewFuzzed, Mutatable, BinarySerialize)]
        struct Foo {
            a: u8,
        }

        #[derive(NewFuzzed, Mutatable, BinarySerialize)]
        struct Bar {
            a: u32,

            #[lain(min = 0, max = 20)]
            b: Vec<u8>,
        }

        #[derive(NewFuzzed, Mutatable, BinarySerialize)]
        enum TestEnum {
            Foo(Foo),
            Bar(Bar),
        }

        const MAX_SIZE: usize = 20;

        let mut mutator = get_mutator();

        let mut constraints = Constraints::new();
        constraints.max_size(MAX_SIZE);

        let mut instance = TestEnum::new_fuzzed(&mut mutator, Some(&constraints));
        for _i in 0..1000 {
            instance = TestEnum::new_fuzzed(&mut mutator, Some(&constraints));
            assert!(instance.serialized_size() <= MAX_SIZE);
        }

        for _i in 0..1000 {
            let mut constraints = Constraints::new();
            constraints.max_size(MAX_SIZE);

            instance.mutate(&mut mutator, Some(&constraints));
            assert!(instance.serialized_size() <= MAX_SIZE);
        }
    }

    #[test]
    /// This test mostly ensures that compilation didn't break
    fn simple_enums_work() {
        #[derive(Copy, Clone, NewFuzzed, Mutatable, BinarySerialize, ToPrimitiveU8)]
        enum SimpleEnum {
            Foo = 1,
            Bar = 2,
        }

        let mut mutator = get_mutator();

        let mut instance = SimpleEnum::new_fuzzed(&mut mutator, None);
        for _i in 0..10 {
            instance = SimpleEnum::new_fuzzed(&mut mutator, None);
        }

        for _i in 0..10 {
            instance.mutate(&mut mutator, None);
        }
    }

    #[test]
    fn failed_docs_testcase() {
        #[derive(Debug, Mutatable, NewFuzzed, BinarySerialize)]
        struct MyStruct {
            field_1: u8,

            #[lain(bits = 3)]
            field_2: u8,

            #[lain(bits = 5)]
            field_3: u8,

            #[lain(min = 5, max = 10000)]
            field_4: u32,

            #[lain(ignore)]
            ignored_field: u64,
        }

        let mut mutator = get_mutator();

        println!("{:?}", MyStruct::new_fuzzed(&mut mutator, None));
    }

    #[test]
    #[should_panic]
    fn test_buffer_too_small_for_max_size() {
        #[derive(Debug, VariableSizeObject, BinarySerialize, NewFuzzed)]
        struct StructWithFixedSizeVec {
            field_1: u8,

            #[lain(min = 512, max = 512)]
            field_2: Vec<u8>,
        }

        let mut constraints = Constraints::new();
        constraints.max_size(200);

        let mut mutator = get_mutator();
        assert!(
            StructWithFixedSizeVec::new_fuzzed(&mut mutator, Some(&constraints)).serialized_size()
                < 200
        );
    }

    #[test]
    fn test_pointers() {
        let ptr = 0xDEADBEEFusize as *const std::ffi::c_void;
        let expected: [u8; 8] = [0x0, 0x0, 0x0, 0x0, 0xDE, 0xAD, 0xBE, 0xEF];

        let mut buffer = vec![];

        ptr.binary_serialize::<_, BigEndian>(&mut buffer);

        compare_slices(&expected[..], buffer.as_slice());

        let mut mutator = get_mutator();

        let ptr = <*const std::ffi::c_void as NewFuzzed>::new_fuzzed(&mut mutator, None);
        assert_eq!(ptr, std::ptr::null());
    }

    #[test]
    fn test_incomplete_bitfield() {
        #[derive(BinarySerialize)]
        struct IncompleteBitfield {
            #[lain(bits = 1)]
            a: u32,
            #[lain(bits = 1)]
            b: u32,
        }

        let mut s = IncompleteBitfield { a: 0, b: 1 };

        assert_eq!(IncompleteBitfield::min_nonzero_elements_size(), 4);

        let mut output = vec![];
        s.binary_serialize::<_, BigEndian>(&mut output);

        let expected: [u8; 4] = [0x0, 0x0, 0x0, 0x2];
        compare_slices(&expected[..], output.as_slice());
    }

    #[test]
    fn test_padded_serialized_size() {
        #[derive(BinarySerialize)]
        #[lain(serialized_size = 2)]
        struct IncompleteStruct {
            a: u8,
        }

        let s = IncompleteStruct { a: 0 };

        let mut output = vec![];
        s.binary_serialize::<_, BigEndian>(&mut output);

        let expected: [u8; 2] = [0x0, 0x0];
        compare_slices(&expected[..], output.as_slice());

        #[derive(BinarySerialize)]
        #[lain(serialized_size = 2)]
        enum IncompleteEnum {
            Variant(u8),
        }

        let s = IncompleteEnum::Variant(0);

        let mut output = vec![];

        if output.serialized_size() > 0x1000 {
            panic!("yeah");
        }
        s.binary_serialize::<_, BigEndian>(&mut output);

        compare_slices(&expected[..], output.as_slice());
    }

    #[test]
    fn test_serializing_enum_as_bitfield() {
        #[derive(Copy, Clone, BinarySerialize, ToPrimitiveU8)]
        #[repr(u8)]
        enum Test {
            Foo = 1,
        }

        #[derive(BinarySerialize)]
        struct ContainingTest {
            #[lain(bits = 1, bitfield_type = "u32")]
            test: Test,
        }

        let mut output = vec![];
        let c = ContainingTest { test: Test::Foo };

        c.binary_serialize::<_, BigEndian>(&mut output);
        assert_eq!(output[3], 1);
    }

    #[test]
    fn test_invalid_zeroing_panic() {
        #[derive(Copy, Clone, NewFuzzed, BinarySerialize, ToPrimitiveU8)]
        #[repr(u8)]
        enum Foo {
            A = 1,
            B = 2,
        }

        #[derive(NewFuzzed, BinarySerialize)]
        struct Bar {
            foo: Foo,
        }

        let mut mutator = get_mutator();
        Bar::new_fuzzed(&mut mutator, None);
    }

    #[test]
    fn test_serialized_size_does_not_exceed_constraint() {
        #[derive(Clone, NewFuzzed, Mutatable, BinarySerialize, Debug)]
        struct Foo {
            #[lain(min = 0, max = 1000)]
            a: Vec<u16>,
            #[lain(min = 0, max = 1000)]
            b: Vec<u16>,
        }

        #[derive(Clone, NewFuzzed, Mutatable, BinarySerialize, Debug)]
        struct Bar {
            #[lain(min = 0, max = 1000)]
            foo: Vec<u16>,
            #[lain(min = 0, max = 1000)]
            b: Vec<u16>,
        }

        #[derive(Clone, NewFuzzed, Mutatable, BinarySerialize, Debug)]
        struct Baz {
            bar: Vec<Bar>,
        }

        const MAX_SIZE: usize = 21;

        let mut constraints = Constraints::new();
        constraints.max_size(MAX_SIZE);

        let mut mutator = get_mutator();

        let mut obj = Foo::new_fuzzed(&mut mutator, Some(&constraints));
        for i in 0..1000 {
            obj = Foo::new_fuzzed(&mut mutator, Some(&constraints));
            assert!(obj.serialized_size() <= MAX_SIZE);

            let prev_obj = obj.clone();

            obj.mutate(&mut mutator, Some(&constraints));
            assert!(
                obj.serialized_size() <= MAX_SIZE,
                "max_size: {}, object_size: {}, obj {:?}, prev: {:?}",
                MAX_SIZE,
                obj.serialized_size(),
                obj,
                prev_obj
            );
        }

        let mut baz = Baz::new_fuzzed(&mut mutator, Some(&constraints));
        for i in 0..1000 {
            baz = Baz::new_fuzzed(&mut mutator, Some(&constraints));
            assert!(baz.serialized_size() <= MAX_SIZE);

            let prev_baz = baz.clone();

            baz.mutate(&mut mutator, Some(&constraints));
            assert!(
                baz.serialized_size() <= MAX_SIZE,
                "max_size: {}, obj {:?}, prev: {:?}",
                MAX_SIZE,
                baz,
                prev_baz
            );
        }
    }

    #[test]
    fn mutatable_object_changes_when_mutate_called() {
        #[derive(
            Debug, PartialEq, Mutatable, Eq, Copy, Clone, NewFuzzed, BinarySerialize, ToPrimitiveU8,
        )]
        #[repr(u8)]
        enum Foo {
            A = 1,
            B = 2,
        }

        #[derive(Debug, PartialEq, Mutatable, Eq, Copy, Clone, NewFuzzed, BinarySerialize)]
        enum Baz {
            A(u8),
            B(u64),
        }

        #[derive(Clone, Debug, Mutatable, PartialEq, Eq, NewFuzzed, BinarySerialize)]
        struct Bar {
            foo: Foo,
            other: u64,
            baz: Baz,
        }

        let mut mutator = get_mutator();
        let mut obj = Bar::new_fuzzed(&mut mutator, None);

        // this breaks at 72 iterations which is fine. would break at a different iteration
        // with a different seed
        for i in 0..50 {
            let prev_obj = obj.clone();
            obj.mutate(&mut mutator, None);

            assert_ne!(obj, prev_obj);
        }
    }

    fn compare_slices(expected: &[u8], actual: &[u8]) {
        assert_eq!(actual.len(), expected.len());

        for i in 0..expected.len() {
            if actual[i] != expected[i] {
                println!("Expected:\n{}", hexdump(&*expected));
                println!("\n\nActual:\n{}\n", hexdump(&*actual));

                panic!(
                    "value at index {} differed (expected {:02X}, actual {:02X})",
                    i, expected[i], actual[i]
                );
            }
            assert_eq!(actual[i], expected[i]);
        }
    }

    fn get_mutator() -> Mutator<SmallRng> {
        let rng = SmallRng::from_seed([1u8; 32]);

        return Mutator::new(rng);
    }
}
