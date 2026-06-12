use zenoh_proto::{exts::*, fields::*, msgs::*, *};

use crate::{
    api::{
        callbacks::{ZCallbacks as _, ZDynCallback as _},
        sample::Sample,
        session::Session,
    },
    config::ZSessionConfig,
    io::transport::ZTransportLinkTx,
};

pub struct PutBuilder<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub(crate) session: &'a Session<'res, Config>,

    pub(crate) ke: &'a keyexpr,
    pub(crate) payload: &'a [u8],

    pub(crate) encoding: Encoding<'a>,
    pub(crate) timestamp: Option<Timestamp>,
    pub(crate) attachment: Option<Attachment<'a>>,
    pub(crate) qos: QoS,
    pub(crate) reliability: Reliability,
}

impl<'a, 'res, Config> PutBuilder<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub(crate) fn new(
        session: &'a Session<'res, Config>,
        ke: &'a keyexpr,
        payload: &'a [u8],
    ) -> Self {
        Self {
            session,
            ke,
            payload,
            encoding: Encoding::default(),
            timestamp: None,
            attachment: None,
            qos: QoS::default(),
            reliability: Reliability::default(),
        }
    }

    pub fn payload(mut self, payload: &'a [u8]) -> Self {
        self.payload = payload;
        self
    }

    pub fn qos(mut self, qos: QoS) -> Self {
        self.qos = qos;
        self
    }

    pub fn reliability(mut self, reliability: Reliability) -> Self {
        self.reliability = reliability;
        self
    }

    pub fn encoding(mut self, encoding: Encoding<'a>) -> Self {
        self.encoding = encoding;
        self
    }

    pub fn timestamp(mut self, timestamp: Timestamp) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn attachment(mut self, attachment: &'a [u8]) -> Self {
        self.attachment = Some(Attachment { buffer: attachment });
        self
    }

    pub async fn finish(self) -> core::result::Result<(), SessionError> {
        let ke = self.ke;
        let payload = self.payload;
        let encoding_id = self.encoding.id;

        let msg = Push {
            wire_expr: WireExpr::from(ke),
            payload: PushBody::Put(Put {
                payload,
                encoding: self.encoding,
                timestamp: self.timestamp,
                attachment: self.attachment,
                ..Default::default()
            }),
            timestamp: self.timestamp,
            ..Default::default()
        };

        self.session
            .driver
            .tx()
            .await
            .send(core::iter::once(NetworkMessage {
                reliability: self.reliability,
                qos: self.qos,
                body: NetworkBody::Push(msg),
            }))
            .await?;

        // Local delivery: call own subscriber callbacks for matching key expressions.
        // The broker does not route publications back to the sender, so the session
        // must deliver locally to any subscriber declared on this same session.
        let sample = Sample::with_encoding_id(ke, payload, encoding_id);
        let mut state = self.session.state().await;
        for cb in state.sub_callbacks.intersects(ke) {
            cb.call_try_sync(&sample).await;
        }

        Ok(())
    }
}

impl<'res, Config> Session<'res, Config>
where
    Config: ZSessionConfig,
{
    pub fn put<'a>(&'a self, ke: &'a keyexpr, payload: &'a [u8]) -> PutBuilder<'a, 'res, Config> {
        PutBuilder::new(self, ke, payload)
    }
}
