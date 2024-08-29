use std::collections::{HashMap, HashSet, BinaryHeap};
use std::hash::{Hash, Hasher, BuildHasher};
use std::ops::{Index, Range};
use seahash::SeaHasher;
use delegate::delegate;

pub struct SeaHash;

impl BuildHasher for SeaHash {
    type Hasher = SeaHasher;

    fn build_hasher(&self) -> Self::Hasher
    {
        Self::Hasher::new()
    }

}

//---- SeaHashKey

pub type SeaHashKey = [u8; 12];

//---- SeaHashMap

pub struct SeaHashMap<K, V>(HashMap<K, V, SeaHash>);

impl<K, V> SeaHashMap<K, V>
    where K: Eq, K: Hash
{
    pub fn new() -> SeaHashMap<K, V> {
        Self {
            0: HashMap::with_hasher(SeaHash)
        }
    }

    delegate! {
        to self.0 {
            pub fn index(&self, index: &K) -> &V;
            pub fn get(&self, k: &K) -> Option<&V>;
            pub fn insert(&mut self, k: K, v: V) -> Option<V>;
            pub fn contains_key(&self, k: &K) -> bool;
            pub fn keys(&self) -> std::collections::hash_map::Keys<K, V>;
        }
    }
}

impl<K, V> IntoIterator for SeaHashMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::collections::hash_map::IntoIter<K, V>;
    delegate! {
        to self.0 {
            fn into_iter(self) -> Self::IntoIter;
        }
    }
}

impl<'a, K, V> IntoIterator for &'a SeaHashMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = std::collections::hash_map::Iter<'a, K, V>;
    delegate! {
        to self.0 {
            #[call(iter)]
            fn into_iter(self) -> Self::IntoIter;
        }
    }
}

//---- SeaHashSet

pub struct SeaHashSet<V>(HashSet<V, SeaHash>);

impl<V> SeaHashSet<V>
    where V: Eq, V: Hash
{
    pub fn new() -> SeaHashSet<V> {
        Self {
            0: HashSet::with_hasher(SeaHash)
        }
    }

    delegate! {
        to self.0 {
            pub fn contains(&self, v: &V) -> bool;
            pub fn insert(&mut self, v: V) -> bool;
            pub fn clear(&mut self);
            pub fn remove(&mut self, v: &V) -> bool;
        }
    }

}

