//! HAL-independent Embassy USB CDC-ACM console. The board supplies the driver.
use embassy_usb::{
    Builder, Config, UsbDevice,
    class::cdc_acm::{CdcAcmClass, Receiver, Sender, State},
    driver::{Driver, EndpointError},
};
use zenoh_examples_common::cli::{ConsoleRead, ConsoleWrite};

const PACKET_SIZE: usize = 64;

/// Static descriptor/control storage; no allocator or HAL dependency.
pub struct Buffers {
    config: [u8; 256],
    bos: [u8; 256],
    msos: [u8; 64],
    control: [u8; 64],
    state: State<'static>,
}
impl Default for Buffers {
    fn default() -> Self {
        Self::new()
    }
}
impl Buffers {
    pub const fn new() -> Self {
        Self {
            config: [0; 256],
            bos: [0; 256],
            msos: [0; 64],
            control: [0; 64],
            state: State::new(),
        }
    }
}

/// Create the USB device and its CLI stream adapters. Poll `device.run()`
/// concurrently with the CLI for the entire console lifetime. Device identity
/// and power policy come from the board's configuration.
pub fn new<D: Driver<'static>>(
    driver: D,
    config: Config<'static>,
    buffers: &'static mut Buffers,
) -> (UsbDevice<'static, D>, UsbRead<D>, UsbWrite<D>) {
    let mut builder = Builder::new(
        driver,
        config,
        &mut buffers.config,
        &mut buffers.bos,
        &mut buffers.msos,
        &mut buffers.control,
    );
    let class = CdcAcmClass::new(&mut builder, &mut buffers.state, PACKET_SIZE as u16);
    let (tx, rx) = class.split();
    (
        builder.build(),
        UsbRead {
            receiver: rx,
            packet: [0; PACKET_SIZE],
            start: 0,
            end: 0,
        },
        UsbWrite(tx),
    )
}

pub struct UsbRead<D: Driver<'static>> {
    receiver: Receiver<'static, D>,
    packet: [u8; PACKET_SIZE],
    start: usize,
    end: usize,
}
impl<D: Driver<'static>> ConsoleRead for UsbRead<D> {
    async fn read(&mut self, bytes: &mut [u8]) -> Result<usize, ()> {
        if bytes.is_empty() {
            return Ok(0);
        }
        loop {
            if self.start < self.end {
                let n = bytes.len().min(self.end - self.start);
                bytes[..n].copy_from_slice(&self.packet[self.start..self.start + n]);
                self.start += n;
                return Ok(n);
            }
            self.receiver.wait_connection().await;
            // CDC reads need a full-sized packet buffer even when the CLI asks
            // for fewer bytes. Preserve the remaining bytes for the next read.
            match self.receiver.read_packet(&mut self.packet).await {
                Ok(n) => {
                    self.start = 0;
                    self.end = n;
                }
                Err(EndpointError::Disabled) => {
                    self.start = 0;
                    self.end = 0;
                }
                Err(EndpointError::BufferOverflow) => return Err(()),
            }
        }
    }
}

pub struct UsbWrite<D: Driver<'static>>(Sender<'static, D>);
impl<D: Driver<'static>> UsbWrite<D> {
    async fn packet(&mut self, data: &[u8]) -> Result<(), ()> {
        loop {
            self.0.wait_connection().await;
            match self.0.write_packet(data).await {
                Ok(()) => return Ok(()),
                Err(EndpointError::Disabled) => {}
                Err(EndpointError::BufferOverflow) => return Err(()),
            }
        }
    }
}
impl<D: Driver<'static>> ConsoleWrite for UsbWrite<D> {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), ()> {
        for chunk in bytes.chunks(PACKET_SIZE) {
            self.packet(chunk).await?;
        }
        // USB bulk transfers require a short packet to complete at the host.
        if !bytes.is_empty() && bytes.len().is_multiple_of(PACKET_SIZE) {
            self.packet(&[]).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use core::{
        future::Future,
        task::{Context, Poll, Waker},
    };
    use embassy_usb::driver::*;
    use std::{boxed::Box, cell::RefCell, collections::VecDeque, rc::Rc, vec, vec::Vec};

    #[derive(Default)]
    struct Traffic {
        reads: VecDeque<Result<Vec<u8>, EndpointError>>,
        writes: Vec<Vec<u8>>,
        fail_write: bool,
        waits: usize,
        read_calls: usize,
    }
    struct MockEndpoint {
        traffic: Rc<RefCell<Traffic>>,
        info: EndpointInfo,
    }
    impl Endpoint for MockEndpoint {
        fn info(&self) -> &EndpointInfo {
            &self.info
        }
        async fn wait_enabled(&mut self) {
            self.traffic.borrow_mut().waits += 1;
        }
    }
    impl EndpointOut for MockEndpoint {
        async fn read(&mut self, data: &mut [u8]) -> Result<usize, EndpointError> {
            assert_eq!(data.len(), PACKET_SIZE);
            let mut traffic = self.traffic.borrow_mut();
            traffic.read_calls += 1;
            let packet = traffic.reads.pop_front().expect("unexpected USB read")?;
            data[..packet.len()].copy_from_slice(&packet);
            Ok(packet.len())
        }
    }
    impl EndpointIn for MockEndpoint {
        async fn write(&mut self, data: &[u8]) -> Result<(), EndpointError> {
            assert!(data.len() <= PACKET_SIZE);
            let mut traffic = self.traffic.borrow_mut();
            if core::mem::take(&mut traffic.fail_write) {
                return Err(EndpointError::Disabled);
            }
            traffic.writes.push(data.to_vec());
            Ok(())
        }
    }
    struct MockDriver {
        traffic: Rc<RefCell<Traffic>>,
        index: usize,
    }
    impl MockDriver {
        fn endpoint(
            &mut self,
            ep_type: EndpointType,
            dir: Direction,
            max_packet_size: u16,
            interval_ms: u8,
        ) -> MockEndpoint {
            self.index += 1;
            MockEndpoint {
                traffic: self.traffic.clone(),
                info: EndpointInfo {
                    addr: EndpointAddress::from_parts(self.index, dir),
                    ep_type,
                    max_packet_size,
                    interval_ms,
                },
            }
        }
    }
    struct Unused;
    impl Bus for Unused {
        async fn enable(&mut self) {}
        async fn disable(&mut self) {}
        async fn poll(&mut self) -> Event {
            core::future::pending().await
        }
        fn endpoint_set_enabled(&mut self, _: EndpointAddress, _: bool) {}
        fn endpoint_set_stalled(&mut self, _: EndpointAddress, _: bool) {}
        fn endpoint_is_stalled(&mut self, _: EndpointAddress) -> bool {
            false
        }
        async fn remote_wakeup(&mut self) -> Result<(), Unsupported> {
            Err(Unsupported)
        }
    }
    impl ControlPipe for Unused {
        fn max_packet_size(&self) -> usize {
            PACKET_SIZE
        }
        async fn setup(&mut self) -> [u8; 8] {
            core::future::pending().await
        }
        async fn data_out(
            &mut self,
            _: &mut [u8],
            _: bool,
            _: bool,
        ) -> Result<usize, EndpointError> {
            unreachable!()
        }
        async fn data_in(&mut self, _: &[u8], _: bool, _: bool) -> Result<(), EndpointError> {
            unreachable!()
        }
        async fn accept(&mut self) {
            unreachable!()
        }
        async fn reject(&mut self) {
            unreachable!()
        }
        async fn accept_set_address(&mut self, _: u8) {
            unreachable!()
        }
    }
    impl Driver<'static> for MockDriver {
        type EndpointIn = MockEndpoint;
        type EndpointOut = MockEndpoint;
        type ControlPipe = Unused;
        type Bus = Unused;
        fn alloc_endpoint_in(
            &mut self,
            ty: EndpointType,
            _: Option<EndpointAddress>,
            size: u16,
            interval: u8,
        ) -> Result<MockEndpoint, EndpointAllocError> {
            Ok(self.endpoint(ty, Direction::In, size, interval))
        }
        fn alloc_endpoint_out(
            &mut self,
            ty: EndpointType,
            _: Option<EndpointAddress>,
            size: u16,
            interval: u8,
        ) -> Result<MockEndpoint, EndpointAllocError> {
            Ok(self.endpoint(ty, Direction::Out, size, interval))
        }
        fn start(self, _: u16) -> (Unused, Unused) {
            (Unused, Unused)
        }
    }
    fn io() -> (
        Rc<RefCell<Traffic>>,
        UsbRead<MockDriver>,
        UsbWrite<MockDriver>,
    ) {
        let traffic = Rc::new(RefCell::new(Traffic::default()));
        let driver = MockDriver {
            traffic: traffic.clone(),
            index: 0,
        };
        let (_device, rx, tx) = new(
            driver,
            Config::new(0xc0de, 0xcafe),
            Box::leak(Box::new(Buffers::new())),
        );
        (traffic, rx, tx)
    }
    fn ready<T>(future: impl Future<Output = T>) -> T {
        match core::pin::pin!(future)
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("unexpected wait in USB endpoint fixture"),
        }
    }
    #[test]
    fn small_cli_reads_preserve_a_full_usb_packet() {
        let (traffic, mut rx, _) = io();
        let packet: Vec<u8> = (0..64).collect();
        traffic.borrow_mut().reads.push_back(Ok(packet.clone()));
        let mut first = [0; 5];
        assert_eq!(ready(rx.read(&mut first)), Ok(5));
        assert_eq!(first, packet[..5]);
        let mut rest = [0; 64];
        assert_eq!(ready(rx.read(&mut rest)), Ok(59));
        assert_eq!(&rest[..59], &packet[5..]);
        assert_eq!(traffic.borrow().read_calls, 1);
        assert_eq!(ready(rx.read(&mut [])), Ok(0));
    }
    #[test]
    fn disconnected_rx_recovers_and_ignores_zero_length_packets() {
        let (traffic, mut rx, _) = io();
        traffic.borrow_mut().reads.extend([
            Err(EndpointError::Disabled),
            Ok(vec![]),
            Ok(b"help\n".to_vec()),
        ]);
        let mut bytes = [0; 32];
        assert_eq!(ready(rx.read(&mut bytes)), Ok(5));
        assert_eq!(&bytes[..5], b"help\n");
        assert_eq!(traffic.borrow().waits, 3);
    }
    #[test]
    fn tx_splits_packets_finishes_full_packets_and_recovers_after_disconnect() {
        let (traffic, _, mut tx) = io();
        traffic.borrow_mut().fail_write = true;
        ready(tx.write(&[7; 128])).unwrap();
        assert_eq!(
            traffic
                .borrow()
                .writes
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            [64, 64, 0]
        );
        assert_eq!(traffic.borrow().waits, 4);
        traffic.borrow_mut().writes.clear();
        ready(tx.write(&[9; 65])).unwrap();
        assert_eq!(traffic.borrow().writes, vec![vec![9; 64], vec![9]]);
        traffic.borrow_mut().writes.clear();
        ready(tx.write(&[])).unwrap();
        assert!(traffic.borrow().writes.is_empty());
    }
}
