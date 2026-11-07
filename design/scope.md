# Scope

## Requirement

`base-message` must hold the field sections of the message abstraction that HTTP (RFC 9110 section 6), the Internet Message Format (RFC 5322), and MIME entities (RFC 2045) share: header and trailer sections of named fields, their interpretation into typed structs, and, as they are added, their wire syntax for parsing and writing. Control data, such as request lines, status codes, and HTTP/2 pseudo-headers, as well as content, framing, and transport, belong to each protocol's own crate.

The crate must not depend on an HTTP, web, or email crate; `base-http`, `base-web-multipart`, and future email crates depend on it. Protocol-specific rules stay with those crates: which fields are singletons or forbidden for a protocol is expressed in their field types and section declarations, and protocol static tables, such as HPACK's and QPACK's, are defined by the crates that decode them.

Interpretation takes fields as name and value pairs in wire order and never reorders, merges, or stores them beyond what a section declares. The ordered fields remain the source of truth, owned by the caller, so that any number of sections can interpret them.

## Rationale

The user asked for a shared message layer, after the Ruby `protocol-message` gem, so that HTTP and multipart part headers, and later email, interpret fields the same way. Calling it `base-web-message` would make `base-http`, the transport, depend on the web content family, and putting it in `base-http` would make multipart depend on HTTP, so it is a standalone crate both depend on. The user chose the name `base-message` because RFC 9110 and RFC 5322 both name this structure the message; the scope is recorded here, and in `base-project`'s Package Organization guide, so that the generic name does not draw in bodies, framing, or transport. The user's goal was a standard way to map N fields to M ≤ N typed entries, because interpretation is where implementations go wrong.

## Scope

The `base-message` and `base-message-macros` crates as a whole.

## Validation

Inspect `cargo tree -p base-message -e normal`, which must list only `base-message-macros`, and the macro crate's dependencies, which must be only `proc-macro2`, `quote`, and `syn`. Review new public items against this scope.
