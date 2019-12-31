#![no_std]

mod hash;

extern crate alloc;

use crate::hash::hash;

#[cfg(feature = "generate")]
use crate::hash::zero_hash;

use alloc::{collections::BTreeMap, vec::Vec};
use arrayref::array_ref;
use bonsai::expand;
use core::mem::size_of;
use core::slice::{from_raw_parts, from_raw_parts_mut};

#[cfg(feature = "generate")]
use bonsai::{children, relative_depth, subtree_index_to_general};

type K = u128;
type V = [u8; 32];
type Map = BTreeMap<K, V>;

#[derive(Clone)]
pub struct Oof {
    pub map: Map,
    pub height: u32,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    EntryNotFound(K),
}

impl Oof {
    pub fn new(keys: &[K], values: &[V], height: u32) -> Self {
        let mut map = Map::new();

        for i in 0..keys.len() {
            map.insert(keys[i], values[i]);
        }

        Self { map, height }
    }

    pub unsafe fn from_blob(data: *mut u8, height: u32) -> Self {
        let count = u32::from_le_bytes(*array_ref![from_raw_parts(data, 4), 0, 4]) as usize;
        let keys = data.offset(4) as *mut K;
        let values = data.offset(4 + (count * size_of::<K>()) as isize) as *mut V;

        Self::new(
            from_raw_parts_mut(keys, count),
            from_raw_parts_mut(values, count),
            height,
        )
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(&key)
    }

    pub fn set(&mut self, key: K, value: V) -> Option<V> {
        self.map.insert(key, value)
    }

    pub fn root(&mut self) -> Result<&V, Error> {
        self.refresh()?;
        Ok(self.get(&1).ok_or(Error::EntryNotFound(1))?)
    }

    fn keys(&self) -> Vec<K> {
        let mut keys: Vec<u128> = self.map.keys().cloned().collect();
        keys.sort_by(|a, b| b.cmp(a));
        keys
    }

    fn refresh(&mut self) -> Result<(), Error> {
        let mut nodes: Vec<u128> = self.keys();
        let mut position = 0;

        while nodes[position] > 1 {
            let (left, right, parent) = expand(nodes[position]);

            match (self.get(&left), self.get(&right), self.get(&parent)) {
                (Some(l), Some(r), None) => {
                    let h = hash(l, r);
                    self.set(parent, h);
                    nodes.push(parent);
                }
                (Some(_), Some(_), Some(_)) => (),
                (None, _, _) => return Err(Error::EntryNotFound(left)),
                (_, None, _) => return Err(Error::EntryNotFound(right)),
            };

            position += 1;
        }

        Ok(())
    }

    #[cfg(feature = "generate")]
    pub fn to_map(self) -> Map {
        self.map
    }

    #[cfg(feature = "generate")]
    pub fn from_map(map: Map, height: u32) -> Self {
        Self { map, height }
    }

    #[cfg(feature = "generate")]
    pub fn fill_with_default(&mut self, default: &V) {
        let mut nodes: Vec<u128> = self.keys();
        nodes.sort_by(|a, b| b.cmp(a));

        let mut position = 0;
        while nodes[position] > 1 {
            let (left, right, parent) = expand(nodes[position]);

            if !self.map.contains_key(&parent) {
                let left = self
                    .map
                    .entry(left)
                    .or_insert(zero_hash(default, relative_depth(left, 1 << self.height)))
                    .clone();

                let right = self
                    .map
                    .entry(right)
                    .or_insert(zero_hash(default, relative_depth(right, 1 << self.height)));

                let h = hash(&left, &right);
                self.set(parent, h);
                nodes.push(parent);
            }

            position += 1;
        }
    }

    #[cfg(feature = "generate")]
    pub fn into_branch(mut self) -> Self {
        for key in self.keys() {
            let (left, right) = children(key);

            if self.map.contains_key(&left) || self.map.contains_key(&right) {
                self.map.remove(&key);
            }
        }

        self
    }

    #[cfg(feature = "generate")]
    pub fn to_subtree(&mut self, root: K) {
        let keys = self.keys();
        for i in 0..keys.len() {
            let value = self.map.remove(&keys[i]).unwrap();
            self.set(subtree_index_to_general(root, keys[i]), value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::transmute;

    fn build_value(n: u8) -> [u8; 32] {
        let mut tmp = [0u8; 32];
        tmp[0] = n;
        tmp
    }

    #[test]
    fn root() {
        let mut keys = [2, 6, 7];
        let mut values = [build_value(2), build_value(6), build_value(7)];
        let mut oof = Oof::new(&mut keys, &mut values, 1);

        let three = hash(&values[1], &values[2]);
        let one = hash(&values[0], &three);

        assert_eq!(oof.root(), Ok(&one));
    }

    #[cfg(feature = "generate")]
    #[test]
    fn fill_and_minimize() {
        let mut map = Map::new();
        map.insert(14, build_value(14));
        map.insert(15, build_value(15));

        let mut oof = Oof::from_map(map.clone(), 3);
        oof.fill_with_default(&[0u8; 32]);

        map.insert(7, hash(map.get(&14).unwrap(), map.get(&15).unwrap()));
        map.insert(6, zero_hash(&[0u8; 32], 1));
        map.insert(2, zero_hash(&[0u8; 32], 2));
        map.insert(3, hash(map.get(&6).unwrap(), map.get(&7).unwrap()));
        map.insert(1, hash(map.get(&2).unwrap(), map.get(&3).unwrap()));

        assert_eq!(oof.clone().to_map(), map);

        map.remove(&1);
        map.remove(&3);
        map.remove(&7);

        assert_eq!(oof.into_branch().to_map(), map);
    }

    #[cfg(feature = "generate")]
    #[test]
    fn to_subtree() {
        let mut keys = [1, 2, 3];
        let mut values = [build_value(1), build_value(2), build_value(3)];
        let mut oof = Oof::new(&mut keys, &mut values, 1);

        oof.to_subtree(5);

        assert_eq!(oof.get(&5), Some(&build_value(1)));
        assert_eq!(oof.get(&10), Some(&build_value(2)));
        assert_eq!(oof.get(&11), Some(&build_value(3)));

        assert_eq!(oof.get(&1), None);
        assert_eq!(oof.get(&2), None);
        assert_eq!(oof.get(&3), None);
        assert_eq!(oof.get(&12), None);
    }

    #[test]
    fn from_blob() {
        let count: u32 = 3;

        let keys: [K; 3] = [1, 2, 3];
        let values: [V; 3] = [build_value(1), build_value(2), build_value(3)];

        let keys: [u8; 48] = unsafe { transmute(keys) };
        let values: [u8; 96] = unsafe { transmute(values) };

        let mut blob = [0u8; (4 + 48 + 96)];
        blob[0..4].copy_from_slice(&count.to_le_bytes());
        blob[4..52].copy_from_slice(&keys[..]);
        blob[52..148].copy_from_slice(&values[..]);

        let oof = unsafe { Oof::from_blob(blob[..].as_ptr() as *mut u8, 2) };

        assert_eq!(oof.get(&1), Some(&build_value(1)));
        assert_eq!(oof.get(&2), Some(&build_value(2)));
        assert_eq!(oof.get(&3), Some(&build_value(3)));
        assert_eq!(oof.get(&4), None);
    }
}
