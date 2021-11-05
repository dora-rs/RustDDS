// This module defines traits to specifiy a key as defined in DDS specification.
// See e.g. Figure 2.3 in "2.2.1.2.2 Overall Conceptual Model"
use std::{
  collections::hash_map::DefaultHasher,
  convert::TryFrom,
  hash::{Hash, Hasher},
};

use byteorder::BigEndian;
use rand::Rng;
use log::error;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
pub use cdr_encoding_size::*;

use crate::serialization::{cdr_serializer::to_bytes, error::Error};

/// A sample data type may be `Keyed` : It allows a Key to be extracted from the
/// sample. In its simplest form, the key may be just a part of the sample data,
/// but it can be anything computable from a sample by an application-defined
/// function.
///
/// The key is used to distinguish between different Instances of the data in a
/// DDS Topic.
///
/// A `Keyed` type has an associated type `K`, which is the actual key type. `K`
/// must implement [`Key`]. Otherwise, `K` can be chosen to suit the
/// application. It is advisable that `K` is something that can be cloned with
/// reasonable effort.
///
/// [`Key`]: trait.Key.html

pub trait Keyed {
  //type K: Key;  // This does not work yet is stable Rust, 2020-08-11
  // Instead, where D:Keyed we do anything with D::K, we must specify bound:
  // where <D as Keyed>::K : Key,
  type K;

  fn get_key(&self) -> Self::K;

  // provided method (TODO: what for?)
  fn get_hash(&self) -> u64
  where
    Self::K: Key,
  {
    let mut hasher = DefaultHasher::new();
    self.get_key().hash(&mut hasher);
    hasher.finish()
  }
}

// See RTPS spec Section 8.7.10 Key Hash
// and Section 9.6.3.8 KeyHash
#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Clone, Copy)]
pub struct KeyHash([u8; 16]);

impl KeyHash {
  pub fn zero() -> KeyHash {
    KeyHash([0; 16])
  }

  pub fn to_vec(self) -> Vec<u8> {
    Vec::from(self.0)
  }

  pub fn into_cdr_bytes(self) -> Result<Vec<u8>, Error> {
    Ok(self.to_vec())
  }

  pub fn from_cdr_bytes(bytes: Vec<u8>) -> Result<KeyHash, Error> {
    let a = <[u8; 16]>::try_from(bytes).map_err(|_e| Error::Eof)?;
    Ok(KeyHash(a))
  }
}

/// Key trait for Keyed Topics
///
/// It is a combination of traits from the standard library
/// * [PartialEq](https://doc.rust-lang.org/std/cmp/trait.PartialEq.html)
/// * [Eq](https://doc.rust-lang.org/std/cmp/trait.Eq.html)
/// * [PartialOrd](https://doc.rust-lang.org/std/cmp/trait.PartialOrd.html)
/// * [Ord](https://doc.rust-lang.org/std/cmp/trait.Ord.html)
/// * [Hash](https://doc.rust-lang.org/std/hash/trait.Hash.html)
/// * [Clone](https://doc.rust-lang.org/std/clone/trait.Clone.html)
///
/// and Serde traits
/// * [Serialize](https://docs.serde.rs/serde/trait.Serialize.html) and
/// * [DeserializeOwned](https://docs.serde.rs/serde/de/trait.DeserializeOwned.html)
///   .
///
/// Note: When implementing Key, DeserializeOwned cannot and need not be
/// derived, as it is a type alias. Derive (or implement) the Deserialize trait
/// instead.

pub trait Key:
  Eq + PartialEq + PartialOrd + Ord + Hash + Clone + Serialize + DeserializeOwned + CdrEncodingSize
{
  // no methods required

  // provided method:
  fn hash_key(&self) -> KeyHash {
    // See RTPS Spec v2.3 Section 9.6.3.8 KeyHash

    /* The KeyHash_t is computed from the Data as follows using one of two algorithms depending on whether
        the Data type is such that the maximum size of the sequential CDR encapsulation of
        all the key fields is less than or equal to 128 bits (the size of the KeyHash_t).

        • If the maximum size of the sequential CDR representation of all the key fields is less
        than or equal to 128 bits, then the KeyHash_t shall be computed as the CDR Big-Endian
        representation of all the Key fields in sequence. Any unfilled bits in the KeyHash_t
        shall be set to zero.
        • Otherwise the KeyHash_t shall be computed as a 128-bit MD5 Digest (IETF RFC 1321)
        applied to the CDR Big- Endian representation of all the Key fields in sequence.

        Note that the choice of the algorithm to use depends on the data-type,
        not on any particular data value.
    */

    // The specification calls for "sequential CDR representation of all the key
    // fields" and "CDR Big- Endian representation of all the Key fields in
    // sequence". We take this to mean the CDR encoding of the Key.
    // (Does it include CDR-specified alignment padding too?)
    //

    let mut cdr_bytes = to_bytes::<Self, BigEndian>(self).unwrap_or_else(|e| {
      error!("Hashing key {:?} failed!", e);
      // This would cause a lot of hash collisions, but wht else we could do
      // if the key cannot be serialized? Are there any realistic conditions
      // this could even ocur?
      vec![0; 16]
    });

    KeyHash(
      if Self::cdr_encoding_max_size() > CdrEncodingMaxSize::Bytes(16) {
        // use MD5 hash to get the hash. The MD5 hash is always exactly
        // 16 bytes, so just deref it to [u8;16]
        *md5::compute(&cdr_bytes)
      } else {
        cdr_bytes.resize(16, 0x00); // pad with zeros to get 16 bytes
        <[u8; 16]>::try_from(cdr_bytes).unwrap() // this succeeds, because of
                                                 // the resize above
      },
    )
  }
}

impl Key for () {
  fn hash_key(&self) -> KeyHash {
    KeyHash::zero()
  }
}

/// Key for a reference type `&D` is the same as for the value type `D`.
/// This is required internally for the implementation of NoKey topics.
impl<D: Keyed> Keyed for &D {
  type K = D::K;
  fn get_key(&self) -> Self::K {
    (*self).get_key()
  }
}

// TODO: might want to implement this for each primitive?
impl Key for bool {}
impl Key for char {}
impl Key for i8 {}
impl Key for i16 {}
impl Key for i32 {}
impl Key for i64 {}
impl Key for i128 {}
//impl Key for isize {} // should not be used in serializable data, as size is
// platform-dependent
impl Key for u8 {}
impl Key for u16 {}
impl Key for u32 {}
impl Key for u64 {}
impl Key for u128 {}
//impl Key for usize {} // should not be used in serializable data, as size is
// platform-dependent

impl Key for String {}

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize, CdrEncodingSize)]
/// Key type to identicy data instances in builtin topics
pub struct BuiltInTopicKey {
  /// IDL PSM (2.3.3, pg 138) uses array of 3x long to implement this
  value: [i32; 3],
}

impl BuiltInTopicKey {
  pub fn get_random_key() -> BuiltInTopicKey {
    let mut rng = rand::thread_rng();
    BuiltInTopicKey {
      value: [rng.gen(), rng.gen(), rng.gen()],
    }
  }

  pub fn default() -> BuiltInTopicKey {
    BuiltInTopicKey { value: [0, 0, 0] }
  }
}
