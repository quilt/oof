#![cfg_attr(not(test), no_std)]

use core::mem::size_of;
use core::slice::{from_raw_parts, from_raw_parts_mut};

use arrayref::array_ref;
use bonsai::expand;
use sha2::{Digest, Sha256};

type K = u128;
type V = [u8; 32];

pub struct Oof<'a> {
    pub keys: &'a [K],
    pub values: &'a mut [V],
    pub height: u32,
    is_dirty: bool,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    EntryNotFound(K),
}

impl<'a> Oof<'a> {
    pub fn new(keys: &'a [K], values: &'a mut [V], height: u32) -> Self {
        Oof {
            keys,
            values,
            height,
            is_dirty: false,
        }
    }

    pub unsafe fn from_blob(data: *mut u8, height: u32) -> Self {
        let count = u32::from_le_bytes(*array_ref![from_raw_parts(data, 4), 0, 4]) as usize;
        let keys = data.offset(4) as *const K;
        let values = data.offset(4 + (count * size_of::<K>()) as isize) as *mut V;

        Self::new(
            from_raw_parts(keys, count),
            from_raw_parts_mut(values, count),
            height,
        )
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        match self.keys.binary_search(&key) {
            Ok(index) => Some(&self.values[index]),
            Err(_) => None,
        }
    }

    pub fn set(&mut self, key: K, value: V) -> Result<V, Error> {
        match self.keys.binary_search(&key) {
            Ok(index) => {
                let old = self.values[index];
                self.values[index] = value;
                self.is_dirty = true;
                Ok(old)
            }
            Err(_) => Err(Error::EntryNotFound(key)),
        }
    }

    pub fn root(&mut self) -> Result<&V, Error> {
        if self.is_dirty {
            self.refresh()?;
        }

        Ok(self.get(&1).ok_or(Error::EntryNotFound(1))?)
    }

    fn refresh(&mut self) -> Result<(), Error> {
        let mut position = self.keys.len() - 1;

        while position > 0 {
            let (left, right, parent) = expand(self.keys[position]);

            let left = self.get(&left).ok_or(Error::EntryNotFound(left))?;
            let right = self.get(&right).ok_or(Error::EntryNotFound(right))?;

            let mut buf = [0u8; 64];
            hash_children(&mut buf, left, right);

            self.set(parent, *array_ref![buf, 0, 32])?;

            position -= 1;
        }

        self.is_dirty = false;

        Ok(())
    }
}

fn hash_children(buf: &mut [u8; 64], left: &V, right: &V) {
    buf[0..32].copy_from_slice(left);
    buf[32..64].copy_from_slice(right);
    let tmp = Sha256::digest(buf);
    buf[0..32].copy_from_slice(tmp.as_ref());
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
    fn get() {
        let oof = Oof {
            keys: &[1, 2, 3],
            values: &mut [build_value(1), build_value(2), build_value(3)],
            height: 1,
            is_dirty: false,
        };

        assert_eq!(oof.get(&1), Some(&build_value(1)));
        assert_eq!(oof.get(&2), Some(&build_value(2)));
        assert_eq!(oof.get(&3), Some(&build_value(3)));
        assert_eq!(oof.get(&4), None);
    }

    #[test]
    fn set() {
        let mut oof = Oof {
            keys: &[1, 2, 3],
            values: &mut [build_value(1), build_value(2), build_value(3)],
            height: 1,
            is_dirty: false,
        };

        assert_eq!(oof.set(1, build_value(2)), Ok(build_value(1)));
        assert_eq!(oof.set(2, build_value(3)), Ok(build_value(2)));
        assert_eq!(oof.set(3, build_value(4)), Ok(build_value(3)));
        assert_eq!(oof.set(4, build_value(5)), Err(Error::EntryNotFound(4)));
    }

    #[test]
    fn root() {
        let mut oof = Oof {
            keys: &[1, 2, 3],
            values: &mut [build_value(1), build_value(2), build_value(3)],
            height: 2,
            is_dirty: true,
        };

        let mut buf = [0u8; 64];
        hash_children(&mut buf, &oof.values[1], &oof.values[2]);

        assert_eq!(oof.root(), Ok(array_ref![buf, 0, 32]));
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
