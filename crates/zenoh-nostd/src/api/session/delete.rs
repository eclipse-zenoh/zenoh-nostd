use zenoh_proto::{exts::*, fields::*, msgs::{Del, *}, *};

use crate::{api::session::Session, config::ZSessionConfig, io::transport::ZTransportLinkTx};

pub struct DeleteBuilder<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub(crate) session: &'a Session<'res, Config>,

    pub(crate) ke: &'a keyexpr,

    pub(crate) timestamp: Option<Timestamp>,
    pub(crate) attachment: Option<Attachment<'a>>,
    pub(crate) qos: QoS,
    pub(crate) reliability: Reliability,
}

impl<'a, 'res, Config> DeleteBuilder<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub(crate) fn new(session: &'a Session<'res, Config>, ke: &'a keyexpr) -> Self {
        Self {
            session,
            ke,
            timestamp: None,
            attachment: None,
            qos: QoS::default(),
            reliability: Reliability::default(),
        }
    }

    pub fn keyexpr(mut self, ke: &'a keyexpr) -> Self {
        self.ke = ke;
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

    pub fn qos(mut self, qos: QoS) -> Self {
        self.qos = qos;
        self
    }

    pub fn reliability(mut self, reliability: Reliability) -> Self {
        self.reliability = reliability;
        self
    }

    pub async fn finish(self) -> core::result::Result<(), SessionError> {
        let msg = Push {
            wire_expr: WireExpr::from(self.ke),
            payload: PushBody::Del(Del {
                timestamp: self.timestamp,
                sinfo: None,
                attachment: self.attachment,
            }),
            timestamp: self.timestamp,
            ..Default::default()
        };

        Ok(self
            .session
            .driver
            .tx()
            .await
            .send(core::iter::once(NetworkMessage {
                reliability: self.reliability,
                qos: self.qos,
                body: NetworkBody::Push(msg),
            }))
            .await?)
    }
}

impl<'res, Config> Session<'res, Config>
where
    Config: ZSessionConfig,
{
    pub fn delete<'a>(&'a self, ke: &'a keyexpr) -> DeleteBuilder<'a, 'res, Config> {
        DeleteBuilder::new(self, ke)
    }
}
