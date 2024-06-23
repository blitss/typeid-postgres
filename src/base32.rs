use uuid::Uuid;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The ID suffix was not valid
    #[error("id is invalid")]
    InvalidData,
}

fn decode_base32_to_u128(id: &str) -> Result<u128, Error> {
  let mut id: [u8; 26] = id.as_bytes().try_into().map_err(|_| Error::InvalidData)?;
  let mut max = 0;
  for b in &mut id {
      *b = CROCKFORD_INV[*b as usize];
      max |= *b;
  }
  if max > 32 || id[0] > 7 {
      return Err(Error::InvalidData);
  }

  let mut out = 0u128;
  for b in id {
      out <<= 5;
      out |= b as u128;
  }

  Ok(out)
}

fn encode_u128_to_base32(data: u128) -> String {
  let mut buf = [0u8; 26];
  let mut data = data;
  for i in (0..26).rev() {
      buf[i] = CROCKFORD[(data & 0x1f) as usize];
      debug_assert!(buf[i].is_ascii());
      data >>= 5;
  }
  unsafe { String::from_utf8_unchecked(buf.to_vec()) }
}

const CROCKFORD: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";
const CROCKFORD_INV: &[u8; 256] = &{
  let mut output = [255; 256];

  let mut i = 0;
  while i < 32 {
      output[CROCKFORD[i as usize] as usize] = i;
      i += 1;
  }

  output
};


pub fn encode_base32_uuid(uuid: &Uuid) -> String {
  encode_u128_to_base32(uuid.as_u128())
}

pub fn decode_base32_uuid(encoded: &str) -> Result<Uuid, Error> {
  decode_base32_to_u128(encoded).map(|result: u128| Uuid::from_u128(result))
}


#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn test_encode_decode_base32_uuid() {
        let uuid = Uuid::now_v7();
        let encoded = encode_base32_uuid(&uuid);
        println!("{}", encoded);
        let decoded = decode_base32_uuid(&encoded).unwrap();
        assert_eq!(uuid, decoded);
    }
}