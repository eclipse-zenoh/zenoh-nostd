use zenoh_proto::{
    SessionError,
    exts::Attachment,
    fields::{Encoding, Timestamp},
    keyexpr,
};

use crate::{
    api::session::{Session, put::PutBuilder},
    config::ZSessionConfig,
};

pub struct Publisher<'a, 's, 'res, Config>
where
    Config: ZSessionConfig,
{
    session: &'a Session<'s, 'res, Config>,

    ke: &'a keyexpr,

    encoding: Encoding<'a>,
    timestamp: Option<Timestamp>,
    attachment: Option<Attachment<'a>>,
}

impl<'a, 's, 'res, Config> Publisher<'a, 's, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub fn put(&self, payload: &'a [u8]) -> PutBuilder<'a, 's, 'res, Config> {
        PutBuilder {
            session: self.session,
            ke: self.ke,
            payload,
            encoding: self.encoding.clone(),
            timestamp: self.timestamp,
            attachment: self.attachment.clone(),
        }
    }

    #[allow(dead_code)]
    async fn undeclare(self) -> core::result::Result<(), SessionError> {
        todo!("send undeclare interest")
    }

    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }
}

pub struct PublisherBuilder<'a, 's, 'res, Config>
where
    Config: ZSessionConfig,
{
    session: &'a Session<'s, 'res, Config>,

    ke: &'a keyexpr,
    encoding: Encoding<'a>,
    timestamp: Option<Timestamp>,
    attachment: Option<Attachment<'a>>,
}

impl<'a, 's, 'res, Config> PublisherBuilder<'a, 's, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub(crate) fn new(session: &'a Session<'s, 'res, Config>, ke: &'a keyexpr) -> Self {
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

    pub async fn finish(self) -> core::result::Result<Publisher<'a, 's, 'res, Config>, SessionError> {
        // TODO: send interest msg
        Ok(Publisher {
            session: self.session,
            ke: self.ke,
            encoding: self.encoding,
            timestamp: self.timestamp,
            attachment: self.attachment,
        })
    }
}

impl<'s, 'res, Config> Session<'s, 'res, Config>
where
    Config: ZSessionConfig,
    'res: 's,
{
    pub fn declare_publisher<'a>(&'a self, ke: &'a keyexpr) -> PublisherBuilder<'a, 's, 'res, Config> {
        PublisherBuilder::new(self, ke)
    }
}
