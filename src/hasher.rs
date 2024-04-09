use std::collections::{HashMap, HashSet, BinaryHeap};
use std::hash::{Hasher, BuildHasher};
use seahash::SeaHasher;

pub struct SeaHash;

pub type SeaHashKey = [u8; 12];
pub type SeaHashMap<T, V> = HashMap<T, V, SeaHash>;
pub type SeaHashSet<V> = HashSet<V, SeaHash>;

impl BuildHasher for SeaHash {
    type Hasher = SeaHasher;

    fn build_hasher(&self) -> Self::Hasher
    {
        Self::Hasher::new()
    }

}

impl SeaHash {
    pub fn set<V>() -> SeaHashSet<V> {
        HashSet::with_hasher(SeaHash)
    }
    pub fn map<T, V>() -> SeaHashMap<V, T> {
        HashMap::with_hasher(SeaHash)
    }
}

