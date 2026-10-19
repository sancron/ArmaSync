// SPDX-License-Identifier: AGPL-3.0-only
//! A bounded, data-only reader for the object serialization stream format used by existing repository servers.
//!
//! The format is publicly documented (type codes, handles, class descriptors). The reader turns a stream into plain
//! data (objects with named fields); it never loads, instantiates or executes anything named in the stream. It
//! enforces limits on nesting, object count and array lengths before allocating, rejects cyclic class chains and
//! dangling references, and requires the stream to end exactly after its single top-level object.
//!
//! Values of fields whose name marks them as secret (see [`is_secret_field`]) are **never kept as text**: the bytes
//! are skipped and only "present or empty" is recorded ([`Value::Redacted`]).

use crate::error::{ParseError, ParseErrorKind};

const MAGIC: u16 = 0xACED;
const VERSION: u16 = 5;
const BASE_HANDLE: i32 = 0x007E_0000;

const TC_NULL: u8 = 0x70;
const TC_REFERENCE: u8 = 0x71;
const TC_CLASSDESC: u8 = 0x72;
const TC_OBJECT: u8 = 0x73;
const TC_STRING: u8 = 0x74;
const TC_ARRAY: u8 = 0x75;
const TC_BLOCKDATA: u8 = 0x77;
const TC_ENDBLOCKDATA: u8 = 0x78;
const TC_BLOCKDATALONG: u8 = 0x7A;
const TC_LONGSTRING: u8 = 0x7C;
const TC_ENUM: u8 = 0x7E;

const SC_WRITE_METHOD: u8 = 0x01;
const SC_SERIALIZABLE: u8 = 0x02;
const SC_EXTERNALIZABLE: u8 = 0x04;

/// Limits applied while reading. Everything is checked before memory is reserved for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Largest accepted compressed input in bytes.
    pub max_compressed: usize,
    /// Largest accepted decompressed stream in bytes.
    pub max_decompressed: usize,
    /// Deepest nesting of objects and class descriptors.
    pub max_depth: usize,
    /// Most handles (strings, objects, class descriptors) and array/collection elements in one stream.
    pub max_handles: usize,
}

impl Limits {
    /// Limits for a repository auto-configuration file (small by nature: a few hundred bytes to a few kilobytes).
    pub const AUTOCONFIG: Self = Self {
        max_compressed: 1024 * 1024,
        max_decompressed: 4 * 1024 * 1024,
        max_depth: 100,
        max_handles: 50_000,
    };

    /// Limits for a repository's file tree (`.a3s/sync`) and `.a3s/serverinfo`. The tree grows with the number
    /// of files (roughly three handles per file); the values allow a few hundred thousand files and are **not
    /// yet measured** against large real repositories (`docs/migration/SYNC_METADATA.md`).
    pub const SYNC: Self = Self {
        max_compressed: 32 * 1024 * 1024,
        max_decompressed: 128 * 1024 * 1024,
        max_depth: 100,
        max_handles: 1_000_000,
    };
}

/// Whether a field name marks its string value as a secret that must not be kept.
#[must_use]
pub fn is_secret_field(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [
        "password",
        "passwd",
        "passwort",
        "login",
        "secret",
        "token",
        "credential",
        "key",
    ]
    .iter()
    .any(|part| lower.contains(part))
}

/// A value read from the stream.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    Null,
    Bool(bool),
    Int(i64),
    /// A primitive of a kind this reader does not interpret (floating point); its bytes were skipped.
    Other,
    Text(String),
    /// A secret string: only whether it was non-empty is kept.
    Redacted(bool),
    /// Index into [`Document::objects`] (objects, enum constants and arrays).
    Object(usize),
}

#[derive(Debug, Clone)]
pub(crate) struct ClassDesc {
    pub name: String,
    pub suid: i64,
    flags: u8,
    fields: Vec<(u8, String)>,
    sup: Option<usize>,
    complete: bool,
}

impl ClassDesc {
    /// The class version number as unsigned bits (the format stores it as a signed 64-bit value).
    pub fn suid_bits(&self) -> u64 {
        u64::from_be_bytes(self.suid.to_be_bytes())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Object {
    pub class: usize,
    pub fields: Vec<(String, Value)>,
    /// Objects written by a custom writer after the fields (the elements of a list); block data is skipped.
    pub items: Vec<Value>,
    /// The constant name if this is an enum constant.
    pub enum_name: Option<String>,
}

/// A fully read stream.
#[derive(Debug)]
pub(crate) struct Document {
    pub classes: Vec<ClassDesc>,
    pub objects: Vec<Object>,
    pub top: Value,
}

impl Document {
    pub fn object(&self, value: &Value) -> Option<&Object> {
        match value {
            Value::Object(index) => self.objects.get(*index),
            _ => None,
        }
    }

    pub fn class_of(&self, object: &Object) -> &ClassDesc {
        &self.classes[object.class]
    }

    /// Rejects the stream if it names any class or class version outside `known` (name, version bits).
    pub fn ensure_only(&self, known: &[(&str, u64)]) -> Result<(), ParseError> {
        let all_known = self.classes.iter().all(|class| {
            known
                .iter()
                .any(|(name, suid)| *name == class.name && *suid == class.suid_bits())
        });
        if all_known {
            Ok(())
        } else {
            Err(ParseError::new(
                ParseErrorKind::UnknownClass,
                "the file uses a class or class version that is not supported",
            ))
        }
    }
}

impl Object {
    pub fn field(&self, name: &str) -> Option<&Value> {
        self.fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value)
    }
}

enum Handle {
    Class(usize),
    Value(Value),
}

fn fail(kind: ParseErrorKind, detail: &'static str) -> ParseError {
    ParseError::new(kind, detail)
}

fn malformed(detail: &'static str) -> ParseError {
    fail(ParseErrorKind::Malformed, detail)
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    limits: &'a Limits,
    handles: Vec<Handle>,
    classes: Vec<ClassDesc>,
    objects: Vec<Object>,
    depth: usize,
    elements: usize,
}

impl<'a> Reader<'a> {
    fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], ParseError> {
        if n > self.remaining() {
            return Err(fail(
                ParseErrorKind::Truncated,
                "the stream ends unexpectedly",
            ));
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, ParseError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, ParseError> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn i32(&mut self) -> Result<i32, ParseError> {
        let b = self.take(4)?;
        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i64(&mut self) -> Result<i64, ParseError> {
        let b = self.take(8)?;
        let mut array = [0u8; 8];
        array.copy_from_slice(b);
        Ok(i64::from_be_bytes(array))
    }

    fn utf(&mut self) -> Result<String, ParseError> {
        let length = usize::from(self.u16()?);
        decode_modified_utf8(self.take(length)?)
    }

    fn new_handle(&mut self, handle: Handle) -> Result<(), ParseError> {
        if self.handles.len() >= self.limits.max_handles {
            return Err(fail(
                ParseErrorKind::LimitExceeded,
                "the stream contains too many objects",
            ));
        }
        self.handles.push(handle);
        Ok(())
    }

    fn count_element(&mut self) -> Result<(), ParseError> {
        self.elements += 1;
        if self.elements > self.limits.max_handles {
            return Err(fail(
                ParseErrorKind::LimitExceeded,
                "the stream contains too many elements",
            ));
        }
        Ok(())
    }

    fn enter(&mut self) -> Result<(), ParseError> {
        self.depth += 1;
        if self.depth > self.limits.max_depth {
            return Err(fail(
                ParseErrorKind::LimitExceeded,
                "the stream is nested too deeply",
            ));
        }
        Ok(())
    }

    fn content(&mut self, secret: bool) -> Result<Value, ParseError> {
        let tc = self.u8()?;
        self.content_by_code(tc, secret)
    }

    fn content_by_code(&mut self, tc: u8, secret: bool) -> Result<Value, ParseError> {
        self.enter()?;
        let result = self.content_inner(tc, secret);
        self.depth -= 1;
        result
    }

    fn content_inner(&mut self, tc: u8, secret: bool) -> Result<Value, ParseError> {
        match tc {
            TC_NULL => Ok(Value::Null),
            TC_REFERENCE => {
                let index = i64::from(self.i32()?) - i64::from(BASE_HANDLE);
                let value = usize::try_from(index)
                    .ok()
                    .and_then(|i| self.handles.get(i))
                    .and_then(|handle| match handle {
                        Handle::Value(value) => Some(value.clone()),
                        Handle::Class(_) => None,
                    })
                    .ok_or_else(|| malformed("a reference does not point to a known value"))?;
                Ok(if secret { redact(value) } else { value })
            }
            TC_STRING => {
                let length = usize::from(self.u16()?);
                self.string_of(length, secret)
            }
            TC_LONGSTRING => {
                let length = self.i64()?;
                let length = usize::try_from(length)
                    .ok()
                    .filter(|l| *l <= self.remaining())
                    .ok_or_else(|| malformed("a string length is outside the stream"))?;
                self.string_of(length, secret)
            }
            TC_OBJECT => self.object(),
            TC_ENUM => self.enum_constant(),
            TC_ARRAY => self.array(),
            _ => Err(malformed("the stream contains an unsupported element")),
        }
    }

    fn string_of(&mut self, length: usize, secret: bool) -> Result<Value, ParseError> {
        let bytes = self.take(length)?;
        let value = if secret {
            Value::Redacted(length > 0)
        } else {
            Value::Text(decode_modified_utf8(bytes)?)
        };
        self.new_handle(Handle::Value(value.clone()))?;
        Ok(value)
    }

    fn class_desc(&mut self) -> Result<Option<usize>, ParseError> {
        match self.u8()? {
            TC_NULL => Ok(None),
            TC_REFERENCE => {
                let index = i64::from(self.i32()?) - i64::from(BASE_HANDLE);
                match usize::try_from(index)
                    .ok()
                    .and_then(|i| self.handles.get(i))
                {
                    Some(Handle::Class(class)) => {
                        if self.classes[*class].complete {
                            Ok(Some(*class))
                        } else {
                            Err(malformed("a class descriptor refers to itself"))
                        }
                    }
                    _ => Err(malformed(
                        "a reference does not point to a class descriptor",
                    )),
                }
            }
            TC_CLASSDESC => {
                self.enter()?;
                let result = self.class_desc_body();
                self.depth -= 1;
                result.map(Some)
            }
            _ => Err(malformed("a class descriptor was expected")),
        }
    }

    fn required_class(&mut self) -> Result<usize, ParseError> {
        self.class_desc()?
            .ok_or_else(|| malformed("an object without a class descriptor"))
    }

    fn class_desc_body(&mut self) -> Result<usize, ParseError> {
        let name = self.utf()?;
        let suid = self.i64()?;
        let index = self.classes.len();
        self.classes.push(ClassDesc {
            name,
            suid,
            flags: 0,
            fields: Vec::new(),
            sup: None,
            complete: false,
        });
        self.new_handle(Handle::Class(index))?;
        let flags = self.u8()?;
        if flags & SC_EXTERNALIZABLE != 0 {
            return Err(fail(
                ParseErrorKind::UnknownClass,
                "externalizable classes are not supported",
            ));
        }
        let count = usize::from(self.u16()?);
        let mut fields = Vec::with_capacity(count.min(64));
        for _ in 0..count {
            let code = self.u8()?;
            if !matches!(
                code,
                b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b'L' | b'['
            ) {
                return Err(malformed("a field has an invalid type code"));
            }
            let field_name = self.utf()?;
            if matches!(code, b'L' | b'[') && !matches!(self.content(false)?, Value::Text(_)) {
                return Err(malformed("a field type must be a string"));
            }
            fields.push((code, field_name));
        }
        self.skip_annotation(None)?;
        let sup = self.class_desc()?;
        let desc = &mut self.classes[index];
        desc.flags = flags;
        desc.fields = fields;
        desc.sup = sup;
        desc.complete = true;
        Ok(index)
    }

    /// Reads annotation content up to the end marker. Block data is skipped; objects are read (they carry handles)
    /// and appended to `items` if given.
    fn skip_annotation(&mut self, mut items: Option<&mut Vec<Value>>) -> Result<(), ParseError> {
        loop {
            match self.u8()? {
                TC_ENDBLOCKDATA => return Ok(()),
                TC_BLOCKDATA => {
                    let length = usize::from(self.u8()?);
                    self.take(length)?;
                }
                TC_BLOCKDATALONG => {
                    let length = usize::try_from(self.i32()?)
                        .map_err(|_| malformed("a block length is negative"))?;
                    self.take(length)?;
                }
                tc => {
                    let value = self.content_by_code(tc, false)?;
                    if let Some(list) = items.as_deref_mut() {
                        self.count_element()?;
                        list.push(value);
                    }
                }
            }
        }
    }

    fn object(&mut self) -> Result<Value, ParseError> {
        let class = self.required_class()?;
        let index = self.objects.len();
        self.objects.push(Object {
            class,
            fields: Vec::new(),
            items: Vec::new(),
            enum_name: None,
        });
        self.new_handle(Handle::Value(Value::Object(index)))?;
        // Class data is written from the topmost superclass down to the object's own class.
        let mut chain = vec![class];
        let mut cursor = self.classes[class].sup;
        while let Some(next) = cursor {
            if chain.len() >= self.limits.max_depth {
                return Err(fail(
                    ParseErrorKind::LimitExceeded,
                    "a class chain is too long",
                ));
            }
            chain.push(next);
            cursor = self.classes[next].sup;
        }
        let mut fields = Vec::new();
        let mut items = Vec::new();
        for &class_index in chain.iter().rev() {
            let (flags, descriptors) = {
                let desc = &self.classes[class_index];
                (desc.flags, desc.fields.clone())
            };
            if flags & SC_SERIALIZABLE == 0 {
                continue;
            }
            for (code, name) in descriptors {
                let value = if matches!(code, b'L' | b'[') {
                    self.content(is_secret_field(&name))?
                } else {
                    self.primitive(code)?
                };
                fields.push((name, value));
            }
            if flags & SC_WRITE_METHOD != 0 {
                self.skip_annotation(Some(&mut items))?;
            }
        }
        let object = &mut self.objects[index];
        object.fields = fields;
        object.items = items;
        Ok(Value::Object(index))
    }

    fn primitive(&mut self, code: u8) -> Result<Value, ParseError> {
        Ok(match code {
            b'B' => Value::Int(i64::from(i8::from_be_bytes([self.u8()?]))),
            b'C' => Value::Int(i64::from(self.u16()?)),
            b'S' => {
                let b = self.take(2)?;
                Value::Int(i64::from(i16::from_be_bytes([b[0], b[1]])))
            }
            b'I' => Value::Int(i64::from(self.i32()?)),
            b'J' => Value::Int(self.i64()?),
            b'Z' => Value::Bool(self.u8()? != 0),
            b'F' => {
                self.take(4)?;
                Value::Other
            }
            b'D' => {
                self.take(8)?;
                Value::Other
            }
            _ => return Err(malformed("a field has an invalid type code")),
        })
    }

    fn enum_constant(&mut self) -> Result<Value, ParseError> {
        let class = self.required_class()?;
        let index = self.objects.len();
        self.objects.push(Object {
            class,
            fields: Vec::new(),
            items: Vec::new(),
            enum_name: None,
        });
        self.new_handle(Handle::Value(Value::Object(index)))?;
        match self.content(false)? {
            Value::Text(name) => {
                self.objects[index].enum_name = Some(name);
                Ok(Value::Object(index))
            }
            _ => Err(malformed("an enum constant needs a name")),
        }
    }

    fn array(&mut self) -> Result<Value, ParseError> {
        let class = self.required_class()?;
        let length =
            usize::try_from(self.i32()?).map_err(|_| malformed("an array length is negative"))?;
        let component = self.classes[class]
            .name
            .strip_prefix('[')
            .and_then(|rest| rest.bytes().next())
            .ok_or_else(|| malformed("an array class name is invalid"))?;
        let element_size = match component {
            b'B' | b'Z' => Some(1usize),
            b'C' | b'S' => Some(2),
            b'I' | b'F' => Some(4),
            b'J' | b'D' => Some(8),
            _ => None,
        };
        // Every element needs at least one byte, so a length beyond the rest of the stream is a lie.
        let needed = length.saturating_mul(element_size.unwrap_or(1));
        if needed > self.remaining() {
            return Err(fail(
                ParseErrorKind::LimitExceeded,
                "an array is longer than the stream",
            ));
        }
        let index = self.objects.len();
        self.objects.push(Object {
            class,
            fields: Vec::new(),
            items: Vec::new(),
            enum_name: None,
        });
        self.new_handle(Handle::Value(Value::Object(index)))?;
        if element_size.is_some() {
            self.take(needed)?;
        } else {
            let mut items = Vec::with_capacity(length.min(1024));
            for _ in 0..length {
                self.count_element()?;
                items.push(self.content(false)?);
            }
            self.objects[index].items = items;
        }
        Ok(Value::Object(index))
    }
}

fn redact(value: Value) -> Value {
    match value {
        Value::Text(text) => Value::Redacted(!text.is_empty()),
        other => other,
    }
}

/// Decodes the stream's modified UTF-8 (NUL as `C0 80`, supplementary characters as surrogate pairs).
fn decode_modified_utf8(bytes: &[u8]) -> Result<String, ParseError> {
    let bad = || malformed("a string is not valid text");
    let mut units: Vec<u16> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b & 0x80 == 0 {
            units.push(u16::from(b));
            i += 1;
        } else if b & 0xE0 == 0xC0 {
            let c = *bytes.get(i + 1).ok_or_else(bad)?;
            if c & 0xC0 != 0x80 {
                return Err(bad());
            }
            units.push((u16::from(b & 0x1F) << 6) | u16::from(c & 0x3F));
            i += 2;
        } else if b & 0xF0 == 0xE0 {
            let (c, d) = (
                *bytes.get(i + 1).ok_or_else(bad)?,
                *bytes.get(i + 2).ok_or_else(bad)?,
            );
            if c & 0xC0 != 0x80 || d & 0xC0 != 0x80 {
                return Err(bad());
            }
            units.push(
                (u16::from(b & 0x0F) << 12) | (u16::from(c & 0x3F) << 6) | u16::from(d & 0x3F),
            );
            i += 3;
        } else {
            return Err(bad());
        }
    }
    char::decode_utf16(units)
        .collect::<Result<String, _>>()
        .map_err(|_| bad())
}

/// Reads a decompressed serialization stream with exactly one top-level object.
pub(crate) fn read_stream(data: &[u8], limits: &Limits) -> Result<Document, ParseError> {
    let mut reader = Reader {
        data,
        pos: 0,
        limits,
        handles: Vec::new(),
        classes: Vec::new(),
        objects: Vec::new(),
        depth: 0,
        elements: 0,
    };
    if reader.u16().ok() != Some(MAGIC) || reader.u16().ok() != Some(VERSION) {
        return Err(fail(
            ParseErrorKind::NotSerialization,
            "the data is not a supported serialization stream",
        ));
    }
    let top = reader.content(false)?;
    if reader.remaining() != 0 {
        return Err(malformed("unexpected data after the top-level object"));
    }
    Ok(Document {
        classes: reader.classes,
        objects: reader.objects,
        top,
    })
}
