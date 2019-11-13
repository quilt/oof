#![cfg_attr(not(test), no_std)]

use core::mem::size_of;
use core::slice::{from_raw_parts, from_raw_parts_mut};

use arrayref::array_ref;
use bonsai::expand;
use sha2::{Digest, Sha256};

type K = u128;
type V = u128;

pub struct Oof<'a> {
    pub keys: &'a [K],
    pub values: &'a mut [V],
    pub height: u32,
    is_dirty: bool,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
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

    pub unsafe fn from_blob(data: *mut u8, height: u32) -> Oof<'a> {
        let count = from_raw_parts(data, 4);
        let count = u32::from_le_bytes(*array_ref![count, 0, 4]) as usize;

        let keys_ptr = data.offset(4) as *const u128;
        let keys = from_raw_parts(keys_ptr, count);

        let values_ptr = data.offset(4 + (count * size_of::<K>()) as isize) as *mut u128;
        let values = from_raw_parts_mut(values_ptr, count);

        Oof {
            keys,
            values,
            height,
            is_dirty: false,
        }
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

            let mut buf = [0u8; 32];
            hash_children(&mut buf, left, right);

            self.set(parent, u128::from_le_bytes(*array_ref![buf, 0, 16]))?;

            position -= 1;
        }

        self.is_dirty = false;

        Ok(())
    }
}

fn hash_children(buf: &mut [u8; 32], left: &V, right: &V) {
    buf[0..16].copy_from_slice(&left.to_le_bytes());
    buf[16..32].copy_from_slice(&right.to_le_bytes());
    let tmp = Sha256::digest(buf);
    buf[0..32].copy_from_slice(tmp.as_ref());
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::transmute;

    #[test]
    fn get() {
        let oof = Oof {
            keys: &[1, 2, 3],
            values: &mut [1, 2, 3],
            height: 1,
            is_dirty: false,
        };

        assert_eq!(oof.get(&1), Some(&1));
        assert_eq!(oof.get(&2), Some(&2));
        assert_eq!(oof.get(&3), Some(&3));
        assert_eq!(oof.get(&4), None);
    }

    #[test]
    fn set() {
        let mut oof = Oof {
            keys: &[1, 2, 3],
            values: &mut [1, 2, 3],
            height: 1,
            is_dirty: false,
        };

        assert_eq!(oof.set(1, 2), Ok(1));
        assert_eq!(oof.set(2, 3), Ok(2));
        assert_eq!(oof.set(3, 4), Ok(3));
        assert_eq!(oof.set(4, 5), Err(Error::EntryNotFound(4)));
    }

    #[test]
    fn root() {
        let mut oof = Oof {
            keys: &[1, 2, 3],
            values: &mut [1, 2, 3],
            height: 2,
            is_dirty: true,
        };

        let mut buf = [0u8; 32];
        hash_children(&mut buf, &oof.values[1], &oof.values[2]);
        let root = u128::from_le_bytes(*array_ref![buf, 0, 16]);

        assert_eq!(oof.root(), Ok(&root));
    }

    #[test]
    fn from_blob() {
        let count: u32 = 3;

        let keys: [u128; 3] = [1, 2, 3];
        let values: [u128; 3] = [1, 2, 3];

        let keys: [u8; 48] = unsafe { transmute(keys) };
        let values: [u8; 48] = unsafe { transmute(values) };

        let mut blob = [0u8; (4 + 48 + 48)];
        blob[0..4].copy_from_slice(&count.to_le_bytes());
        blob[4..52].copy_from_slice(&keys[..]);
        blob[52..100].copy_from_slice(&values[..]);

        let oof = unsafe { Oof::from_blob(blob[..].as_ptr() as *mut u8, 2) };

        assert_eq!(oof.get(&1), Some(&1));
        assert_eq!(oof.get(&2), Some(&2));
        assert_eq!(oof.get(&3), Some(&3));
        assert_eq!(oof.get(&4), None);
    }
}
