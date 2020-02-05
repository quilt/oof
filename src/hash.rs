use crate::V;
use arrayref::array_ref;
use sha2::{Digest, Sha256};

pub fn hash(left: &V, right: &V) -> V {
    let mut buf = [0u8; 64];
    buf[0..32].copy_from_slice(left);
    buf[32..64].copy_from_slice(right);
    let tmp = Sha256::digest(&buf);
    buf[0..32].copy_from_slice(tmp.as_ref());
    *array_ref![buf, 0, 32]
}
