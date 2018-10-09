use agent::chain_header::ChainHeader;
use holochain_core_types::{cas::storage::ContentAddressableStorage, entry_type::EntryType};

#[derive(Debug, PartialEq, Clone)]
pub struct ChainStore<CAS>
where
    CAS: ContentAddressableStorage + Sized + Clone + PartialEq,
{
    // Storages holding local shard data
    content_storage: CAS,
}

impl<CAS> ChainStore<CAS>
where
    CAS: ContentAddressableStorage + Sized + Clone + PartialEq,
{
    pub fn new(content_storage: CAS) -> Self {
        ChainStore { content_storage }
    }

    pub fn content_storage(&self) -> CAS {
        self.content_storage.clone()
    }

    pub fn iter(&self, start_chain_header: &Option<ChainHeader>) -> ChainStoreIterator<CAS> {
        ChainStoreIterator::new(self.content_storage.clone(), start_chain_header.clone())
    }

    pub fn iter_type(
        &self,
        start_chain_header: &Option<ChainHeader>,
        entry_type: &EntryType,
    ) -> ChainStoreTypeIterator<CAS> {
        ChainStoreTypeIterator::new(
            self.content_storage.clone(),
            self.iter(start_chain_header)
                .find(|chain_header| chain_header.entry_type() == entry_type),
        )
    }
}

pub struct ChainStoreIterator<CAS>
where
    CAS: ContentAddressableStorage + Sized + Clone + PartialEq,
{
    content_storage: CAS,
    current: Option<ChainHeader>,
}

impl<CAS> ChainStoreIterator<CAS>
where
    CAS: ContentAddressableStorage + Sized + Clone + PartialEq,
{
    #[allow(unknown_lints)]
    #[allow(needless_pass_by_value)]
    pub fn new(content_storage: CAS, current: Option<ChainHeader>) -> ChainStoreIterator<CAS> {
        ChainStoreIterator {
            content_storage,
            current,
        }
    }
}

impl<CAS> Iterator for ChainStoreIterator<CAS>
where
    CAS: ContentAddressableStorage + Sized + Clone + PartialEq,
{
    type Item = ChainHeader;

    /// May panic if there is an underlying error in the table
    fn next(&mut self) -> Option<ChainHeader> {
        let previous = self.current.take();

        self.current = previous
            .as_ref()
            .and_then(|chain_header| chain_header.link())
            .as_ref()
            // @TODO should this panic?
            // @see https://github.com/holochain/holochain-rust/issues/146
            .and_then(|linked_chain_header_address| {
                self.content_storage.fetch(linked_chain_header_address).expect("failed to fetch from CAS")
            });
        previous
    }
}

pub struct ChainStoreTypeIterator<CAS>
where
    CAS: ContentAddressableStorage + Sized + Clone + PartialEq,
{
    content_storage: CAS,
    current: Option<ChainHeader>,
}

impl<CAS> ChainStoreTypeIterator<CAS>
where
    CAS: ContentAddressableStorage + Sized + Clone + PartialEq,
{
    #[allow(unknown_lints)]
    #[allow(needless_pass_by_value)]
    pub fn new(content_storage: CAS, current: Option<ChainHeader>) -> ChainStoreTypeIterator<CAS> {
        ChainStoreTypeIterator {
            content_storage,
            current,
        }
    }
}

impl<CAS> Iterator for ChainStoreTypeIterator<CAS>
where
    CAS: ContentAddressableStorage + Sized + Clone + PartialEq,
{
    type Item = ChainHeader;

    /// May panic if there is an underlying error in the table
    fn next(&mut self) -> Option<ChainHeader> {
        let previous = self.current.take();

        self.current = previous
            .as_ref()
            .and_then(|chain_header| chain_header.link_same_type())
            .as_ref()
            // @TODO should this panic?
            // @see https://github.com/holochain/holochain-rust/issues/146
            .and_then(|linked_chain_header_address| {
                self.content_storage.fetch(linked_chain_header_address).expect("failed to fetch from CAS")
            });
        previous
    }
}

#[cfg(test)]
pub mod tests {

    use agent::{
        chain_header::{tests::test_chain_header, ChainHeader},
        chain_store::ChainStore,
    };
    use holochain_cas_implementations::cas::memory::MemoryStorage;
    use holochain_core_types::{
        cas::{content::AddressableContent, storage::ContentAddressableStorage},
        entry::{test_entry, test_entry_type, test_entry_type_a, test_entry_type_b},
    };

    pub fn test_chain_store() -> ChainStore<MemoryStorage> {
        ChainStore::new(MemoryStorage::new().expect("could not create new chain store"))
    }

    #[test]
    /// show Iterator implementation for chain store
    fn iterator_test() {
        let chain_store = test_chain_store();

        let chain_header_a = test_chain_header();
        let chain_header_b = ChainHeader::new(
            &test_entry_type(),
            &String::new(),
            Some(chain_header_a.address()),
            &test_entry().address(),
            &String::new(),
            None,
        );

        chain_store
            .content_storage()
            .add(&chain_header_a)
            .expect("could not add header to cas");
        chain_store
            .content_storage()
            .add(&chain_header_b)
            .expect("could not add header to cas");

        let expected = vec![chain_header_b.clone(), chain_header_a.clone()];
        let mut found = vec![];
        for chain_header in chain_store.iter(&Some(chain_header_b)) {
            found.push(chain_header);
        }
        assert_eq!(expected, found);

        let expected = vec![chain_header_a.clone()];
        let mut found = vec![];
        for chain_header in chain_store.iter(&Some(chain_header_a)) {
            found.push(chain_header);
        }
        assert_eq!(expected, found);
    }

    #[test]
    /// show entry typed Iterator implementation for chain store
    fn type_iterator_test() {
        let chain_store = test_chain_store();

        let chain_header_a = test_chain_header();
        // b has a different type to a
        let chain_header_b = ChainHeader::new(
            &test_entry_type_b(),
            &String::new(),
            Some(chain_header_a.address()),
            &test_entry().address(),
            &String::new(),
            None,
        );
        // c has same type as a
        let chain_header_c = ChainHeader::new(
            &test_entry_type_a(),
            &String::new(),
            Some(chain_header_b.address()),
            &test_entry().address(),
            &String::new(),
            Some(chain_header_a.address()),
        );

        for chain_header in vec![&chain_header_a, &chain_header_b, &chain_header_c] {
            chain_store
                .content_storage()
                .add(chain_header)
                .expect("could not add header to cas");
        }

        let expected = vec![chain_header_c.clone(), chain_header_a.clone()];
        let mut found = vec![];
        for chain_header in
            chain_store.iter_type(&Some(chain_header_c.clone()), &chain_header_c.entry_type())
        {
            found.push(chain_header);
        }
        assert_eq!(expected, found);

        let expected = vec![chain_header_a.clone()];
        let mut found = vec![];
        for chain_header in
            chain_store.iter_type(&Some(chain_header_b.clone()), &chain_header_c.entry_type())
        {
            found.push(chain_header);
        }
        assert_eq!(expected, found);

        let expected = vec![chain_header_b.clone()];
        let mut found = vec![];
        for chain_header in
            chain_store.iter_type(&Some(chain_header_c.clone()), &chain_header_b.entry_type())
        {
            found.push(chain_header);
        }
        assert_eq!(expected, found);

        let expected = vec![chain_header_b.clone()];
        let mut found = vec![];
        for chain_header in
            chain_store.iter_type(&Some(chain_header_b.clone()), &chain_header_b.entry_type())
        {
            found.push(chain_header);
        }
        assert_eq!(expected, found);
    }
}
