#![cfg_attr(not(test), no_std)]

type K = u128;
type V = u128;

pub struct Proof<'a> {
    pub keys: &'a [K],
    pub values: &'a mut [V],
    pub height: u32,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum Error {
    EntryNotFound,
}

impl<'a> Proof<'a> {
    pub fn new(keys: &'a [K], values: &'a mut [V], height: u32) -> Self {
        Proof {
            keys,
            values,
            height,
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
                Ok(old)
            }
            Err(_) => Err(Error::EntryNotFound),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get() {
        let proof = Proof {
            keys: &[1, 2, 3],
            values: &mut [1, 2, 3],
            height: 2,
        };

        assert_eq!(proof.get(&1), Some(&1));
        assert_eq!(proof.get(&2), Some(&2));
        assert_eq!(proof.get(&3), Some(&3));
        assert_eq!(proof.get(&4), None);
    }

    #[test]
    fn set() {
        let mut proof = Proof {
            keys: &[1, 2, 3],
            values: &mut [1, 2, 3],
            height: 2,
        };

        assert_eq!(proof.set(1, 2), Ok(1));
        assert_eq!(proof.set(2, 3), Ok(2));
        assert_eq!(proof.set(3, 4), Ok(3));
        assert_eq!(proof.set(4, 5), Err(Error::EntryNotFound));
    }
}
