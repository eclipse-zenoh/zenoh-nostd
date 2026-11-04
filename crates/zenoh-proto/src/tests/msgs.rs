use crate::{exts::*, msgs::*};

macro_rules! roundtrip {
    ($ty:ty) => {{
        let mut rand = [0u8; MAX_PAYLOAD_SIZE];
        let mut data = [0u8; MAX_PAYLOAD_SIZE];

        for _ in 0..NUM_ITER {
            let value = <$ty>::rand(&mut &mut rand[..]);

            let len = $crate::ZLen::z_len(&value);
            $crate::ZEncode::z_encode(&value, &mut &mut data[..]).unwrap();

            let ret = <$ty as $crate::ZDecode>::z_decode(&mut &data[..len]).unwrap();

            assert_eq!(ret, value);
        }
    }};

    (ext, $ty:ty) => {{
        let mut rand = [0u8; MAX_PAYLOAD_SIZE];
        let mut data = [0u8; MAX_PAYLOAD_SIZE];

        for _ in 0..NUM_ITER {
            let value = <$ty>::rand(&mut &mut rand[..]);

            $crate::zext_encode::<_, 0x1, true>(&value, &mut &mut data[..], false).unwrap();

            let ret = $crate::zext_decode::<$ty>(&mut &data[..]).unwrap();

            assert_eq!(ret, value);
        }
    }};
}

macro_rules! roundtrips {
    (ext, $namespace:ident, $($ty:ty),* $(,)?) => {
        $(
            paste::paste! {
                #[test]
                fn [<$namespace _proto_ext_ $ty:lower>]() {
                    roundtrip!(ext, $ty);
                }
            }
        )*
    };

    ($namespace:ident, $($ty:ty),* $(,)?) => {
        $(
            paste::paste! {
                #[test]
                fn [<$namespace _proto_ $ty:lower>]() {
                    roundtrip!($ty);
                }
            }
        )*
    };
}

const NUM_ITER: usize = 100;
const MAX_PAYLOAD_SIZE: usize = 512;

roundtrips!(ext, zenoh, EntityGlobalId, SourceInfo, Value, Attachment);
roundtrips!(zenoh, Err, Put, Query, Reply,);

roundtrips!(
    ext,
    network,
    QoS,
    NodeId,
    QueryTarget,
    Budget,
    QueryableInfo
);

roundtrips!(
    network,
    DeclareKeyExpr,
    UndeclareKeyExpr,
    DeclareSubscriber,
    UndeclareSubscriber,
    DeclareQueryable,
    UndeclareQueryable,
    DeclareToken,
    UndeclareToken,
    DeclareFinal,
    Declare,
    Interest,
    InterestFinal,
    Push,
    Request,
    Response,
    ResponseFinal,
);

roundtrips!(ext, transport, Auth, Patch);
roundtrips!(
    transport,
    Close,
    FrameHeader,
    InitSyn,
    InitAck,
    KeepAlive,
    OpenSyn,
    OpenAck
);

/// zenoh writes the key expression suffix of the `WireExpr` extension without a length prefix:
/// it fills the rest of the extension.
#[test]
fn network_proto_undeclare_wire_expr_ext_from_zenoh() {
    // UndeclareToken { id: 5, ext 0x0f (mandatory, ZBuf, len 5): flags N|M, scope 0, "a/b" }
    let data = [0x87, 0x05, 0x5f, 0x05, 0x03, 0x00, b'a', b'/', b'b'];

    let token = <UndeclareToken as crate::ZDecode>::z_decode(&mut &data[..]).unwrap();
    let wire_expr = token.wire_expr.as_ref().unwrap();
    assert_eq!(token.id, 5);
    assert_eq!(wire_expr.scope, 0);
    assert_eq!(wire_expr.suffix, "a/b");

    let mut buf = [0u8; 16];
    let len = crate::ZLen::z_len(&token);
    crate::ZEncode::z_encode(&token, &mut &mut buf[..]).unwrap();
    // Same extension body; zenoh also sets the mandatory bit in the extension header.
    assert_eq!(&buf[3..len], &data[3..]);
}
