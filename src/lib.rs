#![no_std]

pub mod hash;

extern crate alloc;

use crate::hash::hash;

use alloc::collections::{BTreeMap, BTreeSet, BinaryHeap};
use arrayref::array_ref;
use bonsai::expand;
use core::mem::size_of;
use core::slice::{from_raw_parts, from_raw_parts_mut};

#[cfg(any(test, feature = "generate"))]
use alloc::vec::Vec;

type K = u128;
type V = [u8; 32];
type Map = BTreeMap<K, V>;

#[derive(Clone, Debug, PartialEq)]
pub struct Oof {
    pub map: Map,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    EntryNotFound(K),
}

impl Oof {
    pub fn new(keys: &[K], values: &[V]) -> Self {
        let mut map = Map::new();

        for i in 0..keys.len() {
            map.insert(keys[i], values[i]);
        }

        Self { map }
    }

    pub unsafe fn from_raw(data: *mut u8) -> Self {
        let count = u32::from_le_bytes(*array_ref![from_raw_parts(data, 4), 0, 4]) as usize;
        let keys = data.offset(4) as *mut K;
        let values = data.offset(4 + (count * size_of::<K>()) as isize) as *mut V;

        Self::new(
            from_raw_parts_mut(keys, count),
            from_raw_parts_mut(values, count),
        )
    }

    pub fn from_map(map: Map) -> Self {
        Self { map }
    }

    #[cfg(any(test, feature = "generate"))]
    pub fn to_map(self) -> Map {
        self.map
    }

    #[cfg(any(test, feature = "generate"))]
    pub fn to_bytes(&self) -> Vec<u8> {
        let keys: Vec<u8> = self
            .map
            .keys()
            .flat_map(|k| k.to_le_bytes().to_vec())
            .collect();

        let values: Vec<u8> = self.map.values().flatten().cloned().collect();

        let mut ret = (self.map.keys().len() as u32).to_le_bytes().to_vec();
        ret.extend(keys);
        ret.extend(values);
        ret
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    pub fn set(&mut self, key: K, value: V) -> Option<V> {
        let (_, _, parent) = expand(key);
        self.map.remove(&parent);
        self.map.insert(key, value)
    }

    pub fn root(&mut self) -> Result<&V, Error> {
        self.refresh()?;
        Ok(self.get(&1).ok_or(Error::EntryNotFound(1))?)
    }

    pub fn keys(&self) -> BTreeSet<K> {
        self.map.keys().cloned().collect()
    }

    fn refresh(&mut self) -> Result<(), Error> {
        let mut keys: BinaryHeap<u128> = self.keys().into_iter().collect();

        while let Some(key) = keys.pop() {
            if key <= 1 {
                break;
            }

            let (left, right, parent) = expand(key);

            match (self.get(&left), self.get(&right), self.get(&parent)) {
                (Some(l), Some(r), None) => {
                    let h = hash(l, r);
                    self.set(parent, h);
                    keys.push(parent);
                }
                (Some(_), Some(_), Some(_)) => (),
                (None, _, _) => return Err(Error::EntryNotFound(left)),
                (_, None, _) => return Err(Error::EntryNotFound(right)),
            };
        }

        Ok(())
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
        let mut oof = Oof::new(&mut keys, &mut values);

        let three = hash(&values[1], &values[2]);
        let one = hash(&values[0], &three);

        assert_eq!(oof.root(), Ok(&one));
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

        let oof = unsafe { Oof::from_raw(blob[..].as_ptr() as *mut u8) };

        assert_eq!(oof.get(&1), Some(&build_value(1)));
        assert_eq!(oof.get(&2), Some(&build_value(2)));
        assert_eq!(oof.get(&3), Some(&build_value(3)));
        assert_eq!(oof.get(&4), None);
    }
}
