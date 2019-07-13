#![feature(specialization)]

extern crate lain;
extern crate ctrlc;

use lain::prelude::*;
use lain::rand::Rng;
// the driver is optional -- you can figure out how to manage
// your fuzzer's threads
use lain::driver::*;

use std::io::prelude::*;
use std::net::TcpStream;
use std::sync::{Arc, RwLock};

const THREAD_COUNT: usize = 10;

#[derive(Default)]
struct FuzzerThreadContext {
    last_packet: Option<PacketData>,
    scratch_packet: PacketData,
    thread_packet_iterations: usize,
}

#[derive(Default)]
struct GlobalContext {
    // unused, but you could put an iteration
    // counter per operation here or whatever you'd like
}

#[derive(Debug, Default, Clone, PostFuzzerIteration, FixupChildren, NewFuzzed, Mutatable, VariableSizeObject, BinarySerialize)]
struct PacketData {
    typ: UnsafeEnum<PacketType, u32>,

    offset: u64,
    length: u64,

    #[fuzzer(min = 0, max = 10)]
    data: Vec<u8>,
}

impl Fixup for PacketData {
    fn fixup<R: Rng>(&mut self, mutator: &mut Mutator<R>) {
        self.length = self.data.len() as u64;

        self.fixup_children(mutator);
    }
}

#[derive(Debug, Copy, Clone, FuzzerObject, ToPrimitiveU32, BinarySerialize)]
#[repr(u32)]
enum PacketType {
    Read = 0x0,
    Write = 0x1,
    Reset = 0x2,
}

impl Default for PacketType {
    fn default() -> Self {
        PacketType::Read
    }
}

fn main() {
    let mut driver = FuzzerDriver::<GlobalContext>::new(THREAD_COUNT);

    driver.set_global_context(Default::default());

    let driver = Arc::new(driver);
    let ctrlc_driver = driver.clone();

    ctrlc::set_handler(move || {
        ctrlc_driver.signal_exit();
    }).expect("couldn't set CTRL-C handler");

    start_fuzzer(driver.clone(), fuzzer_routine);

    driver.join_threads();

    println!("Finished in {} iterations", driver.num_iterations());
}

fn fuzzer_routine<R: Rng>(mutator: &mut Mutator<R>, thread_context: &mut FuzzerThreadContext, _global_context: Option<Arc<RwLock<GlobalContext>>>) -> Result<(), ()> {
    // TODO: we have overhead here of re-estabilishing the connection every time
    let mut stream = TcpStream::connect("127.0.0.1:8080").expect("server isn't running. possible crash?");

    let packet = match thread_context.last_packet {
        Some(ref mut last_packet) => {
            if mutator.mode() == MutatorMode::Havoc {
                last_packet.mutate(mutator, None);
                last_packet
            } else {
                // We want to do fuzzing of every field separately
                thread_context.scratch_packet = last_packet.clone();
                thread_context.scratch_packet.mutate(mutator, None);
                &thread_context.scratch_packet
            }
        }
        _ => {
            mutator.begin_new_corpus();

            thread_context.last_packet = Some(PacketData::new_fuzzed(mutator, None));
            thread_context.last_packet.as_mut().unwrap()
        }
    };

    let mut serialized_data = Vec::with_capacity(packet.serialized_size());
    packet.binary_serialize::<_, LittleEndian>(&mut serialized_data);

    println!("Sending packet: {:?}", packet);

    stream.write(&serialized_data).expect("failed to write data");

    let mut response_data = Vec::new();
    stream.read(&mut response_data);

    thread_context.thread_packet_iterations += 1;

    Ok(())
}
