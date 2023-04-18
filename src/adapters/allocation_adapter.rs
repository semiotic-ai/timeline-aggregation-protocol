use ethereum_types::Address;

pub trait AllocationAdapter {
    fn is_valid_allocation_id(&self, id: Address) -> bool;
}
