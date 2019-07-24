# lain

This crate provides functionality one may find useful while developing a fuzzer. A recent
nightly Rust build is required for the specialization feature.

Please consider this crate in "beta" and subject to breaking changes for minor version releases for pre-1.0.

[![crates.io](https://img.shields.io/crates/v/lain.svg)](https://crates.io/crates/lain)
[![docs.rs](https://docs.rs/lain/badge.svg)](https://docs.rs/lain)

### Documentation

Please refer to [the wiki](https://github.com/microsoft/lain/wiki) for a high-level overview.

For API documentation: https://docs.rs/lain

### Installation

Lain requires rust nightly builds for specialization support.

Add the following to your Cargo.toml:

```toml
[dependencies]
lain = "0.1"
```

### Example Usage

```rust
extern crate lain;

use lain::prelude::*;
use lain::rand;
use lain::hexdump;

#[derive(Debug, Mutatable, NewFuzzed, BinarySerialize)]
struct MyStruct {
    field_1: u8,

    #[bitfield(backing_type = "u8", bits = 3)]
    field_2: u8,

    #[bitfield(backing_type = "u8", bits = 5)]
    field_3: u8,

    #[fuzzer(min = 5, max = 10000)]
    field_4: u32,

    #[fuzzer(ignore)]
    ignored_field: u64,
}

fn main() {
    let mut mutator = Mutator::new(rand::thread_rng());

    let mut instance = MyStruct::new_fuzzed(&mut mutator, None);

    let mut serialized_data = Vec::with_capacity(instance.serialized_size());
    instance.binary_serialize::<_, BigEndian>(&mut serialized_data);

    println!("{:?}", instance);
    println!("hex representation:\n{}", hexdump(&serialized_data));

    // perform small mutations on the instance
    instance.mutate(&mut mutator, None);

    println!("{:?}", instance);
}

// Output:
//
// MyStruct { field_1: 95, field_2: 5, field_3: 14, field_4: 8383, ignored_field: 0 }
// hex representation:
// ------00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F
// 0000: 5F 75 00 00 20 BF 00 00 00 00 00 00 00 00         _u...¿........
// MyStruct { field_1: 160, field_2: 5, field_3: 14, field_4: 8383, ignored_field: 0 }
```

A complete example of a fuzzer and its target can be found in the [examples](examples/)
directory. The server is written in C and takes data over a TCP socket, parses a message, and
mutates some state. The fuzzer has Rust definitions of the C data structure and will send fully
mutated messages to the server and utilizes the `Driver` object to manage fuzzer threads and
state.

## Contributing

This project welcomes contributions and suggestions.  Most contributions require you to agree to
a Contributor License Agreement (CLA) declaring that you have the right to, and actually do,
grant us the rights to use your contribution. For details, visit https://cla.microsoft.com.

When you submit a pull request, a CLA-bot will automatically determine whether you need to
provide a CLA and decorate the PR appropriately (e.g., label, comment). Simply follow the
instructions provided by the bot. You will only need to do this once across all repos using our
CLA.

This project has adopted the [Microsoft Open Source Code of
Conduct](https://opensource.microsoft.com/codeofconduct/). For more information see the [Code of
Conduct FAQ](https://opensource.microsoft.com/codeofconduct/faq/) or contact
[opencode@microsoft.com](mailto:opencode@microsoft.com) with any additional questions or
comments.

License: MIT
