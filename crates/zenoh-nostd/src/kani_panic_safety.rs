use core::time::Duration;

/// Proof 1: core::time::Duration::as_micros() never panics.
///
/// as_micros() returns a u128 and is infallible for any Duration value.
/// The `as u64` truncation is safe (no panic path, only semantic truncation
/// for durations exceeding ~584k years which is irrelevant in practice).
#[cfg(kani)]
#[kani::proof]
fn duration_as_micros_never_panics() {
    let secs: u64 = kani::any();
    let nsecs: u32 = kani::any();
    kani::assume(nsecs < 1_000_000_000);
    let d = Duration::new(secs, nsecs);
    let _micros = d.as_micros() as u64;
}

/// Proof 2: core::time::Duration::from_micros never panics for capped u64.
///
/// `from_micros` has an internal overflow in `div_ceil` for values near
/// u64::MAX. We cap at `u64::MAX >> 1` (~292,471 years in microseconds)
/// which is far beyond any realistic lease/timeout value.
#[cfg(kani)]
#[kani::proof]
fn core_dur_from_micros_never_panics() {
    let micros: u64 = kani::any();
    let capped = micros.min(u64::MAX >> 1);
    let _dur = core::time::Duration::from_micros(capped);
}

/// Proof 3: Combined conversion pattern used in driver.rs and transport.rs.
///
/// This simulates the exact pattern for any valid `core::time::Duration`
/// with the same capping used in the actual code. This is the replacement
/// for the old `d.try_into().unwrap()` pattern that could panic on
/// attacker-controlled values.
#[cfg(kani)]
#[kani::proof]
fn duration_conversion_never_panics() {
    let secs: u64 = kani::any();
    let nsecs: u32 = kani::any();
    kani::assume(nsecs < 1_000_000_000);
    let d = Duration::new(secs, nsecs);
    let micros = (d.as_micros() as u64).min(u64::MAX >> 1);
    let _core_dur = core::time::Duration::from_micros(micros);
}

/// Proof 4: ResourcesInner::init() exhaustive match is correct.
///
/// This proves that after `init()` sets the enum to `Init`, the subsequent
/// exhaustive match always reaches the `Init` arm, regardless of the prior
/// variant and regardless of the value being stored. Both possible prior
/// states (Uninit and Init with any prior value) are checked. A simplified
/// model is used because the real `ResourcesInner` has complex generic
/// trait bounds.
///
/// This replaces the old `unreachable!()` wildcard with an explicit
/// exhaustive match, making new-variant additions a compile-time error.
#[cfg(kani)]
#[kani::proof]
fn resources_init_exhaustive_match_always_hits_init() {
    #[derive(Default)]
    enum Inner {
        #[default]
        Uninit,
        Init {
            value: u32,
        },
    }

    fn init(inner: &mut Inner, value: u32) -> &mut u32 {
        *inner = Inner::Init { value };
        match inner {
            Inner::Init { value } => value,
            Inner::Uninit => unreachable!(),
        }
    }

    let prior_value: u32 = kani::any();
    let new_value: u32 = kani::any();
    {
        let mut inner = Inner::Uninit;
        let result = init(&mut inner, new_value);
        assert_eq!(*result, new_value);
    }
    {
        let mut inner = Inner::Init { value: prior_value };
        let result = init(&mut inner, new_value);
        assert_eq!(*result, new_value);
    }
}
