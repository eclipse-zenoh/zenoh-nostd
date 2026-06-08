use core::marker::PhantomData;

use dyn_utils::{DynObject, storage::Storage};
use heapless::FnvIndexMap;
use zenoh_proto::keyexpr;

use crate::api::arg::ZArg;

#[dyn_utils::dyn_trait(trait = ZDynCallback)]
#[dyn_trait(dyn_utils::dyn_object)]
pub trait ZCallback {
    type Arg: ZArg;

    fn call(&mut self, arg: <Self::Arg as ZArg>::Of<'_>);
}

pub type DynCallback<'a, Callback, Arg> = DynObject<dyn ZDynCallback<Arg = Arg> + 'a, Callback>;

pub trait ZCallbacks<'a, Arg>
where
    Arg: ZArg,
{
    type Callback: Storage;

    fn empty() -> Self;

    fn insert(
        &mut self,
        id: u32,
        ke: &'static keyexpr,
        timedout: Option<u64>,
        callback: DynCallback<'a, Self::Callback, Arg>,
    ) -> core::result::Result<(), zenoh_proto::CollectionError>;

    fn drop_timedout(&mut self);
    fn get(&mut self, id: u32) -> Option<&mut DynCallback<'a, Self::Callback, Arg>>;

    fn remove(&mut self, id: u32) -> core::result::Result<(), zenoh_proto::CollectionError>;

    fn set_counter(
        &mut self,
        id: u32,
        value: usize,
    ) -> core::result::Result<(), zenoh_proto::CollectionError>;

    fn decrease(&mut self, id: u32) -> bool;

    fn intersects<'r>(
        &'r mut self,
        ke: &keyexpr,
    ) -> impl Iterator<Item = &'r mut DynCallback<'a, Self::Callback, Arg>>
    where
        DynCallback<'a, Self::Callback, Arg>: 'r;

    fn for_each_keyexpr(&self, f: impl FnMut(u32, &'static keyexpr));
}

pub struct FixedCapacityCallbacks<'a, Arg: ZArg, const CAPACITY: usize, Callback: Storage> {
    keyexprs: FnvIndexMap<u32, &'static keyexpr, CAPACITY>,
    callbacks: FnvIndexMap<(u32, &'static keyexpr), DynCallback<'a, Callback, Arg>, CAPACITY>,
    timedouts: FnvIndexMap<u32, u64, CAPACITY>,
    counters: FnvIndexMap<u32, usize, CAPACITY>,
}

impl<'a, Arg: ZArg + 'a, const CAPACITY: usize, Callback: Storage> ZCallbacks<'a, Arg>
    for FixedCapacityCallbacks<'a, Arg, CAPACITY, Callback>
{
    type Callback = Callback;

    fn empty() -> Self {
        Self {
            keyexprs: FnvIndexMap::new(),
            callbacks: FnvIndexMap::new(),
            timedouts: FnvIndexMap::new(),
            counters: FnvIndexMap::new(),
        }
    }

    fn insert(
        &mut self,
        id: u32,
        ke: &'static keyexpr,
        timedout: Option<u64>,
        callback: DynCallback<'a, Callback, Arg>,
    ) -> core::result::Result<(), zenoh_proto::CollectionError> {
        if self.keyexprs.contains_key(&id) {
            return Err(zenoh_proto::CollectionError::KeyAlreadyExists);
        }

        if self.callbacks.contains_key(&(id, ke)) {
            return Err(zenoh_proto::CollectionError::KeyAlreadyExists);
        }

        if self.timedouts.contains_key(&id) {
            return Err(zenoh_proto::CollectionError::KeyAlreadyExists);
        }

        self.keyexprs
            .insert(id, ke)
            .map_err(|_| zenoh_proto::CollectionError::CollectionIsFull)?;

        self.callbacks
            .insert((id, ke), callback)
            .map_err(|_| zenoh_proto::CollectionError::CollectionIsFull)?;

        if let Some(timedout) = timedout {
            self.timedouts
                .insert(id, timedout)
                .map_err(|_| zenoh_proto::CollectionError::CollectionIsFull)?;
        }

        Ok(())
    }

    fn drop_timedout(&mut self) {
        self.timedouts.retain(|_id, _timedout| {
            // Timeout-based drop is disabled in sync mode.
            // The caller is responsible for managing timeouts.
            true
        });
    }

    fn remove(&mut self, id: u32) -> core::result::Result<(), zenoh_proto::CollectionError> {
        if let Some(ke) = self.keyexprs.remove(&id) {
            self.callbacks.remove(&(id, ke));
        }
        self.timedouts.remove(&id);
        self.counters.remove(&id);

        Ok(())
    }

    fn get(&mut self, id: u32) -> Option<&mut DynCallback<'a, Callback, Arg>> {
        let ke = self.keyexprs.get(&id)?;
        self.callbacks.get_mut(&(id, ke))
    }

    fn set_counter(
        &mut self,
        id: u32,
        value: usize,
    ) -> core::result::Result<(), zenoh_proto::CollectionError> {
        self.counters
            .insert(id, value)
            .map_err(|_| zenoh_proto::CollectionError::CollectionIsFull)
            .map(|_| ())
    }

    fn decrease(&mut self, id: u32) -> bool {
        if let Some(value) = self.counters.get_mut(&id) {
            if *value > 0 {
                *value -= 1;
            }

            *value == 0
        } else {
            false
        }
    }

    fn intersects<'r>(
        &'r mut self,
        ke: &keyexpr,
    ) -> impl Iterator<Item = &'r mut DynCallback<'a, Callback, Arg>>
    where
        DynCallback<'a, Callback, Arg>: 'r,
    {
        self.callbacks
            .iter_mut()
            .filter_map(move |((_, registered_ke), callback)| {
                if registered_ke.intersects(ke) {
                    Some(callback)
                } else {
                    None
                }
            })
    }

    fn for_each_keyexpr(&self, mut f: impl FnMut(u32, &'static keyexpr)) {
        for (id, ke) in self.keyexprs.iter() {
            f(*id, ke)
        }
    }
}

#[cfg(feature = "alloc")]
pub struct AllocCallbacks<'a, Arg: ZArg, Callback: Storage> {
    keyexprs: alloc::collections::BTreeMap<u32, &'static keyexpr>,
    callbacks:
        alloc::collections::BTreeMap<(u32, &'static keyexpr), DynCallback<'a, Callback, Arg>>,
    timedouts: alloc::collections::BTreeMap<u32, u64>,
    counters: alloc::collections::BTreeMap<u32, usize>,
}

#[cfg(feature = "alloc")]
impl<'a, Arg: ZArg + 'a, Callback: Storage> ZCallbacks<'a, Arg>
    for AllocCallbacks<'a, Arg, Callback>
{
    type Callback = Callback;

    fn empty() -> Self {
        Self {
            keyexprs: alloc::collections::BTreeMap::new(),
            callbacks: alloc::collections::BTreeMap::new(),
            timedouts: alloc::collections::BTreeMap::new(),
            counters: alloc::collections::BTreeMap::new(),
        }
    }

    fn insert(
        &mut self,
        id: u32,
        ke: &'static keyexpr,
        timedout: Option<u64>,
        callback: DynCallback<'a, Callback, Arg>,
    ) -> core::result::Result<(), zenoh_proto::CollectionError> {
        if self.keyexprs.contains_key(&id) {
            return Err(zenoh_proto::CollectionError::KeyAlreadyExists);
        }

        if self.callbacks.contains_key(&(id, ke)) {
            return Err(zenoh_proto::CollectionError::KeyAlreadyExists);
        }

        if self.timedouts.contains_key(&id) {
            return Err(zenoh_proto::CollectionError::KeyAlreadyExists);
        }

        self.keyexprs.insert(id, ke);

        self.callbacks.insert((id, ke), callback);

        if let Some(timedout) = timedout {
            self.timedouts.insert(id, timedout);
        }

        Ok(())
    }

    fn drop_timedout(&mut self) {
        self.timedouts.retain(|_id, _timedout| true);
    }

    fn remove(&mut self, id: u32) -> core::result::Result<(), zenoh_proto::CollectionError> {
        if let Some(ke) = self.keyexprs.remove(&id) {
            self.callbacks.remove(&(id, ke));
        }
        self.timedouts.remove(&id);
        self.counters.remove(&id);

        Ok(())
    }

    fn get(&mut self, id: u32) -> Option<&mut DynCallback<'a, Callback, Arg>> {
        let ke = self.keyexprs.get(&id)?;
        self.callbacks.get_mut(&(id, ke))
    }

    fn set_counter(
        &mut self,
        id: u32,
        value: usize,
    ) -> core::result::Result<(), zenoh_proto::CollectionError> {
        self.counters.insert(id, value);

        Ok(())
    }

    fn decrease(&mut self, id: u32) -> bool {
        if let Some(value) = self.counters.get_mut(&id) {
            if *value > 0 {
                *value -= 1;
            }

            *value == 0
        } else {
            false
        }
    }

    fn intersects<'r>(
        &'r mut self,
        ke: &keyexpr,
    ) -> impl Iterator<Item = &'r mut DynCallback<'a, Callback, Arg>>
    where
        DynCallback<'a, Callback, Arg>: 'r,
    {
        self.callbacks
            .iter_mut()
            .filter_map(move |((_, registered_ke), callback)| {
                if registered_ke.intersects(ke) {
                    Some(callback)
                } else {
                    None
                }
            })
    }

    fn for_each_keyexpr(&self, mut f: impl FnMut(u32, &'static keyexpr)) {
        for (id, ke) in self.keyexprs.iter() {
            f(*id, ke)
        }
    }
}

pub mod storage {
    #[cfg(feature = "alloc")]
    pub use dyn_utils::storage::Box;
    pub use dyn_utils::storage::{Raw, RawOrBox};
}

pub struct SyncCallback<Arg, F>(F, PhantomData<Arg>);

impl<Arg, F> SyncCallback<Arg, F> {
    pub fn new(f: F) -> Self {
        Self(f, PhantomData)
    }
}

impl<Arg, F> ZCallback for SyncCallback<Arg, F>
where
    Arg: ZArg,
    F: FnMut(Arg::Of<'_>),
{
    type Arg = Arg;

    fn call(&mut self, arg: <Self::Arg as ZArg>::Of<'_>) {
        (self.0)(arg)
    }
}

#[allow(dead_code)]
pub struct AsyncCallback<Arg, F>(F, PhantomData<Arg>);

#[allow(dead_code)]
impl<Arg, F> AsyncCallback<Arg, F> {
    pub fn new(f: F) -> Self {
        Self(f, PhantomData)
    }
}
impl<Arg, F> ZCallback for AsyncCallback<Arg, F>
where
    Arg: ZArg,
    F: FnMut(Arg::Of<'_>),
{
    type Arg = Arg;

    fn call(&mut self, arg: <Self::Arg as ZArg>::Of<'_>) {
        (self.0)(arg)
    }
}
