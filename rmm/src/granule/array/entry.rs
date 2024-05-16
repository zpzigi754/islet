use crate::granule::array::GRANULE_STATUS_TABLE;
use crate::rmi::error::Error;

use super::{GranuleState, GRANULE_SIZE};
use spinning_top::{Spinlock, SpinlockGuard};
use vmsa::guard::Content;

#[cfg(not(kani))]
use crate::granule::{FVP_DRAM0_REGION, FVP_DRAM1_IDX, FVP_DRAM1_REGION};

// Safety: concurrency safety
//  - For a granule status table that manages granules, it doesn't use a big lock for efficiency.
//    So, we need to associate "lock" with each granule entry.

#[cfg(not(kani))]
pub struct Granule {
    /// granule state
    state: u8,
}
#[cfg(kani)]
// DIFF: `gpt` ghost field is added to track GPT entry's status
pub struct Granule {
    /// granule state
    state: u8,
    /// granule protection table (ghost field)
    pub gpt: GranuleGpt,
    /// granule index (ghost field)
    pub idx: usize,
}

#[cfg(kani)]
#[derive(Copy, Clone, PartialEq, kani::Arbitrary)]
pub enum GranuleGpt {
    GPT_NS,
    GPT_OTHER,
    GPT_REALM,
}

impl Granule {
    #[cfg(not(kani))]
    fn new() -> Self {
        let state = GranuleState::Undelegated;
        Granule { state }
    }
    #[cfg(kani)]
    // DIFF: `state` and `gpt` are filled with non-deterministic values
    //       `idx` is passed
    fn new(idx: usize) -> Self {
        let state = kani::any();
        kani::assume(state >= GranuleState::Undelegated && state <= GranuleState::RTT);
        let gpt = {
            if state != GranuleState::Undelegated {
                match state {
                    GranuleState::RD => {
                        // index_to_addr()
                        use crate::granule::GRANULE_REGION;
                        use crate::realm::rd::Rd;
                        let addr = GRANULE_REGION.as_ptr() as usize + (idx * GRANULE_SIZE);
                        // content()
                        let rd = unsafe { &*(addr as *const Rd) };
                        kani::assume(rd.is_valid());
                    }
                    _ => {}
                };
                GranuleGpt::GPT_REALM
            } else {
                let gpt = kani::any();
                kani::assume(gpt != GranuleGpt::GPT_REALM);
                gpt
            }
        };
        Granule { state, gpt, idx }
    }

    #[cfg(kani)]
    pub fn set_gpt(&mut self, gpt: GranuleGpt) {
        self.gpt = gpt;
    }

    #[cfg(kani)]
    pub fn is_valid(&self) -> bool {
        self.state >= GranuleState::Undelegated &&
        self.state <= GranuleState::RTT &&
        // XXX: the below condition holds from beta0 to eac4
        if self.state != GranuleState::Undelegated {
            let is_inner_valid = match self.state {
                GranuleState::RD => {
                    use crate::realm::rd::Rd;
                    let rd = self.content::<Rd>();
                    rd.is_valid()
                },
                _ => true,
            };
            is_inner_valid &&
            self.gpt == GranuleGpt::GPT_REALM
        } else {
            self.gpt != GranuleGpt::GPT_REALM
        }
    }

    pub fn state(&self) -> u8 {
        self.state
    }

    pub fn set_state(&mut self, state: u8) -> Result<(), Error> {
        let prev = self.state;
        if (prev == GranuleState::Delegated && state == GranuleState::Undelegated)
            || (state == GranuleState::Delegated)
        {
            self.zeroize();
        }
        self.state = state;
        Ok(())
    }

    pub fn content_mut<T: Content>(&mut self) -> &mut T {
        let addr = self.index_to_addr();
        unsafe { &mut *(addr as *mut T) }
    }

    pub fn content<T: Content>(&self) -> &T {
        let addr = self.index_to_addr();
        unsafe { &*(addr as *const T) }
    }

    #[cfg(not(kani))]
    fn index(&self) -> usize {
        let entry_size = core::mem::size_of::<Entry>();
        let granule_size = core::mem::size_of::<Granule>();
        //  XXX: is there a clever way of getting the Entry from Granule (e.g., container_of())?
        //  [        Entry        ]
        //  [  offset ] [ Granule ]
        let granule_offset = entry_size - granule_size;
        let granule_addr = self as *const Granule as usize;
        let entry_addr = granule_addr - granule_offset;
        let gst = &GRANULE_STATUS_TABLE;
        let table_base = gst.entries.as_ptr() as usize;
        (entry_addr - table_base) / core::mem::size_of::<Entry>()
    }
    #[cfg(kani)]
    // DIFF: the inner `idx` field is used directly without calculation
    fn index(&self) -> usize {
        self.idx
    }

    #[cfg(not(kani))]
    fn index_to_addr(&self) -> usize {
        let idx = self.index();
        if idx < FVP_DRAM1_IDX {
            return FVP_DRAM0_REGION.start + (idx * GRANULE_SIZE);
        }
        FVP_DRAM1_REGION.start + ((idx - FVP_DRAM1_IDX) * GRANULE_SIZE)
    }
    #[cfg(kani)]
    // DIFF: calculate addr using GRANULE_REGION
    pub fn index_to_addr(&self) -> usize {
        use crate::granule::GRANULE_REGION;
        let idx = self.index();
        assert!(idx >= 0 && idx < 8);
        return GRANULE_REGION.as_ptr() as usize + (idx * GRANULE_SIZE);
    }

    #[cfg(not(kani))]
    fn zeroize(&mut self) {
        let addr = self.index_to_addr();
        unsafe {
            core::ptr::write_bytes(addr as *mut u8, 0x0, GRANULE_SIZE);
        }
    }
    #[cfg(kani)]
    // DIFF: assertion is added to reduce the proof burden
    //       `write_bytes()` uses a small count value
    fn zeroize(&mut self) {
        let addr = self.index_to_addr();
        let g_start = crate::granule::array::GRANULE_REGION.as_ptr() as usize;
        let g_end = g_start + crate::granule::array::GRANULE_MEM_SIZE;
        assert!(addr >= g_start && addr < g_end);

        unsafe {
            core::ptr::write_bytes(addr as *mut u8, 0x0, 8);
            assert!(*(addr as *const u8) == 0);
        }
    }
}

pub struct Entry(Spinlock<Granule>);
impl Entry {
    #[cfg(not(kani))]
    pub fn new() -> Self {
        Self(Spinlock::new(Granule::new()))
    }
    #[cfg(kani)]
    // DIFF: assertion is added to reduce the proof burden
    //       `idx` is passed
    pub fn new(idx: usize) -> Self {
        let granule = Granule::new(idx);
        assert!(granule.is_valid());
        Self(Spinlock::new(granule))
    }

    pub fn lock(&self) -> Result<SpinlockGuard<'_, Granule>, Error> {
        let granule = self.0.lock();
        Ok(granule)
    }
}
