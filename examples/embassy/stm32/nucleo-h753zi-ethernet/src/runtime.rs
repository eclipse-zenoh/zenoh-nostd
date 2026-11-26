use core::{cell::RefCell, num::NonZeroU32};
use critical_section::Mutex;
use defmt_rtt as _;
use embassy_stm32::{peripherals, rng::Rng};
use panic_probe as _;

static RNG: Mutex<RefCell<Option<Rng<'static, peripherals::RNG>>>> = Mutex::new(RefCell::new(None));

pub fn init_rng(mut rng: Rng<'static, peripherals::RNG>) -> u64 {
    let seed = rng.next_u64();
    critical_section::with(|cs| RNG.borrow(cs).replace(Some(rng)));
    seed
}

getrandom::register_custom_getrandom!(getrandom_custom);
fn getrandom_custom(bytes: &mut [u8]) -> Result<(), getrandom::Error> {
    critical_section::with(|cs| {
        let mut rng = RNG.borrow(cs).borrow_mut();
        let rng = rng.as_mut().ok_or_else(|| {
            getrandom::Error::from(NonZeroU32::new(getrandom::Error::CUSTOM_START).unwrap())
        })?;
        rng.fill_bytes(bytes);
        Ok(())
    })
}

#[cfg(feature = "alloc")]
#[global_allocator]
static HEAP: embedded_alloc::LlffHeap = embedded_alloc::LlffHeap::empty();

pub fn init_heap() {
    #[cfg(feature = "alloc")]
    {
        const SIZE: usize = 64 * 1024;
        static mut MEMORY: [core::mem::MaybeUninit<u8>; SIZE] =
            [core::mem::MaybeUninit::uninit(); SIZE];
        // SAFETY: board initialization runs once before any allocating API; the
        // static region belongs exclusively to this allocator for the program's lifetime.
        unsafe {
            HEAP.init(core::ptr::addr_of_mut!(MEMORY) as usize, SIZE);
        }
    }
}
