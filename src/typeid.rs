use core::fmt;

use pgrx::prelude::*;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

use crate::base32::{decode_base32_uuid, encode_base32_uuid};

#[derive(Serialize, Deserialize, PostgresType)]
#[inoutfuncs]
pub struct TypeID(String, Uuid);

impl TypeID {
  pub fn new(type_prefix: String, uuid: Uuid) -> Self {
      TypeID(type_prefix, uuid)
  }

  pub fn type_prefix(&self) -> &str {
      &self.0
  }

  pub fn uuid(&self) -> &Uuid {
      &self.1
  }
}

impl fmt::Display for TypeID {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      write!(f, "{}_{}", self.type_prefix(), encode_base32_uuid(self.uuid()))
  }
}


impl InOutFuncs for TypeID {
  fn input(input: &core::ffi::CStr) -> Self {
      // Convert the input to a str and handle potential UTF-8 errors
      let str_input = input.to_str().expect("text input is not valid UTF8");

      // Split the input string once at the first occurrence of '_'
      let mut parts: std::str::SplitN<char> = str_input.splitn(2, '_');
      let part1 = parts.next().expect("Invalid TypeID format");
      let part2 = parts.next().expect("Invalid TypeID format");

      // Decode the UUID part and handle potential errors
      let uuid = decode_base32_uuid(part2).expect("Invalid UUID");

      // Create and return the TypeID
      TypeID(part1.to_string(), uuid)
  }

  fn output(&self, buffer: &mut pgrx::StringInfo) {
      // Use write! macro to directly push the string representation into the buffer
      use std::fmt::Write;
      write!(buffer, "{}", self).expect("Failed to write to buffer");
  }
}