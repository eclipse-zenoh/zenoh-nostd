use core::str::FromStr;

use zenoh_proto::{CollectionError, SessionError, exts::QoS, fields::ZenohIdProto, keyexpr};
use zenoh_proto::fields::Encoding;
use zenoh_proto::msgs::exts::QueryTarget;

use crate::{api::session::Session, config::ZSessionConfig};

pub struct QueryableQuery<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    session: &'a Session<'res, Config>,
    rid: u32,
    ke: &'a keyexpr,
    parameters: Option<&'a str>,
    payload: Option<&'a [u8]>,
    query_target: QueryTarget,
    qos: QoS,
    finalized: bool,
    encoding: Option<Encoding<'a>>,
    attachment: Option<&'a [u8]>,
    source_info: Option<ZenohIdProto>,
    accept_replies: Option<&'a keyexpr>,
}

impl<'a, 'res, Config> QueryableQuery<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub(crate) fn new(
        session: &'a Session<'res, Config>,
        rid: u32,
        ke: &'a keyexpr,
        parameters: Option<&'a str>,
        payload: Option<&'a [u8]>,
        query_target: QueryTarget,
        qos: QoS,
    ) -> Self {
        Self {
            session,
            rid,
            ke,
            parameters,
            payload,
            query_target,
            qos,
            finalized: false,
            encoding: None,
            attachment: None,
            source_info: None,
            accept_replies: None,
        }
    }

    pub fn zid(&self) -> ZenohIdProto {
        self.session.zid()
    }

    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }

    pub fn parameters(&self) -> Option<&str> {
        self.parameters
    }

    pub fn payload(&self) -> Option<&[u8]> {
        self.payload
    }

    pub fn query_target(&self) -> QueryTarget {
        self.query_target
    }

    pub fn qos(&self) -> QoS {
        self.qos
    }

    pub fn encoding(&self) -> Option<&Encoding<'a>> {
        self.encoding.as_ref()
    }

    pub fn attachment(&self) -> Option<&[u8]> {
        self.attachment
    }

    pub fn source_info(&self) -> Option<ZenohIdProto> {
        self.source_info
    }

    #[allow(clippy::unnecessary_map_or)]
    pub fn accept_replies(&self, ke: &keyexpr) -> bool {
        self.accept_replies.map_or(true, |ar| ar.intersects(ke))
    }

    pub fn reply_del(
        &self,
        ke: &keyexpr,
    ) -> core::result::Result<(), SessionError> {
        self.session.reply_del(self.rid, ke)
    }

    pub fn reply(
        &self,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        self.session.reply(self.rid, ke, payload)
    }

    pub fn err(
        &self,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        self.session.err(self.rid, ke, payload)
    }

    pub fn finalize(&mut self) -> core::result::Result<(), SessionError> {
        if !self.finalized {
            self.session.finalize(self.rid)?;
            self.finalized = true
        }

        Ok(())
    }
}

pub struct FixedCapacityQueryableQuery<
    Config,
    const MAX_KEYEXPR: usize,
    const MAX_PARAMETERS: usize,
    const MAX_PAYLOAD: usize,
> where
    Config: ZSessionConfig + 'static,
{
    session: &'static Session<'static, Config>,
    rid: u32,
    ke: heapless::String<MAX_KEYEXPR>,
    parameters: Option<heapless::String<MAX_PARAMETERS>>,
    payload: Option<heapless::Vec<u8, MAX_PAYLOAD>>,
    query_target: QueryTarget,
    qos: QoS,
    finalized: bool,
}

impl<Config, const MAX_KEYEXPR: usize, const MAX_PARAMETERS: usize, const MAX_PAYLOAD: usize>
    FixedCapacityQueryableQuery<Config, MAX_KEYEXPR, MAX_PARAMETERS, MAX_PAYLOAD>
where
    Config: ZSessionConfig,
{
    pub fn keyexpr(&self) -> &keyexpr {
        keyexpr::from_str_unchecked(self.ke.as_str())
    }

    pub fn parameters(&self) -> Option<&str> {
        self.parameters.as_ref().map(|p| p.as_str())
    }

    pub fn payload(&self) -> Option<&[u8]> {
        self.payload.as_ref().map(|p| p.as_slice())
    }

    pub fn query_target(&self) -> QueryTarget {
        self.query_target
    }

    pub fn qos(&self) -> QoS {
        self.qos
    }

    pub fn reply(
        &self,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        self.session.reply(self.rid, ke, payload)
    }

    pub fn err(
        &self,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        self.session.err(self.rid, ke, payload)
    }

    pub fn finalize(&mut self) -> core::result::Result<(), SessionError> {
        if !self.finalized {
            self.session.finalize(self.rid)?;
            self.finalized = true
        }

        Ok(())
    }
}

impl<'a, Config, const MAX_KEYEXPR: usize, const MAX_PARAMETERS: usize, const MAX_PAYLOAD: usize>
    TryFrom<(
        &QueryableQuery<'a, 'static, Config>,
        &'static Session<'static, Config>,
    )> for FixedCapacityQueryableQuery<Config, MAX_KEYEXPR, MAX_PARAMETERS, MAX_PAYLOAD>
where
    Config: ZSessionConfig,
{
    type Error = CollectionError;

    fn try_from(
        value: (
            &QueryableQuery<'a, 'static, Config>,
            &'static Session<'static, Config>,
        ),
    ) -> Result<Self, Self::Error> {
        let (value, session) = value;

        Ok(Self {
            session,
            rid: value.rid,
            ke: heapless::String::from_str(value.keyexpr().as_str())
                .map_err(|_| CollectionError::CollectionTooSmall)?,
            parameters: value
                .parameters
                .map(heapless::String::from_str)
                .transpose()
                .map_err(|_| CollectionError::CollectionTooSmall)?,
            payload: value
                .payload
                .map(heapless::Vec::from_slice)
                .transpose()
                .map_err(|_| CollectionError::CollectionTooSmall)?,
            query_target: value.query_target,
            qos: value.qos,
            finalized: value.finalized,
        })
    }
}

#[cfg(feature = "alloc")]
pub struct AllocQueryableQuery<Config>
where
    Config: ZSessionConfig + 'static,
{
    session: &'static Session<'static, Config>,
    rid: u32,
    ke: alloc::string::String,
    parameters: Option<alloc::string::String>,
    payload: Option<alloc::vec::Vec<u8>>,
    query_target: QueryTarget,
    qos: QoS,
    finalized: bool,
}

#[cfg(feature = "alloc")]
impl<Config> AllocQueryableQuery<Config>
where
    Config: ZSessionConfig,
{
    pub fn keyexpr(&self) -> &keyexpr {
        keyexpr::from_str_unchecked(self.ke.as_str())
    }

    pub fn parameters(&self) -> Option<&str> {
        self.parameters.as_deref()
    }

    pub fn payload(&self) -> Option<&[u8]> {
        self.payload.as_deref()
    }

    pub fn query_target(&self) -> QueryTarget {
        self.query_target
    }

    pub fn qos(&self) -> QoS {
        self.qos
    }

    pub fn reply(
        &self,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        self.session.reply(self.rid, ke, payload)
    }

    pub fn err(
        &self,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        self.session.err(self.rid, ke, payload)
    }

    pub fn finalize(&mut self) -> core::result::Result<(), SessionError> {
        if !self.finalized {
            self.session.finalize(self.rid)?;
            self.finalized = true
        }

        Ok(())
    }
}

#[cfg(feature = "alloc")]
impl<'a, Config>
    TryFrom<(
        &QueryableQuery<'a, 'static, Config>,
        &'static Session<'static, Config>,
    )> for AllocQueryableQuery<Config>
where
    Config: ZSessionConfig,
{
    type Error = CollectionError;

    fn try_from(
        value: (
            &QueryableQuery<'a, 'static, Config>,
            &'static Session<'static, Config>,
        ),
    ) -> Result<Self, Self::Error> {
        let (value, session) = value;

        Ok(Self {
            session,
            rid: value.rid,
            ke: alloc::string::String::from(value.keyexpr().as_str()),
            parameters: value.parameters.map(alloc::string::String::from),
            payload: value.payload.map(alloc::vec::Vec::from),
            query_target: value.query_target,
            qos: value.qos,
            finalized: value.finalized,
        })
    }
}