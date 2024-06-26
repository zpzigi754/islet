#![no_std]
#![no_main]
#![feature(asm_const)]
#![feature(naked_functions)]
#![warn(rust_2018_idioms)]
#![deny(warnings)]

#[macro_use]
extern crate log;

mod entry;

use aarch64_cpu::registers::*;
use islet_rmm::allocator;
use islet_rmm::cpu;

#[no_mangle]
pub unsafe fn main() -> ! {
    info!(
        "booted on core {:2} with EL{}!",
        cpu::get_cpu_id(),
        CurrentEL.read(CurrentEL::EL) as u8
    );

    islet_rmm::start(cpu::get_cpu_id());

    panic!("failed to run the mainloop");
}
