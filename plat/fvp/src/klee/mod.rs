use core::panic::PanicInfo;
use verification_annotations::prelude::*;

#[cfg(feature = "verifier-klee")]
#[no_mangle]
pub fn main() {
    let a = u32::abstract_value();
    let b = u32::abstract_value();
    verifier::assume(1 <= a && a <= 1000);
    verifier::assume(1 <= b && b <= 1000);
    let r = a * b;
    verifier::assert!(1 <= r && r < 1000000);
}

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}
