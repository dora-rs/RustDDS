use speedy::{Readable, Writable};

use super::submessage_elements::crypto_header_builtin::CryptoHeader;

/// See sections 7.3.7.3 and 7.3.7.8.1
#[derive(Debug, PartialEq, Eq, Clone, Readable, Writable)]
pub struct SecureRTPSPrefix {
  submessage_length: u16, // ushort

  crypto_header: CryptoHeader,
}
