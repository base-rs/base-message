// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! The field sections of protocol messages, interpreted into typed structs.
//!
//! HTTP (RFC 9110 section 6), the Internet Message Format of email (RFC 5322), and MIME parts (RFC 2045) share one message structure: header and trailer sections of named fields, around the content. This crate interprets those sections. Each [`FieldType`] says how its instances combine and how one value parses, and `#[derive(FieldSection)]` generates the interpretation of a whole section, so duplicate singletons, lists with quoted commas, forbidden fields, and unknown fields are handled in one reviewed place rather than by each consumer.
//!
//! ```
//! use base_message::{FieldSection, FieldType, FieldValueError, ascii_text, combination::{List, Singleton}};
//!
//! /// A field type that borrows its value from the input.
//! struct Host<'a>(&'a str);
//!
//! impl<'a> FieldType<'a> for Host<'a> {
//!     type Combination = Singleton;
//!     fn parse(value: &'a [u8]) -> Result<Self, FieldValueError> {
//!         ascii_text(value).map(Self)
//!     }
//! }
//!
//! /// A field type that owns its value.
//! struct Accept(String);
//!
//! impl FieldType<'_> for Accept {
//!     type Combination = List;
//!     fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
//!         Ok(Self(ascii_text(value)?.to_owned()))
//!     }
//! }
//!
//! #[derive(FieldSection)]
//! #[fields(forbid("transfer-encoding"))]
//! struct Request<'a> {
//!     #[field("host")]
//!     host: Host<'a>,
//!     #[field("accept")]
//!     accept: Vec<Accept>,
//! }
//!
//! let fields = [("Host", "example.com"), ("Accept", "text/html, image/*"), ("User-Agent", "curl/8")];
//! let request = Request::interpret(fields)?;
//! assert_eq!(request.host.0, "example.com");
//! assert_eq!(request.accept.len(), 2);
//!
//! let duplicate = Request::interpret([("Host", "a"), ("Host", "b")]).err().expect("a duplicate host");
//! assert_eq!(duplicate.to_string(), "duplicate host field at position 1, first at 0");
//! # Ok::<(), base_message::FieldError>(())
//! ```
//!
//! The crate interprets field sections only. Parsing them from the wire, as well as control data, such as a request line or status, content, framing, and transport, belongs to each protocol's own crate, which passes the fields it parses as name and value pairs in wire order. The [Getting Started](https://github.com/base-rs/base-message/blob/main/context/getting-started.md) guide covers field types and sections, and the [Layering](https://github.com/base-rs/base-message/blob/main/context/layering.md) guide covers passing fields between layers, trailers, and static tables.

mod accumulate;
pub mod combination;
mod field_error;
mod field_name;
mod field_names;
mod field_ref;
mod field_section;
mod field_type;
mod field_value;
mod field_value_error;
mod owned_fields;
mod static_table_entry;

#[doc(hidden)]
#[path = "private.rs"]
pub mod __private;

pub use base_message_macros::FieldSection;
pub use field_error::{FieldError, FieldErrorKind};
pub use field_name::FieldName;
pub use field_names::FieldNames;
pub use field_ref::{FieldRef, IntoField};
pub use field_section::FieldSection;
pub use field_type::{FieldType, InvalidValue};
pub use field_value::{ListElements, ascii_text, split_list, trim_whitespace};
pub use field_value_error::FieldValueError;
pub use owned_fields::{OwnedField, OwnedFields};
pub use static_table_entry::{Classification, StaticTableEntry, static_table_entries};

#[cfg(doctest)]
#[doc = include_str!("../readme.md")]
struct Readme;

#[cfg(doctest)]
#[doc = include_str!("../context/getting-started.md")]
struct GettingStartedGuide;

#[cfg(doctest)]
#[doc = include_str!("../context/layering.md")]
struct LayeringGuide;
