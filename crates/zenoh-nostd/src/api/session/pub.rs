use zenoh_proto::{
    SessionError,
    exts::Attachment,
    exts::QoS,
    fields::{Encoding, Locality, Reliability, Timestamp},
    keyexpr,
};

use crate::{
    api::session::{Session, put::PutBuilder},
    config::ZSessionConfig,
};

pub struct Publisher<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    session: &'a Session<'res, Config>,

    ke: &'a keyexpr,

    encoding: Encoding<'a>,
    timestamp: Option<Timestamp>,
    attachment: Option<Attachment<'a>>,
    reliability: Reliability,
}

impl<'a, 'res, Config> Publisher<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub fn put(&self, payload: &'a [u8]) -> PutBuilder<'a, 'res, Config> {
        PutBuilder {
            session: self.session,
            ke: self.ke,
            payload,
            encoding: self.encoding.clone(),
            timestamp: self.timestamp,
            attachment: self.attachment.clone(),
            allowed_destination: Locality::default(),
            qos: QoS::default(),
            reliability: self.reliability,
        }
    }

    pub async fn delete(&self, timestamp: Option<Timestamp>, attachment: Option<&'a [u8]>) -> core::result::Result<(), SessionError> {
        let mut builder = self.session.delete(self.ke);
        if let Some(ts) = timestamp {
            builder = builder.timestamp(ts);
        }
        if let Some(att) = attachment {
            builder = builder.attachment(att);
        }
        builder.finish().await
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
    reliability: Reliability,
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
            reliability: Reliability::default(),
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

    pub fn reliability(mut self, reliability: Reliability) -> Self {
        self.reliability = reliability;
        self
    }

    pub async fn finish(self) -> core::result::Result<Publisher<'a, 'res, Config>, SessionError> {
        // TODO: send interest msg
        Ok(Publisher {
            session: self.session,
            ke: self.ke,
            encoding: self.encoding,
            timestamp: self.timestamp,
            attachment: self.attachment,
            reliability: self.reliability,
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
