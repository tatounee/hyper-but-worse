use std::fmt;
use std::io::{self, Write};

use serde::{
    Serialize, Serializer,
    ser::{self, Impossible},
};
use thiserror::Error as ThisError;

/// Serialize a value into the field format of a server sent event.
///
/// Every field of the serialized value is written as `<field>:<value>`, one
/// field per line. A value spanning several lines is repeated with its field
/// name on each of them, so `message: "hello\nworld"` is written as
/// `message:hello\nmessage:world\n`. A sequence follows the same rule: one
/// line per item.
///
/// Only a struct or a map can be serialized: they are what carries the field
/// names. Any other top level type is a [`SSError::NotSupported`].
pub(super) struct SSESerializer<W> {
    buf: W,
}

impl<W> SSESerializer<W>
where
    W: Write,
{
    pub fn new(buf: W) -> Self {
        Self { buf }
    }

    pub fn into_inner(self) -> W {
        self.buf
    }
}

/// Write `value` as one `<field>:<line>` line per line of `value`.
fn write_field<W>(buf: &mut W, field: &str, value: &str) -> Result<(), SSError>
where
    W: Write,
{
    for line in value.split('\n') {
        // `value` may use CRLF as line separator, the CR is part of the
        // separator and not of the line.
        writeln!(buf, "{field}:{}", line.trim_suffix('\r'))?;
    }

    Ok(())
}

/// Implement the serialization of the types a SSE field cannot hold.
macro_rules! not_supported {
    ($($method:ident($ty:ty) => $name:literal,)*) => {
        $(
            fn $method(self, _v: $ty) -> Result<Self::Ok, Self::Error> {
                Err(SSError::NotSupported($name))
            }
        )*
    };
}

impl<'a, W> Serializer for &'a mut SSESerializer<W>
where
    W: Write,
{
    type Ok = ();

    type Error = SSError;

    type SerializeSeq = Impossible<(), SSError>;

    type SerializeTuple = Impossible<(), SSError>;

    type SerializeTupleStruct = Impossible<(), SSError>;

    type SerializeTupleVariant = Impossible<(), SSError>;

    type SerializeMap = MapSerializer<'a, W>;

    type SerializeStruct = FieldsSerializer<'a, W>;

    type SerializeStructVariant = FieldsSerializer<'a, W>;

    not_supported! {
        serialize_bool(bool) => "a boolean",
        serialize_i8(i8) => "an integer",
        serialize_i16(i16) => "an integer",
        serialize_i32(i32) => "an integer",
        serialize_i64(i64) => "an integer",
        serialize_u8(u8) => "an integer",
        serialize_u16(u16) => "an integer",
        serialize_u32(u32) => "an integer",
        serialize_u64(u64) => "an integer",
        serialize_f32(f32) => "a float",
        serialize_f64(f64) => "a float",
        serialize_char(char) => "a character",
        serialize_str(&str) => "a string",
        serialize_bytes(&[u8]) => "bytes",
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(SSError::NotSupported("none"))
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(SSError::NotSupported("a unit"))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(SSError::NotSupported("a unit struct"))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(SSError::NotSupported("a unit variant"))
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    /// The variant names the field, as in `Data(&str)` giving `Data:<value>`.
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(FieldSerializer {
            buf: &mut self.buf,
            field: variant,
        })
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(SSError::NotSupported("a sequence"))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(SSError::NotSupported("a tuple"))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(SSError::NotSupported("a tuple struct"))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(SSError::NotSupported("a tuple variant"))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer {
            buf: &mut self.buf,
            field: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(FieldsSerializer { buf: &mut self.buf })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(FieldsSerializer { buf: &mut self.buf })
    }
}

/// Write the fields of a struct, one `<field>:<value>` line each.
pub struct FieldsSerializer<'a, W> {
    buf: &'a mut W,
}

impl<W> ser::SerializeStruct for FieldsSerializer<'_, W>
where
    W: Write,
{
    type Ok = ();

    type Error = SSError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(FieldSerializer {
            buf: &mut *self.buf,
            field: key,
        })
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W> ser::SerializeStructVariant for FieldsSerializer<'_, W>
where
    W: Write,
{
    type Ok = ();

    type Error = SSError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

/// Write the entries of a map, the key naming the field.
pub struct MapSerializer<'a, W> {
    buf: &'a mut W,
    field: Option<String>,
}

impl<W> ser::SerializeMap for MapSerializer<'_, W>
where
    W: Write,
{
    type Ok = ();

    type Error = SSError;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.field = Some(key.serialize(FieldNameSerializer)?);

        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let field = self
            .field
            .take()
            .ok_or(SSError::Custom("value serialized before its key".into()))?;

        value.serialize(FieldSerializer {
            buf: &mut *self.buf,
            field: &field,
        })
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

/// Turn a map key into a field name.
struct FieldNameSerializer;

impl Serializer for FieldNameSerializer {
    type Ok = String;

    type Error = SSError;

    type SerializeSeq = Impossible<String, SSError>;

    type SerializeTuple = Impossible<String, SSError>;

    type SerializeTupleStruct = Impossible<String, SSError>;

    type SerializeTupleVariant = Impossible<String, SSError>;

    type SerializeMap = Impossible<String, SSError>;

    type SerializeStruct = Impossible<String, SSError>;

    type SerializeStructVariant = Impossible<String, SSError>;

    not_supported! {
        serialize_bool(bool) => "a boolean field name",
        serialize_f32(f32) => "a float field name",
        serialize_f64(f64) => "a float field name",
        serialize_bytes(&[u8]) => "a bytes field name",
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_owned())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(SSError::NotSupported("an empty field name"))
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(SSError::NotSupported("an empty field name"))
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(name.to_owned())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(variant.to_owned())
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(SSError::NotSupported("a newtype variant field name"))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(SSError::NotSupported("a sequence field name"))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(SSError::NotSupported("a tuple field name"))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(SSError::NotSupported("a tuple struct field name"))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(SSError::NotSupported("a tuple variant field name"))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(SSError::NotSupported("a map field name"))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(SSError::NotSupported("a struct field name"))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(SSError::NotSupported("a struct variant field name"))
    }
}

/// Write the value of a single field, prefixing every of its lines with
/// `field`.
struct FieldSerializer<'a, W> {
    buf: &'a mut W,
    field: &'a str,
}

/// Implement the serialization of the values holding no line separator.
macro_rules! serialize_inline {
    ($($method:ident($ty:ty),)*) => {
        $(
            fn $method(self, v: $ty) -> Result<Self::Ok, Self::Error> {
                writeln!(self.buf, "{}:{v}", self.field)?;

                Ok(())
            }
        )*
    };
}

impl<'a, W> Serializer for FieldSerializer<'a, W>
where
    W: Write,
{
    type Ok = ();

    type Error = SSError;

    type SerializeSeq = LinesSerializer<'a, W>;

    type SerializeTuple = LinesSerializer<'a, W>;

    type SerializeTupleStruct = LinesSerializer<'a, W>;

    type SerializeTupleVariant = Impossible<(), SSError>;

    type SerializeMap = Impossible<(), SSError>;

    type SerializeStruct = Impossible<(), SSError>;

    type SerializeStructVariant = Impossible<(), SSError>;

    serialize_inline! {
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_f32(f32),
        serialize_f64(f64),
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        write_field(self.buf, self.field, v.encode_utf8(&mut [0; 4]))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        write_field(self.buf, self.field, v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let value = str::from_utf8(v).map_err(|_| SSError::NotSupported("non UTF-8 bytes"))?;

        write_field(self.buf, self.field, value)
    }

    /// An absent value is an absent field, not an empty one.
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        write_field(self.buf, self.field, variant)
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(LinesSerializer {
            buf: self.buf,
            field: self.field,
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(SSError::NotSupported("a tuple variant in a field"))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(SSError::NotSupported("a map in a field"))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(SSError::NotSupported("a struct in a field"))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(SSError::NotSupported("a struct variant in a field"))
    }
}

/// Write every item of a sequence on its own `<field>:<value>` line.
struct LinesSerializer<'a, W> {
    buf: &'a mut W,
    field: &'a str,
}

impl<W> LinesSerializer<'_, W>
where
    W: Write,
{
    fn serialize_line<T>(&mut self, value: &T) -> Result<(), SSError>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(FieldSerializer {
            buf: &mut *self.buf,
            field: self.field,
        })
    }
}

impl<W> ser::SerializeSeq for LinesSerializer<'_, W>
where
    W: Write,
{
    type Ok = ();

    type Error = SSError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.serialize_line(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W> ser::SerializeTuple for LinesSerializer<'_, W>
where
    W: Write,
{
    type Ok = ();

    type Error = SSError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.serialize_line(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W> ser::SerializeTupleStruct for LinesSerializer<'_, W>
where
    W: Write,
{
    type Ok = ();

    type Error = SSError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.serialize_line(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

#[derive(Debug, ThisError)]
pub enum SSError {
    #[error("{0} cannot be serialized as a server sent event")]
    NotSupported(&'static str),
    #[error("writing the server sent event")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Custom(String),
}

impl ser::Error for SSError {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::Custom(msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::ser::SerializeStruct;

    use super::{SSESerializer, SSError};
    use serde::Serialize;

    fn serialize<T>(value: &T) -> Result<String, SSError>
    where
        T: Serialize,
    {
        let mut serializer = SSESerializer::new(Vec::new());
        value.serialize(&mut serializer)?;

        Ok(String::from_utf8(serializer.into_inner()).unwrap())
    }

    struct Notification {
        event: Option<&'static str>,
        message: &'static str,
        retry: u32,
    }

    impl Serialize for Notification {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut event = serializer.serialize_struct("Notification", 3)?;
            event.serialize_field("event", &self.event)?;
            event.serialize_field("message", &self.message)?;
            event.serialize_field("retry", &self.retry)?;
            event.end()
        }
    }

    #[test]
    fn one_line_per_field() {
        let notification = Notification {
            event: Some("ping"),
            message: "hello",
            retry: 3000,
        };

        assert_eq!(
            serialize(&notification).unwrap(),
            "event:ping\nmessage:hello\nretry:3000\n"
        );
    }

    #[test]
    fn multiline_value_repeats_its_field() {
        let notification = Notification {
            event: None,
            message: "hello\nworld",
            retry: 3000,
        };

        assert_eq!(
            serialize(&notification).unwrap(),
            "message:hello\nmessage:world\nretry:3000\n"
        );
    }

    #[test]
    fn crlf_is_a_line_separator() {
        let map = BTreeMap::from([("data", "hello\r\nworld")]);

        assert_eq!(serialize(&map).unwrap(), "data:hello\ndata:world\n");
    }

    #[test]
    fn sequence_is_one_line_per_item() {
        let map = BTreeMap::from([("data", vec!["hello", "world\n!"])]);

        assert_eq!(serialize(&map).unwrap(), "data:hello\ndata:world\ndata:!\n");
    }

    #[test]
    fn a_field_less_value_is_rejected() {
        assert!(matches!(serialize(&"hello"), Err(SSError::NotSupported(_))));
    }
}
