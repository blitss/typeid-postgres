use core::fmt;
use std::cmp::Ordering;

use pgrx::prelude::*;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use uuid::Uuid;

use crate::base32::{decode_base32_uuid, encode_base32_uuid};

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The ID type was not valid
    #[error("invalid TypeID format: {reason}")]
    InvalidFormat { reason: String },
    /// The ID type did not match the expected type
    #[error("prefix '{actual}' is invalid: {reason}")]
    InvalidPrefix { actual: String, reason: String },
    /// The ID suffix was not valid
    #[error("invalid TypeID suffix: {reason}")]
    InvalidSuffix { reason: String },
    /// TypeID is too long
    #[error("TypeID is too long (maximum length is 89 characters)")]
    TooLong,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, PartialOrd)]
pub struct TypeIDPrefix(String);

impl TypeIDPrefix {
    pub fn new(tag: &str) -> Result<Self, Error> {
        Self::validate_prefix(tag).map(|_| Self(tag.to_string()))
    }

    pub fn try_unsafe(tag: &str) -> Self {
        Self(tag.to_string())
    }

    fn validate_prefix(tag: &str) -> Result<(), Error> {
        // Check length
        if tag.len() > 63 {
            return Err(Error::InvalidPrefix {
                actual: tag.to_string(),
                reason: format!("prefix too long ({} characters, maximum is 63)", tag.len()),
            });
        }

        // Check if the prefix is empty (which is valid)
        if tag.is_empty() {
            return Ok(());
        }

        let bytes = tag.as_bytes();

        // Check first and last character for underscores
        if bytes[0] == b'_' {
            return Err(Error::InvalidPrefix {
                actual: tag.to_string(),
                reason: "prefix cannot start with underscore".to_string(),
            });
        }

        if bytes[bytes.len() - 1] == b'_' {
            return Err(Error::InvalidPrefix {
                actual: tag.to_string(),
                reason: "prefix cannot end with underscore".to_string(),
            });
        }

        // Check for invalid characters and provide specific feedback
        for (i, &b) in bytes.iter().enumerate() {
            match b {
                b'a'..=b'z' | b'_' => continue,
                b'A'..=b'Z' => {
                    return Err(Error::InvalidPrefix {
                        actual: tag.to_string(),
                        reason: format!(
                            "uppercase letter '{}' at position {} (prefixes must be lowercase)",
                            b as char, i
                        ),
                    });
                }
                b'0'..=b'9' => {
                    return Err(Error::InvalidPrefix {
                        actual: tag.to_string(),
                        reason: format!("digit '{}' at position {} (prefixes can only contain letters and underscores)", b as char, i),
                    });
                }
                _ => {
                    return Err(Error::InvalidPrefix {
                        actual: tag.to_string(),
                        reason: format!("invalid character '{}' at position {} (only lowercase letters and underscores allowed)", b as char, i),
                    });
                }
            }
        }

        Ok(())
    }

    fn to_type_prefix(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PostgresType, PartialOrd, PartialEq, Eq)]
#[pg_binary_protocol]
#[inoutfuncs]
pub struct TypeID(TypeIDPrefix, Uuid);

impl TypeID {
    pub fn new(type_prefix: TypeIDPrefix, uuid: Uuid) -> Self {
        TypeID(type_prefix, uuid)
    }

    /// Create a TypeID with no prefix (nil prefix)
    pub fn new_nil(uuid: Uuid) -> Self {
        TypeID(TypeIDPrefix::new("").unwrap(), uuid)
    }

    /// Generate a new TypeID with the given prefix using UUID v7
    pub fn generate(prefix: &str) -> Result<Self, Error> {
        let type_prefix = TypeIDPrefix::new(prefix)?;
        Ok(TypeID(type_prefix, Uuid::now_v7()))
    }

    /// Generate a new TypeID with no prefix using UUID v7
    pub fn generate_nil() -> Self {
        TypeID(TypeIDPrefix::new("").unwrap(), Uuid::now_v7())
    }

    pub fn from_string(id: &str) -> Result<Self, Error> {
        // Early validation of total length to prevent processing overly long strings
        if id.len() > 89 {
            // 63 (max prefix) + 1 (separator) + 26 (uuid) = 90, but we allow 89 to account for edge cases
            return Err(Error::TooLong);
        }

        // Split the input string once at the first occurrence of '_'
        let (tag, id) = match id.rsplit_once('_') {
            Some(("", _)) => {
                return Err(Error::InvalidFormat {
                    reason: "TypeID cannot start with separator '_'".to_string(),
                })
            }
            Some((tag, id)) => (tag, id),
            None => ("", id),
        };

        // Validate suffix length early
        if id.len() != 26 {
            return Err(Error::InvalidSuffix {
                reason: format!("expected 26 characters, got {}", id.len()),
            });
        }

        // Decode the UUID part and handle potential errors
        let uuid = decode_base32_uuid(id).map_err(|e| Error::InvalidSuffix {
            reason: e.to_string(),
        })?;

        let prefix = TypeIDPrefix::new(tag)?;

        // Create and return the TypeID
        Ok(TypeID(prefix, uuid))
    }

    pub fn type_prefix(&self) -> &str {
        self.0.to_type_prefix()
    }

    pub fn uuid(&self) -> &Uuid {
        &self.1
    }

    /// Check if this TypeID has an empty prefix
    pub fn is_nil_prefix(&self) -> bool {
        self.type_prefix().is_empty()
    }
}

impl Ord for TypeID {
    fn cmp(&self, b: &Self) -> Ordering {
        match self.type_prefix().cmp(b.type_prefix()) {
            std::cmp::Ordering::Equal => self.uuid().cmp(b.uuid()),
            other => other,
        }
    }
}

impl Hash for TypeID {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_prefix().as_bytes().hash(state);
        self.uuid().hash(state);
    }
}

impl fmt::Display for TypeID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.type_prefix().is_empty() {
            write!(f, "{}", encode_base32_uuid(self.uuid()))
        } else {
            write!(
                f,
                "{}_{}",
                self.type_prefix(),
                encode_base32_uuid(self.uuid())
            )
        }
    }
}

impl InOutFuncs for TypeID {
    fn input(input: &core::ffi::CStr) -> TypeID {
        // Convert the input to a str and handle potential UTF-8 errors
        let str_input = input.to_str().expect("text input is not valid UTF8");

        match TypeID::from_string(str_input) {
            Ok(typeid) => typeid,
            Err(err) => panic!("Failed to construct TypeId<{str_input}>: {err}"),
        }
    }

    fn output(&self, buffer: &mut pgrx::StringInfo) {
        // Use write! macro to directly push the string representation into the buffer
        use std::fmt::Write;
        write!(buffer, "{}", self).expect("Failed to write to buffer");
    }
}
