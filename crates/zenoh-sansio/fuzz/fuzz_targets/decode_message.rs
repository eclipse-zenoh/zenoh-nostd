#![no_main]

use libfuzzer_sys::fuzz_target;
use zenoh_proto::ZDecode;

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }

    // Decode each message type from the fuzz data at different offsets
    // to exercise different message parsers.
    let _ = <zenoh_proto::msgs::Put as ZDecode>::z_decode(&mut &data[..]);
    let _ = <zenoh_proto::msgs::Del as ZDecode>::z_decode(&mut &data[..]);
    let _ = <zenoh_proto::msgs::Declare as ZDecode>::z_decode(&mut &data[..]);
    let _ = <zenoh_proto::msgs::KeepAlive as ZDecode>::z_decode(&mut &data[..]);
    let _ = <zenoh_proto::msgs::Close as ZDecode>::z_decode(&mut &data[..]);
    let _ = <zenoh_proto::msgs::InitSyn as ZDecode>::z_decode(&mut &data[..]);
    let _ = <zenoh_proto::msgs::InitAck as ZDecode>::z_decode(&mut &data[..]);
    let _ = <zenoh_proto::msgs::OpenSyn as ZDecode>::z_decode(&mut &data[..]);
    let _ = <zenoh_proto::msgs::OpenAck as ZDecode>::z_decode(&mut &data[..]);

    let _ = <zenoh_proto::msgs::Response as ZDecode>::z_decode(&mut &data[..]);
    let _ = <zenoh_proto::msgs::ResponseFinal as ZDecode>::z_decode(&mut &data[..]);

    // Feed suffix of data to exercise misalignment
    if data.len() > 4 {
        let suffix = &data[data.len() / 2..];
        let _ = <zenoh_proto::msgs::Push as ZDecode>::z_decode(&mut &suffix[..]);
        let _ = <zenoh_proto::msgs::Request as ZDecode>::z_decode(&mut &suffix[..]);
        let _ = <zenoh_proto::msgs::Response as ZDecode>::z_decode(&mut &suffix[..]);
        let _ = <zenoh_proto::msgs::ResponseFinal as ZDecode>::z_decode(&mut &suffix[..]);
    }
});
