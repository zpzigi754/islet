use super::vcpu::{Context, VCPU};
use super::vmem::IPATranslation;

use crate::error::{Error, ErrorKind};
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::mutex::Mutex;

extern crate alloc;

#[derive(Debug)]
pub struct VM<T: Context> {
    id: usize,
    pub state: State,
    pub vcpus: Vec<Arc<Mutex<VCPU<T>>>>,
    pub page_table: Arc<Mutex<Box<dyn IPATranslation>>>,
    // TODO: add pagetable
}

impl<T: Context + Default> VM<T> {
    pub fn new(
        id: usize,
        num_vcpu: usize,
        page_table: Arc<Mutex<Box<dyn IPATranslation>>>,
    ) -> Arc<Mutex<Self>> {
        Arc::<Mutex<Self>>::new_cyclic(|me| {
            let mut vcpus = Vec::with_capacity(num_vcpu);
            vcpus.resize_with(num_vcpu, move || VCPU::new(me.clone()));

            let vm = Mutex::new(Self {
                id: id,
                state: State::Init,
                vcpus: vcpus,
                page_table: page_table,
            });

            vm
        })
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn switch_to(&self, vcpu: usize) -> Result<(), Error> {
        self.vcpus
            .get(vcpu)
            .map(|vcpu| vcpu.lock().set_current())
            .ok_or(Error::new(ErrorKind::NotConnected))?;
        self.page_table.lock().set_mmu();

        Ok(())
    }
}

impl<T: Context> Drop for VM<T> {
    fn drop(&mut self) {
        //TODO unset pagetable
    }
}

#[derive(Debug)]
pub enum State {
    Init,
    Ready,
    Running,
    Destroy,
}