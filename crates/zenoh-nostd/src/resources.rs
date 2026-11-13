use core::hint::unreachable_unchecked;

use crate::{config::ZSessionConfig, io::transport::TransportLink, platform::ZLinkManager};

/// Caller-owned storage used by [`crate::session::SessionBuilder`].
///
/// Keeps the connected transport alive while the session borrows it. The builder
/// retains the resource lifetime for that borrow. For a shorter, reconnectable
/// scope, open a local transport through [`TransportLinkManager`](crate::session::TransportLinkManager)
/// and pass it to [`Session::new`](crate::session::Session::new) instead.
/// Static builders use one-shot storage; do not reset an occupied static cell.
pub struct Resources<'res, Config>
where
    Config: ZSessionConfig,
{
    pub(crate) inner: ResourcesInner<'res, Config>,
}

impl<'res, Config> Default for Resources<'res, Config>
where
    Config: ZSessionConfig,
{
    fn default() -> Self {
        Self {
            inner: ResourcesInner::default(),
        }
    }
}

type Link<'res, Config> = <<Config as ZSessionConfig>::LinkManager as ZLinkManager>::Link<'res>;

#[derive(Default)]
pub enum ResourcesInner<'res, Config>
where
    Config: ZSessionConfig + 'res,
{
    #[default]
    Uninit,
    Init {
        transport: TransportLink<Link<'res, Config>, Config::Buff>,
    },
}

impl<'res, Config> Resources<'res, Config>
where
    Config: ZSessionConfig,
{
    pub fn init(
        &mut self,
        transport: TransportLink<Link<'res, Config>, Config::Buff>,
    ) -> &mut TransportLink<Link<'res, Config>, Config::Buff> {
        self.inner = ResourcesInner::Init { transport };

        match &mut self.inner {
            ResourcesInner::Init { transport } => transport,
            _ => unsafe { unreachable_unchecked() },
        }
    }
}
