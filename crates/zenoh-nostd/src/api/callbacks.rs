use core::marker::PhantomData;

use dyn_utils::{DynObject, storage::Storage};
use embassy_time::Instant;
use heapless::FnvIndexMap;
use zenoh_proto::keyexpr;

use crate::api::arg::ZArg;

#[dyn_utils::dyn_trait(trait = ZDynCallback)]
#[dyn_trait(dyn_utils::dyn_object)]
pub trait ZCallback {
    type Arg: ZArg;

    #[dyn_trait(maybe_sync)]
    fn call(&mut self, arg: <Self::Arg as ZArg>::Of<'_>) -> impl Future<Output = ()>;
}
pub type DynCallback<'a, Callback, Future, Arg> =
    DynObject<dyn ZDynCallback<Future, Arg = Arg> + 'a, Callback>;
pub trait ZCallbacks<'a, Arg>
where
    Arg: ZArg,
{
    type Callback: Storage;
    type Future: Storage;
    fn empty() -> Self;
    fn insert(
        &mut self,
        id: u32,
        ke: &keyexpr,
        timedout: Option<Instant>,
        callback: DynCallback<'a, Self::Callback, Self::Future, Arg>,
    ) -> core::result::Result<(), zenoh_proto::CollectionError>;
    fn drop_timedout(&mut self);
    fn get(&mut self, id: u32) -> Option<&mut DynCallback<'a, Self::Callback, Self::Future, Arg>>;
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
    ) -> impl Iterator<Item = &'r mut DynCallback<'a, Self::Callback, Self::Future, Arg>>
    where
        DynCallback<'a, Self::Callback, Self::Future, Arg>: 'r;
}

type Entry<'a, Key, Callback, Future, Arg> =
    (Key, DynCallback<'a, Callback, Future, Arg>, Option<Instant>);
pub struct FixedCapacityCallbacks<
    'a,
    Arg: ZArg,
    const CAPACITY: usize,
    Callback: Storage,
    Future: Storage,
    const MAX_KEYEXPR: usize = 128,
> {
    entries:
        FnvIndexMap<u32, Entry<'a, heapless::String<MAX_KEYEXPR>, Callback, Future, Arg>, CAPACITY>,
    counters: FnvIndexMap<u32, usize, CAPACITY>,
}

impl<
    'a,
    Arg: ZArg + 'a,
    const CAPACITY: usize,
    Callback: Storage,
    Future: Storage,
    const MAX_KEYEXPR: usize,
> ZCallbacks<'a, Arg> for FixedCapacityCallbacks<'a, Arg, CAPACITY, Callback, Future, MAX_KEYEXPR>
{
    type Callback = Callback;
    type Future = Future;
    fn empty() -> Self {
        Self {
            entries: FnvIndexMap::new(),
            counters: FnvIndexMap::new(),
        }
    }
    fn insert(
        &mut self,
        id: u32,
        ke: &keyexpr,
        timedout: Option<Instant>,
        callback: DynCallback<'a, Callback, Future, Arg>,
    ) -> Result<(), zenoh_proto::CollectionError> {
        if self.entries.contains_key(&id) {
            return Err(zenoh_proto::CollectionError::KeyAlreadyExists);
        }
        let key = heapless::String::try_from(ke.as_str())
            .map_err(|_| zenoh_proto::CollectionError::CollectionTooSmall)?;
        self.entries
            .insert(id, (key, callback, timedout))
            .map_err(|_| zenoh_proto::CollectionError::CollectionIsFull)?;
        Ok(())
    }
    fn drop_timedout(&mut self) {
        self.entries.retain(|id, (_, _, timeout)| {
            if timeout.is_some_and(|t| Instant::now() >= t) {
                self.counters.remove(id);
                false
            } else {
                true
            }
        });
    }
    fn remove(&mut self, id: u32) -> Result<(), zenoh_proto::CollectionError> {
        self.entries.remove(&id);
        self.counters.remove(&id);
        Ok(())
    }
    fn get(&mut self, id: u32) -> Option<&mut DynCallback<'a, Callback, Future, Arg>> {
        self.entries.get_mut(&id).map(|(_, callback, _)| callback)
    }
    fn set_counter(&mut self, id: u32, value: usize) -> Result<(), zenoh_proto::CollectionError> {
        self.counters
            .insert(id, value)
            .map_err(|_| zenoh_proto::CollectionError::CollectionIsFull)?;
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
    ) -> impl Iterator<Item = &'r mut DynCallback<'a, Callback, Future, Arg>>
    where
        DynCallback<'a, Callback, Future, Arg>: 'r,
    {
        self.entries
            .iter_mut()
            .filter_map(move |(_, (registered, callback, _))| {
                keyexpr::from_str_unchecked(registered)
                    .intersects(ke)
                    .then_some(callback)
            })
    }
}

#[cfg(feature = "alloc")]
pub struct AllocCallbacks<'a, Arg: ZArg, Callback: Storage, Future: Storage> {
    entries:
        alloc::collections::BTreeMap<u32, Entry<'a, alloc::string::String, Callback, Future, Arg>>,
    counters: alloc::collections::BTreeMap<u32, usize>,
}
#[cfg(feature = "alloc")]
impl<'a, Arg: ZArg + 'a, Callback: Storage, Future: Storage> ZCallbacks<'a, Arg>
    for AllocCallbacks<'a, Arg, Callback, Future>
{
    type Callback = Callback;
    type Future = Future;
    fn empty() -> Self {
        Self {
            entries: alloc::collections::BTreeMap::new(),
            counters: alloc::collections::BTreeMap::new(),
        }
    }
    fn insert(
        &mut self,
        id: u32,
        ke: &keyexpr,
        timedout: Option<Instant>,
        callback: DynCallback<'a, Callback, Future, Arg>,
    ) -> Result<(), zenoh_proto::CollectionError> {
        if self.entries.contains_key(&id) {
            return Err(zenoh_proto::CollectionError::KeyAlreadyExists);
        }
        self.entries.insert(
            id,
            (alloc::string::String::from(ke.as_str()), callback, timedout),
        );
        Ok(())
    }
    fn drop_timedout(&mut self) {
        self.entries.retain(|id, (_, _, timeout)| {
            if timeout.is_some_and(|t| Instant::now() >= t) {
                self.counters.remove(id);
                false
            } else {
                true
            }
        });
    }
    fn remove(&mut self, id: u32) -> Result<(), zenoh_proto::CollectionError> {
        self.entries.remove(&id);
        self.counters.remove(&id);
        Ok(())
    }
    fn get(&mut self, id: u32) -> Option<&mut DynCallback<'a, Callback, Future, Arg>> {
        self.entries.get_mut(&id).map(|(_, callback, _)| callback)
    }
    fn set_counter(&mut self, id: u32, value: usize) -> Result<(), zenoh_proto::CollectionError> {
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
    ) -> impl Iterator<Item = &'r mut DynCallback<'a, Callback, Future, Arg>>
    where
        DynCallback<'a, Callback, Future, Arg>: 'r,
    {
        self.entries
            .iter_mut()
            .filter_map(move |(_, (registered, callback, _))| {
                keyexpr::from_str_unchecked(registered)
                    .intersects(ke)
                    .then_some(callback)
            })
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

    #[dyn_utils::sync]
    async fn call(&mut self, arg: <Self::Arg as ZArg>::Of<'_>) {
        (self.0)(arg)
    }
}
pub struct AsyncCallback<Arg, F>(F, PhantomData<Arg>);

impl<Arg, F> AsyncCallback<Arg, F> {
    pub fn new(f: F) -> Self {
        Self(f, PhantomData)
    }
}
impl<Arg, F> ZCallback for AsyncCallback<Arg, F>
where
    Arg: ZArg,
    F: AsyncFnMut(Arg::Of<'_>),
{
    type Arg = Arg;

    fn call(&mut self, arg: <Self::Arg as ZArg>::Of<'_>) -> impl Future<Output = ()> {
        (self.0)(arg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::arg::SampleRef;
    use dyn_utils::storage::Raw;
    type Registry = FixedCapacityCallbacks<'static, SampleRef, 2, Raw<16>, Raw<32>, 32>;
    fn callback() -> DynCallback<'static, Raw<16>, Raw<32>, SampleRef> {
        DynObject::new(SyncCallback::new(|_: &crate::session::Sample<'_>| {}))
    }
    #[test]
    fn registration_owns_runtime_key_and_slots_are_reusable() {
        let mut registry = Registry::empty();
        for id in 0..1000 {
            let mut input = heapless::String::<32>::try_from("sensors/room/**").unwrap();
            registry
                .insert(id, keyexpr::new(&input).unwrap(), None, callback())
                .unwrap();
            input.clear();
            input.push_str("another/key").unwrap();
            assert_eq!(
                registry
                    .intersects(keyexpr::new("sensors/room/temp").unwrap())
                    .count(),
                1
            );
            registry.remove(id).unwrap();
            assert!(registry.get(id).is_none());
        }
    }
    #[test]
    fn owned_key_insertion_preserves_existing_registrations() {
        let mut registry = Registry::empty();
        let long = keyexpr::new("this/key/is/too/long/for/the/registry/storage").unwrap();
        assert!(registry.insert(1, long, None, callback()).is_err());
        assert!(registry.get(1).is_none());
        for id in 0..2 {
            registry
                .insert(id, keyexpr::new("a").unwrap(), None, callback())
                .unwrap();
        }
        assert!(
            registry
                .insert(2, keyexpr::new("b").unwrap(), None, callback())
                .is_err()
        );
        assert!(
            registry
                .insert(1, keyexpr::new("b").unwrap(), None, callback())
                .is_err()
        );
        registry.remove(0).unwrap();
        registry.remove(1).unwrap();
        assert!(registry.get(0).is_none());
        assert!(registry.get(1).is_none());
        registry
            .insert(2, keyexpr::new("reused/key").unwrap(), None, callback())
            .unwrap();
    }
    #[test]
    fn expired_get_releases_key_and_callback() {
        let mut registry = Registry::empty();
        registry
            .insert(
                1,
                keyexpr::new("a").unwrap(),
                Some(Instant::from_ticks(0)),
                callback(),
            )
            .unwrap();
        registry.drop_timedout();
        assert!(registry.get(1).is_none());
    }
}
