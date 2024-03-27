use core::panic::PanicInfo;
use verification_annotations::prelude::*;
use islet_rmm::monitor::Monitor;

extern "C" {
    fn klee_trace_ret();
    fn klee_trace_param_i32(param: i32, name: *const u8);
}

pub fn example(arg0: i32, arg1: i32) -> i32 {
    unsafe { klee_trace_ret(); }
    unsafe { klee_trace_param_i32(arg0, b"arg0\0".as_ptr()); }
    unsafe { klee_trace_param_i32(arg1, b"arg1\0".as_ptr()); }

    if (arg0 == 0) {
        return 1;
    }

    return 0;
}

#[cfg(feature = "verifier-klee")]
#[no_mangle]
pub fn main() {
    unsafe { islet_rmm::allocator::init(); }

    let x = i32::abstract_value();
    let y = 256;
    let ret = example(x, 256);

    Monitor::new().run();
/*
    let a = u32::abstract_value();
    let b = u32::abstract_value();
    verifier::assume(1 <= a && a <= 1000);
    verifier::assume(1 <= b && b <= 1000);
    let r = a * b;
    verifier::assert!(1 <= r && r < 1000000);*/
}

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    verifier::assert!(false);
    loop {}
}
