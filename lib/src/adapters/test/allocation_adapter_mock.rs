use crate::adapters::allocation_adapter::AllocationAdapter;
use ethereum_types::Address;
use std::collections::HashSet;

pub struct AllocationAdapterMock {
    allocation_ids: HashSet<Address>,
}

impl AllocationAdapterMock {
    pub fn new(allocation_ids: HashSet<Address>) -> Self {
        AllocationAdapterMock { allocation_ids }
    }

    pub fn add_allocation(&mut self, allocation_id: Address) -> bool {
        self.allocation_ids.insert(allocation_id)
    }

    pub fn remove_allocation(&mut self, allocation_id: Address) -> bool {
        self.allocation_ids.remove(&allocation_id)
    }
}

impl AllocationAdapter for AllocationAdapterMock {
    fn is_valid_allocation_id(&self, allocation_id: Address) -> bool {
        self.allocation_ids.contains(&allocation_id)
    }
}
