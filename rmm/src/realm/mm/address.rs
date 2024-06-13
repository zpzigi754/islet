use core::fmt;
use core::ops::{Add, AddAssign, BitAnd, BitOr, Sub, SubAssign};

use vmsa_no_level::address::Address;
use vmsa_no_level::impl_addr;

pub use vmsa_no_level::address::PhysAddr;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GuestPhysAddr(usize);

impl_addr!(GuestPhysAddr);
