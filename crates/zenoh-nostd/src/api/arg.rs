use core::marker::PhantomData;

use crate::{api::query::QueryableQuery, config::ZSessionConfig};

use super::{response::*, sample::*};

pub trait ZArg {
    type Of<'a>
    where
        Self: 'a;
}

pub struct GetResponseRef;
pub struct SampleRef;
pub struct QueryableQueryRef<'transport, 'resources: 'transport, Config>(
    PhantomData<(&'transport (), &'resources Config)>,
);

impl ZArg for GetResponseRef {
    type Of<'a> = &'a GetResponse<'a>;
}

impl ZArg for SampleRef {
    type Of<'a> = &'a Sample<'a>;
}

impl<'transport, 'resources: 'transport, Config> ZArg
    for QueryableQueryRef<'transport, 'resources, Config>
where
    Config: ZSessionConfig,
{
    type Of<'a>
        = &'a QueryableQuery<'a, 'transport, 'resources, Config>
    where
        Self: 'a;
}
