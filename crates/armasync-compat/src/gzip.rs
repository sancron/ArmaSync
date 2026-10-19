// SPDX-License-Identifier: AGPL-3.0-only
//! Bounded GZIP decompression of one member. The checksum trailer is verified (the data is read to its end) and
//! bytes after the member are rejected; the output never exceeds the limit, so a small input cannot expand without
//! bound.

use std::io::Read;

use flate2::bufread::GzDecoder;

use crate::error::{ParseError, ParseErrorKind};
use crate::stream::Limits;

pub(crate) fn gunzip(input: &[u8], limits: &Limits) -> Result<Vec<u8>, ParseError> {
    if input.len() > limits.max_compressed {
        return Err(ParseError::new(
            ParseErrorKind::TooLarge,
            "the file is larger than the accepted maximum",
        ));
    }
    if input.len() < 2 || input[0] != 0x1f || input[1] != 0x8b {
        return Err(ParseError::new(
            ParseErrorKind::NotCompressed,
            "the file is not GZIP data",
        ));
    }
    let mut decoder = GzDecoder::new(input);
    let mut output = Vec::new();
    // One byte more than allowed: reading it proves the limit is exceeded without decompressing further.
    let mut bounded = (&mut decoder).take(limits.max_decompressed as u64 + 1);
    if bounded.read_to_end(&mut output).is_err() {
        return Err(ParseError::new(
            ParseErrorKind::CompressionCorrupt,
            "the GZIP data is damaged or truncated",
        ));
    }
    if output.len() > limits.max_decompressed {
        return Err(ParseError::new(
            ParseErrorKind::TooLarge,
            "the decompressed data exceeds the accepted maximum",
        ));
    }
    if !decoder.get_ref().is_empty() {
        return Err(ParseError::new(
            ParseErrorKind::CompressionCorrupt,
            "unexpected data after the GZIP member",
        ));
    }
    Ok(output)
}
