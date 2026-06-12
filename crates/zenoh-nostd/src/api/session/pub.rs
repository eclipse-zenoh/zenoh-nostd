use zenoh_proto::{
    SessionError,
    exts::{Attachment, QoS},
    fields::*,
    keyexpr,
    msgs::*,
};

use crate::{
    api::session::{Session, put::PutBuilder},
    config::ZSessionConfig,
    io::transport::ZTransportLinkTx,
};

pub struct Publisher<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    id: u32,
    session: &'a Session<'res, Config>,

    ke: &'a keyexpr,

    encoding: Encoding<'a>,
    timestamp: Option<Timestamp>,
    attachment: Option<Attachment<'a>>,
}

impl<'a, 'res, Config> Publisher<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn put(&self, payload: &'a [u8]) -> PutBuilder<'a, 'res, Config> {
        PutBuilder {
            session: self.session,
            ke: self.ke,
            payload,
            encoding: self.encoding.clone(),
            timestamp: self.timestamp,
            attachment: self.attachment.clone(),
            qos: QoS::default(),
            reliability: Reliability::default(),
        }
    }

    pub async fn undeclare(self) -> core::result::Result<(), SessionError> {
        self.session
            .driver
            .tx()
            .await
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::InterestFinal(InterestFinal {
                    id: self.id,
                    ..Default::default()
                }),
            }))
            .await?;
        Ok(())
    }

    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }
}

pub struct PublisherBuilder<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    session: &'a Session<'res, Config>,

    ke: &'a keyexpr,
    encoding: Encoding<'a>,
    timestamp: Option<Timestamp>,
    attachment: Option<Attachment<'a>>,
}

impl<'a, 'res, Config> PublisherBuilder<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub(crate) fn new(session: &'a Session<'res, Config>, ke: &'a keyexpr) -> Self {
        Self {
            session,
            ke,
            encoding: Encoding::default(),
            timestamp: None,
            attachment: None,
        }
    }

    pub fn keyexpr(mut self, ke: &'a keyexpr) -> Self {
        self.ke = ke;
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

    pub async fn finish(self) -> core::result::Result<Publisher<'a, 'res, Config>, SessionError> {
        // Allocate ID under the state lock, then release before the async send.
        let id = {
            let mut state = self.session.state().await;
            state.next()
        };

        // Declare publisher interest: ask the router to route matching subscriber
        // declarations to us (current and future subscribers).
        self.session
            .driver
            .tx()
            .await
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Interest(Interest {
                    id,
                    mode: InterestMode::CurrentFuture,
                    inner: InterestInner {
                        options: InterestOptions::SUBSCRIBERS.options,
                        wire_expr: Some(WireExpr::from(self.ke)),
                    },
                    ..Default::default()
                }),
            }))
            .await?;

        Ok(Publisher {
            id,
            session: self.session,
            ke: self.ke,
            encoding: self.encoding,
            timestamp: self.timestamp,
            attachment: self.attachment,
        })
    }
}

impl<'res, Config> Session<'res, Config>
where
    Config: ZSessionConfig,
{
    pub fn declare_publisher<'a>(&'a self, ke: &'a keyexpr) -> PublisherBuilder<'a, 'res, Config> {
        PublisherBuilder::new(self, ke)
    }
}
