// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use base_message::{
    FieldSection, FieldType, FieldValueError, StaticTableEntry, combination::Singleton,
    static_table_entries,
};

/// A field type that keeps only its value's length, so that the workload measures dispatch and accumulation rather than value parsing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Seen(usize);

impl FieldType<'_> for Seen {
    type Combination = Singleton;

    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self(value.len()))
    }
}

/// The number of declarations.
pub(crate) const DECLARATIONS: usize = 40;

macro_rules! request_section {
    ($($member:ident = $name:literal),+ $(,)?) => {
        /// Forty common HTTP field names, each an optional singleton, as in the measurement `design/performance.md` records.
        #[derive(FieldSection)]
        pub(crate) struct RequestSection {
            $(#[field($name)] $member: Option<Seen>,)+
        }

        /// The declared names in declaration order, which is the order of their declaration ids.
        pub(crate) const DECLARED_NAMES: [&str; DECLARATIONS] = [$($name),+];

        impl RequestSection {
            /// Each declaration's value length, in declaration order.
            pub(crate) fn lengths(&self) -> [Option<usize>; DECLARATIONS] {
                [$(self.$member.map(|seen| seen.0)),+]
            }
        }
    };
}

request_section! {
    accept = "accept",
    accept_charset = "accept-charset",
    accept_encoding = "accept-encoding",
    accept_language = "accept-language",
    authorization = "authorization",
    cache_control = "cache-control",
    connection = "connection",
    content_disposition = "content-disposition",
    content_encoding = "content-encoding",
    content_language = "content-language",
    content_length = "content-length",
    content_location = "content-location",
    content_range = "content-range",
    content_type = "content-type",
    cookie = "cookie",
    date = "date",
    etag = "etag",
    expect = "expect",
    expires = "expires",
    forwarded = "forwarded",
    from = "from",
    host = "host",
    if_match = "if-match",
    if_modified_since = "if-modified-since",
    if_none_match = "if-none-match",
    if_range = "if-range",
    if_unmodified_since = "if-unmodified-since",
    last_modified = "last-modified",
    location = "location",
    max_forwards = "max-forwards",
    origin = "origin",
    pragma = "pragma",
    range = "range",
    referer = "referer",
    set_cookie = "set-cookie",
    te = "te",
    trailer = "trailer",
    transfer_encoding = "transfer-encoding",
    upgrade = "upgrade",
    user_agent = "user-agent",
}

/// A stand-in for a protocol's static table: the declared names themselves, as in the recorded measurement. A real table, such as HPACK's, would also hold names the section does not declare.
pub(crate) const STATIC_TABLE: [StaticTableEntry<RequestSection>; DECLARATIONS] =
    static_table_entries::<RequestSection, DECLARATIONS>(&DECLARED_NAMES);
