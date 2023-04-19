#[cfg(test)]
mod allocation_adapter_unit_test {
    use crate::adapters::{
        allocation_adapter::AllocationAdapter, allocation_adapter_mock::AllocationAdapterMock,
    };
    use ethereum_types::Address;
    use rstest::*;
    use std::{collections::HashSet, str::FromStr};

    #[rstest]
    fn allocation_adapter_test() {
        let allocation_ids = HashSet::from([
            Address::from_str("0xabababababababababababababababababababab").unwrap(),
            Address::from_str("0xbabababababababababababababababababababa").unwrap(),
            Address::from_str("0xdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdf").unwrap(),
        ]);
        let allocation_adapter = AllocationAdapterMock::new(allocation_ids);

        // Check for existing allocation id
        assert!(allocation_adapter.is_valid_allocation_id(
            Address::from_str("0xdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdf").unwrap()
        ));
        // Check for non-existing allocation id
        assert!(!allocation_adapter.is_valid_allocation_id(
            Address::from_str("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap()
        ));
    }
}
