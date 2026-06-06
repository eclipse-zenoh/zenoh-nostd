#![no_std]
#![no_main]

extern crate alloc;
extern crate panic_halt;

use alloc_cortex_m::CortexMHeap;
use alloc::vec::Vec;
use cortex_m::asm;
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;

use zenoh_proto::ZEncode;
use zenoh_proto::ZDecode;
use zenoh_proto::ZLen;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

const HEAP_SIZE: usize = 32768;

#[cfg(all(target_arch = "arm", target_os = "none"))]
#[no_mangle]
fn __getrandom_custom(dest: &mut [u8]) -> Result<(), getrandom::Error> {
    let mut seed = 0xdeadbeef_u32;
    for byte in dest.iter_mut() {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        *byte = (seed >> 16) as u8;
    }
    Ok(())
}

fn test_proto_codec() -> bool {
    let syn = zenoh_proto::msgs::InitSyn::default();
    let len = syn.z_len();
    let mut buf = Vec::new();
    buf.resize(len, 0u8);
    let mut slice = &mut buf[..];
    if syn.z_encode(&mut slice).is_err() {
        hprintln!("InitSyn encode failed");
        return false;
    }
    let decoded = zenoh_proto::msgs::InitSyn::z_decode(&mut &buf[..]);
    match decoded {
        Ok(_) => hprintln!("InitSyn roundtrip OK"),
        Err(e) => {
            hprintln!("InitSyn decode failed: {:?}", e);
            return false;
        }
    }

    let enc = zenoh_proto::fields::Encoding::text_plain();
    if enc.to_string() != "text/plain" {
        hprintln!("Encoding to_string failed");
        return false;
    }
    hprintln!("Encoding OK");

    true
}

#[entry]
fn main() -> ! {
    unsafe {
        ALLOCATOR.init(cortex_m_rt::heap_start() as usize, HEAP_SIZE);
    }

    hprintln!("zenoh-stm32: testing zenoh-proto codec...");

    let result = test_proto_codec();

    if result {
        hprintln!("PASS: all codec tests passed");
    } else {
        hprintln!("FAIL: codec tests failed");
    }

    loop {
        asm::wfi();
    }
}