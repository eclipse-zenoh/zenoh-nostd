use core::fmt::Display;
use core::time::Duration;

use zenoh_proto::{
    EitherError, TransportError, ZBodyDecode, ZDecode, ZReadable, fields::Resolution, msgs::*,
};

use crate::{ZTransportRx, transport::TransportTx};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum State {
    Opened,
    Used,
    Synchronized { last_received: Duration },
    Closed,
}

#[derive(Debug)]
pub struct TransportRx<Buff> {
    buff: Buff,

    cursor: usize,
    batch_size: usize,

    sn: u32,
    resolution: Resolution,
    lease: Duration,

    state: State,

    ignore_invalid_sn: bool,

    max_fragments: usize,
    #[allow(dead_code)]
    fragment_sn: Option<u32>,
}

impl<Buff> TransportRx<Buff> {
    pub(crate) fn new(
        buff: Buff,

        batch_size: usize,
        sn: u32,
        resolution: Resolution,
        lease: Duration,
    ) -> Self {
        Self {
            buff,

            cursor: 0,
            batch_size,

            sn,
            resolution,
            lease,

            state: State::Opened,
            ignore_invalid_sn: false,

            max_fragments: 1,
            fragment_sn: None,
        }
    }

    pub(crate) fn into_inner(self) -> Buff {
        self.buff
    }

    pub fn with_max_fragments(mut self, max: usize) -> Self {
        self.max_fragments = max;
        self
    }

    fn reassemble_fragments(&mut self, size: usize)
    where
        Buff: AsMut<[u8]> + AsRef<[u8]>,
    {
        if self.max_fragments <= 1 {
            return;
        }

        let buff = self.buff.as_mut();
        let mut fragment_sn: Option<u32> = None;
        let mut write_pos = 0;
        let mut read_pos = 0;
        let mut num_fragments = 0;

        while read_pos < size {
            if read_pos >= size {
                break;
            }
            let id = buff[read_pos] & 0x1f;

            if id != 0x05 {
                break;
            }

            let mut frame_reader = &buff[read_pos..];
            if let Ok(header) = <FrameHeader as ZDecode>::z_decode(&mut frame_reader) {
                let frame_header_len = buff[read_pos..].len() - frame_reader.len();

                match fragment_sn {
                    None => {
                        // First frame: stay in place. Track payload extent.
                        // frame_reader.len() is unbounded — cap at size
                        let max_payload = if read_pos + frame_header_len + frame_reader.len() > size
                        {
                            size - read_pos - frame_header_len
                        } else {
                            frame_reader.len()
                        };
                        fragment_sn = Some(header.sn);
                        write_pos = read_pos + frame_header_len + max_payload;
                        read_pos = write_pos;
                        num_fragments = 1;
                    }
                    Some(sn) if sn == header.sn => {
                        let payload_start = read_pos + frame_header_len;
                        let max_payload = if payload_start + frame_reader.len() > size {
                            size - payload_start
                        } else {
                            frame_reader.len()
                        };
                        buff.copy_within(payload_start..payload_start + max_payload, write_pos);
                        write_pos += max_payload;
                        read_pos += frame_header_len + max_payload;
                        num_fragments += 1;
                    }
                    Some(_) => {
                        break;
                    }
                }
            } else {
                break;
            }
        }

        if num_fragments > 1 {
            for b in buff[write_pos..size].iter_mut() {
                *b = 0;
            }
        }
    }

    pub(crate) fn flush_transport(&mut self) -> impl Iterator<Item = (TransportMessage<'_>, &[u8])>
    where
        Buff: AsMut<[u8]> + AsRef<[u8]>,
    {
        let size = core::cmp::min(
            self.buff.as_ref().len(),
            core::cmp::min(self.batch_size, self.cursor),
        );
        self.cursor = 0;

        if self.max_fragments > 1 {
            self.reassemble_fragments(size);
        }

        let size_after = core::cmp::min(self.buff.as_ref().len(), size);
        let mut reader = &self.buff.as_ref()[..size_after];
        let mut last_frame = None;
        let sn = &mut self.sn;
        let resolution = self.resolution;
        let ignore = self.ignore_invalid_sn;

        core::iter::from_fn(move || {
            Self::decode(&mut reader, &mut last_frame, sn, resolution, ignore)
        })
        .filter_map(|m| match m.0 {
            Message::Transport(msg) => Some((msg, m.1)),
            _ => None,
        })
    }

    pub fn ignore_invalid_sn(&mut self) {
        self.ignore_invalid_sn = true;
    }

    pub fn sync(&mut self, tx: Option<&TransportTx<Buff>>, now: Duration) {
        if let Some(tx) = tx
            && tx.closed()
        {
            self.state = State::Closed;
            return;
        }

        if self.should_close(now) {
            self.state = State::Closed;
        }

        if matches!(self.state, State::Used | State::Opened) {
            self.state = State::Synchronized { last_received: now };
        };
    }

    pub fn next_timeout(&self) -> Duration {
        match self.state {
            State::Opened | State::Closed | State::Used => Duration::from_secs(0),
            State::Synchronized { last_received } => last_received + self.lease,
        }
    }

    pub fn should_close(&self, now: Duration) -> bool {
        match self.state {
            State::Opened | State::Closed | State::Used => false,
            State::Synchronized { last_received } => now > last_received + self.lease,
        }
    }

    pub fn closed(&self) -> bool {
        matches!(self.state, State::Closed)
    }

    pub(crate) fn decode<'a>(
        reader: &mut &'a [u8],
        last_frame: &mut Option<FrameHeader>,
        sn: &mut u32,
        resolution: Resolution,
        ignore: bool,
    ) -> Option<(Message<'a>, &'a [u8])>
    where
        Buff: AsRef<[u8]>,
    {
        if !reader.can_read() {
            return None;
        }

        let (data, start) = (reader.as_ptr(), reader.len());

        let header = reader
            .read_u8()
            .expect("reader should not be empty at this stage");

        macro_rules! decode {
            ($ty:ty) => {
                match <$ty as ZBodyDecode>::z_body_decode(reader, header) {
                    Ok(msg) => msg,
                    Err(e) => {
                        zenoh_proto::error!(
                            "Failed to decode message of type {}: {}. Skipping the rest of the message - {}",
                            core::any::type_name::<$ty>(),
                            e,
                            zenoh_proto::zctx!()
                        );

                        return None;
                    }
                }
            };

            (@Transport $ty:ident) => {{
                last_frame.take();
                Message::Transport(TransportMessage:: $ty (decode!($ty)))
            }};

            (@Network $ty:ident) => {
                Message::Network(NetworkMessage {
                    reliability: last_frame.as_ref().expect("Should be in frame").reliability,
                    qos: last_frame.as_ref().expect("Should be in frame").qos,
                    body: NetworkBody:: $ty (decode!($ty))
                })
            };
        }

        let ack = header & 0b0010_0000 != 0;
        let net = last_frame.is_some();
        let ifinal = header & 0b0110_0000 == 0;
        let id = header & 0b0001_1111;

        let body = match id {
            FrameHeader::ID => {
                let header = decode!(FrameHeader);

                *sn = sn.wrapping_add(1);

                if !ignore {
                    // Check for missed messages regarding resolution
                    let _ = resolution;

                    if header.sn < *sn - 1 {
                        zenoh_proto::error!(
                            "Inconsistent `SN` value {}, expected higher than {}",
                            header.sn,
                            *sn - 1
                        );
                        return None;
                    } else if header.sn != *sn - 1 {
                        zenoh_proto::debug!("Transport missed {} messages", header.sn - *sn + 1);
                    }
                }

                last_frame.replace(header);

                return Self::decode(reader, last_frame, sn, resolution, ignore);
            }
            InitAck::ID if ack => decode!(@Transport InitAck),
            InitSyn::ID => decode!(@Transport InitSyn),
            OpenAck::ID if ack => decode!(@Transport OpenAck),
            OpenSyn::ID => decode!(@Transport OpenSyn),
            Close::ID => decode!(@Transport Close),
            KeepAlive::ID => decode!(@Transport KeepAlive),
            Push::ID if net => decode!(@Network Push),
            Request::ID if net => decode!(@Network Request),
            Response::ID if net => decode!(@Network Response),
            ResponseFinal::ID if net => decode!(@Network ResponseFinal),
            InterestFinal::ID if net && ifinal => decode!(@Network InterestFinal),
            Interest::ID if net => decode!(@Network Interest),
            Declare::ID if net => decode!(@Network Declare),
            _ => {
                zenoh_proto::error!(
                    "Unrecognized message header: {:08b}. Skipping the rest of the message - {}",
                    header,
                    zenoh_proto::zctx!()
                );
                return None;
            }
        };

        let len = start - reader.len();
        Some((body, unsafe { core::slice::from_raw_parts(data, len) }))
    }
}

impl<Buff> ZTransportRx for TransportRx<Buff>
where
    Buff: AsMut<[u8]> + AsRef<[u8]>,
{
    fn decode_prefixed(&mut self, mut read: &[u8]) -> core::result::Result<(), TransportError>
    where
        Buff: AsMut<[u8]> + AsRef<[u8]>,
    {
        if read.is_empty() || self.state == State::Closed {
            return Ok(());
        }

        self.decode_prefixed_with(|data| {
            let size = data.len().min(read.len());
            let (ret, remain) = read.split_at(size);
            data[..size].copy_from_slice(ret);
            read = remain;
            Ok::<_, TransportError>(size)
        })
        .map_err(|e| e.flatten())
    }

    fn decode_raw(&mut self, read: &[u8]) -> core::result::Result<(), TransportError>
    where
        Buff: AsMut<[u8]> + AsRef<[u8]>,
    {
        if read.is_empty() || self.state == State::Closed {
            return Ok(());
        }

        let max = core::cmp::min(self.buff.as_ref().len(), self.batch_size);
        let buff = &mut self.buff.as_mut()[self.cursor..max];

        let len = read.len();
        if buff.len() < read.len() {
            zenoh_proto::zbail!(@log TransportError::TransportRxFull);
        }

        buff[..len].copy_from_slice(read);

        if len > 0 {
            self.state = State::Used;
        }

        self.cursor += len;

        Ok(())
    }

    fn decode_prefixed_with<E>(
        &mut self,
        mut read: impl FnMut(&mut [u8]) -> core::result::Result<usize, E>,
    ) -> core::result::Result<(), EitherError<TransportError, E>>
    where
        Buff: AsMut<[u8]> + AsRef<[u8]>,
        E: Display,
    {
        if self.state == State::Closed {
            return Ok(());
        }

        let max = core::cmp::min(self.buff.as_ref().len(), self.batch_size);
        let buff = &mut self.buff.as_mut()[self.cursor..max];

        if 2 > buff.len() {
            zenoh_proto::zbail!(@log TransportError::TransportRxFull);
        }

        let mut len = [0u8; 2];
        let l = read(&mut len).map_err(EitherError::B)?;

        if l == 0 {
            return Ok(());
        } else if l != 2 {
            zenoh_proto::zbail!(@log TransportError::InvalidAttribute)
        }

        let len = u16::from_le_bytes(len) as usize;
        if len > u16::MAX as usize || len > buff.len() {
            zenoh_proto::zbail!(@log TransportError::InvalidAttribute);
        }

        if read(&mut buff[..len]).map_err(EitherError::B)? != len {
            zenoh_proto::zbail!(@log TransportError::InvalidAttribute)
        }

        if len > 0 {
            self.state = State::Used;
        }

        self.cursor += len;

        Ok(())
    }

    fn decode_raw_with<E>(
        &mut self,
        mut read: impl FnMut(&mut [u8]) -> core::result::Result<usize, E>,
    ) -> core::result::Result<(), EitherError<TransportError, E>>
    where
        Buff: AsMut<[u8]> + AsRef<[u8]>,
        E: Display,
    {
        if self.state == State::Closed {
            return Ok(());
        }

        let max = core::cmp::min(self.buff.as_ref().len(), self.batch_size);
        let buff = &mut self.buff.as_mut()[self.cursor..max];

        let len = read(buff).map_err(EitherError::B)?;

        if len > 0 {
            self.state = State::Used;
        }

        self.cursor += len;

        Ok(())
    }

    async fn decode_prefixed_with_async<E>(
        &mut self,
        mut read: impl AsyncFnMut(&mut [u8]) -> core::result::Result<usize, E>,
    ) -> core::result::Result<(), EitherError<TransportError, E>>
    where
        Buff: AsMut<[u8]> + AsRef<[u8]>,
        E: Display,
    {
        if self.state == State::Closed {
            return Ok(());
        }

        let max = core::cmp::min(self.buff.as_ref().len(), self.batch_size);
        let buff = &mut self.buff.as_mut()[self.cursor..max];

        if 2 > buff.len() {
            zenoh_proto::zbail!(@log TransportError::TransportRxFull);
        }

        let mut len = [0u8; 2];
        let l = read(&mut len).await.map_err(EitherError::B)?;

        if l == 0 {
            return Ok(());
        } else if l != 2 {
            zenoh_proto::zbail!(@log TransportError::InvalidAttribute)
        }

        let len = u16::from_le_bytes(len) as usize;
        if len > u16::MAX as usize || len > buff.len() {
            zenoh_proto::zbail!(@log TransportError::InvalidAttribute);
        }

        if read(&mut buff[..len]).await.map_err(EitherError::B)? != len {
            zenoh_proto::zbail!(@log TransportError::InvalidAttribute)
        }

        if len > 0 {
            self.state = State::Used;
        }

        self.cursor += len;

        Ok(())
    }

    async fn decode_raw_with_async<E>(
        &mut self,
        mut read: impl AsyncFnMut(&mut [u8]) -> core::result::Result<usize, E>,
    ) -> core::result::Result<(), EitherError<TransportError, E>>
    where
        Buff: AsMut<[u8]> + AsRef<[u8]>,
        E: Display,
    {
        if self.state == State::Closed {
            return Ok(());
        }

        let max = core::cmp::min(self.buff.as_ref().len(), self.batch_size);
        let buff = &mut self.buff.as_mut()[self.cursor..max];

        let len = read(buff).await.map_err(EitherError::B)?;

        if len > 0 {
            self.state = State::Used;
        }

        self.cursor += len;

        Ok(())
    }

    fn flush(&mut self) -> impl Iterator<Item = (NetworkMessage<'_>, &'_ [u8])>
    where
        Buff: AsMut<[u8]> + AsRef<[u8]>,
    {
        let size = core::cmp::min(
            self.buff.as_ref().len(),
            core::cmp::min(self.batch_size, self.cursor),
        );
        self.clear();

        if self.max_fragments > 1 {
            self.reassemble_fragments(size);
        }

        let size_after = core::cmp::min(self.buff.as_ref().len(), size);
        let mut reader = &self.buff.as_ref()[..size_after];
        let mut last_frame = None;
        let sn = &mut self.sn;
        let resolution = self.resolution;
        let ignore = self.ignore_invalid_sn;

        core::iter::from_fn(move || {
            Self::decode(&mut reader, &mut last_frame, sn, resolution, ignore)
        })
        .filter_map(|m| match m.0 {
            Message::Network(msg) => Some((msg, m.1)),
            _ => None,
        })
    }

    fn clear(&mut self) {
        self.cursor = 0;
    }
}
