pub const NUM_OF_CPU: usize = 8;
pub const NUM_OF_CLUSTER: usize = 2;
pub const NUM_OF_CPU_PER_CLUSTER: usize = NUM_OF_CPU / NUM_OF_CLUSTER;

pub const PAGE_BITS: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_BITS; // 4KiB
pub const LARGE_PAGE_SIZE: usize = 1024 * 1024 * 2; // 2MiB
pub const HUGE_PAGE_SIZE: usize = 1024 * 1024 * 1024; // 1GiB

pub const RMM_STACK_SIZE: usize = 1024 * 1024;
pub const RMM_HEAP_SIZE: usize = 16 * 1024 * 1024;

pub const VM_STACK_SIZE: usize = 1 << 15;
pub const STACK_ALIGN: usize = 16;

pub const GRANULE_SIZE: usize = 4096;
pub const GRANULE_SHIFT: usize = 12;
pub const GRANULE_MASK: usize = !((1 << GRANULE_SHIFT) - 1);
