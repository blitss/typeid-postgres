use core::fmt;
use std::borrow::Cow;

use pgrx::prelude::*;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

use crate::base32::{decode_base32_uuid, encode_base32_uuid};

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The ID type was not valid
    #[error("id type is invalid")]
    InvalidType,
    /// The ID type did not match the expected type
    #[error("id type {actual:?} does not match expected {expected:?}")]
    IncorrectType {
        actual: String,
        expected: Cow<'static, str>,
    },
    /// The ID suffix was not valid
    #[error("id suffix is invalid")]
    InvalidData,
}

#[derive(Serialize, Deserialize)]
pub struct TypeIDPrefix(String);

impl TypeIDPrefix {
    pub fn new(tag: &str) -> Result<Self, Error> {
        Self::try_from_type_prefix(tag).map_err(|expected| Error::IncorrectType {
            actual: tag.into(),
            expected,
        })
    }

    pub fn try_unsafe(tag: &str) -> Self {
      Self(tag.to_string())
    }

    fn try_from_type_prefix(tag: &str) -> Result<Self, Cow<'static, str>> {
        // Check length
        if tag.len() > 63 {
            return Err(tag[..63].to_owned().into());
        }

        // Check if the prefix is empty
        if tag.is_empty() {
            return Ok(Self(tag.to_string()));
        }

        // Check first and last character
        let bytes = tag.as_bytes();
        let first_char = bytes[0];
        let last_char = bytes[bytes.len() - 1];

        if first_char == b'_' || last_char == b'_' {
            return Err(tag.to_lowercase().into());
        }

        // Check all characters
        if !bytes.iter().all(|&b| matches!(b, b'a'..=b'z' | b'_')) {
            return Err(tag.to_lowercase().into());
        }

        Ok(Self(tag.to_string()))
    }

    fn to_type_prefix(&self) -> &str {
        &self.0
    }
}

#[derive(Serialize, Deserialize, PostgresType)]
#[inoutfuncs]
pub struct TypeID(TypeIDPrefix, Uuid);

impl TypeID {
  pub fn new(type_prefix: TypeIDPrefix, uuid: Uuid) -> Self {
      TypeID(type_prefix, uuid)
  }

  pub fn from_string(id: &str) -> Result<Self, Error> {
      // Split the input string once at the first occurrence of '_'
      let (tag, id) = match id.rsplit_once('_') {
        Some(("", _)) => return Err(Error::InvalidType),
        Some((tag, id)) => (tag, id),
        None => ("", id),
      };

      // Decode the UUID part and handle potential errors
      let uuid = decode_base32_uuid(id).map_err(|_| Error::InvalidData)?;

      let prefix = TypeIDPrefix::new(tag)?;

      // Create and return the TypeID
      Ok(TypeID(prefix, uuid))
  }

  pub fn type_prefix(&self) -> &str {
      &self.0.to_type_prefix()
  }

  pub fn uuid(&self) -> &Uuid {
      &self.1
  }
}

impl fmt::Display for TypeID {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      if self.type_prefix().is_empty() {
          write!(f, "{}", encode_base32_uuid(self.uuid()))
      } else {
          write!(f, "{}_{}", self.type_prefix(), encode_base32_uuid(self.uuid()))
      }
  }
}


impl InOutFuncs for TypeID {
  fn input(input: &core::ffi::CStr) -> TypeID {
      // Convert the input to a str and handle potential UTF-8 errors
      let str_input = input.to_str().expect("text input is not valid UTF8");

      TypeID::from_string(str_input).unwrap()
  }

  fn output(&self, buffer: &mut pgrx::StringInfo) {
      // Use write! macro to directly push the string representation into the buffer
      use std::fmt::Write;
      write!(buffer, "{}", self).expect("Failed to write to buffer");
  }
}