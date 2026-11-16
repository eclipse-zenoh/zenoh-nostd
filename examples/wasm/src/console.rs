use static_cell::StaticCell;
use std::{cell::RefCell, collections::VecDeque};
use wasm_bindgen::prelude::*;
use zenoh_examples_common::cli::{self, Cli, ConsoleRead, ConsoleWrite};
use zenoh_examples_wasm::{ENDPOINT, ExampleConfig};

thread_local! { static INPUT: RefCell<VecDeque<u8>> = const { RefCell::new(VecDeque::new()) }; }
#[wasm_bindgen]
pub fn console_input(text: &str) -> bool {
    INPUT.with_borrow_mut(|input| {
        if input.len() + text.len() > 1024 {
            return false;
        }
        input.extend(text.bytes());
        true
    })
}
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = zenohOutput)]
    fn output(bytes: &[u8]);
}
struct Rx;
impl ConsoleRead for Rx {
    async fn read(&mut self, bytes: &mut [u8]) -> Result<usize, ()> {
        loop {
            let n = INPUT.with_borrow_mut(|input| {
                let mut n = 0;
                while n < bytes.len() {
                    let Some(byte) = input.pop_front() else {
                        break;
                    };
                    bytes[n] = byte;
                    n += 1;
                }
                n
            });
            if n != 0 {
                return Ok(n);
            }
            embassy_time::Timer::after_millis(10).await;
        }
    }
}
struct Tx;
impl ConsoleWrite for Tx {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), ()> {
        output(bytes);
        Ok(())
    }
}
#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {
    static CLI: StaticCell<Cli> = StaticCell::new();
    static CONFIG: StaticCell<ExampleConfig> = StaticCell::new();
    let config = CONFIG.init(ExampleConfig::new(zenoh_wasm::WasmLinkManager));
    let _ = cli::run(
        CLI.init(Cli::new()),
        Rx,
        Tx,
        async { &*config },
        ENDPOINT,
        false,
    )
    .await;
}
