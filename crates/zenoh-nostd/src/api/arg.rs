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
pub struct QueryableQueryRef<'s, 'res, Config>(PhantomData<(&'s (), &'res Config)>);

impl ZArg for GetResponseRef {
    type Of<'a> = &'a GetResponse<'a>;
}

impl ZArg for SampleRef {
    type Of<'a> = &'a Sample<'a>;
}

impl<'s, 'res, Config> ZArg for QueryableQueryRef<'s, 'res, Config>
where
    Config: ZSessionConfig + 's,
    'res: 's,
{
    type Of<'a>
        = &'a QueryableQuery<'a, 's, 'res, Config>
    where
        Self: 'a;
}
