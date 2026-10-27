// One target application, with its frontend selected at compile time.
#[cfg(feature = "cli")]
include!("console.rs");
#[cfg(not(feature = "cli"))]
include!("headless.rs");
