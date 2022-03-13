use arrayvec::ArrayString;
use core::marker::PhantomData;
use serde::{
    de::{Error, Unexpected, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{
    borrow::{Borrow, BorrowMut, Cow},
    convert::Infallible,
    ffi::OsStr,
    fmt,
    fmt::Write as FmtWrite,
    iter::FromIterator,
    mem,
    ops::{Deref, DerefMut},
    path::Path,
    str,
    str::{FromStr, Utf8Error},
    string::FromUtf8Error,
};

#[cfg(feature = "postgres_types")]
use bytes::BytesMut;
#[cfg(feature = "postgres_types")]
use postgres_types::{FromSql, IsNull, ToSql, Type};

#[cfg(feature = "rweb-openapi")]
use rweb::openapi::{
    ComponentDescriptor, ComponentOrInlineSchema, Entity, ResponseEntity, Responses,
};

#[cfg(feature = "rweb-openapi")]
use hyper::Body;

#[cfg(feature = "async_graphql")]
use async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value};

use crate::{StackString, MAX_INLINE};

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum SmallString<const CAP: usize> {
    Inline(ArrayString<CAP>),
    Boxed(String),
}

impl<const CAP: usize> Default for SmallString<CAP> {
    fn default() -> Self {
        Self::Inline(ArrayString::new())
    }
}

impl<const CAP: usize> SmallString<CAP> {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::Inline(ArrayString::new())
    }

    #[inline]
    #[must_use]
    pub fn is_inline(&self) -> bool {
        match self {
            Self::Inline(_) => true,
            Self::Boxed(_) => false,
        }
    }

    #[inline]
    #[must_use]
    pub fn is_boxed(&self) -> bool {
        !self.is_inline()
    }

    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Inline(s) => s.as_str(),
            Self::Boxed(s) => s.as_str(),
        }
    }

    #[inline]
    pub fn as_mut_str(&mut self) -> &mut str {
        match self {
            Self::Inline(s) => s.as_mut_str(),
            Self::Boxed(s) => s.as_mut_str(),
        }
    }

    pub fn from_utf8(v: &[u8]) -> Result<Self, Utf8Error> {
        str::from_utf8(v)
            .map(|s| ArrayString::from(s).map_or_else(|_| Self::Boxed(s.into()), Self::Inline))
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn from_utf8_vec(v: Vec<u8>) -> Result<Self, FromUtf8Error> {
        String::from_utf8(v).map(|s| {
            if s.len() > CAP {
                Self::Boxed(s)
            } else {
                let mut astr = ArrayString::new();
                astr.push_str(s.as_str());
                Self::Inline(astr)
            }
        })
    }

    pub fn from_byte_string(b: &[u8; CAP]) -> Result<Self, Utf8Error> {
        ArrayString::from_byte_string(b).map(Self::Inline)
    }

    #[must_use]
    pub fn from_utf8_lossy(v: &[u8]) -> Self {
        if v.len() > CAP {
            match String::from_utf8_lossy(v) {
                Cow::Borrowed(s) => s.into(),
                Cow::Owned(s) => s.into(),
            }
        } else {
            let (v, up_to, error_len) = match str::from_utf8(v) {
                Ok(s) => return s.into(),
                Err(error) => (v, error.valid_up_to(), error.error_len()),
            };
            let mut buf = ArrayString::new();
            let (valid, after_valid) = v.split_at(up_to);
            buf.push_str(unsafe { str::from_utf8_unchecked(valid) });
            buf.push('\u{FFFD}');
            let mut input = after_valid;
            if let Some(invalid_sequence_length) = error_len {
                input = &after_valid[invalid_sequence_length..];
            }
            loop {
                match str::from_utf8(input) {
                    Ok(s) => {
                        buf.push_str(s);
                        break;
                    }
                    Err(error) => {
                        let (valid, after_valid) = input.split_at(error.valid_up_to());
                        buf.push_str(unsafe { str::from_utf8_unchecked(valid) });
                        buf.push('\u{FFFD}');
                        if let Some(invalid_sequence_length) = error.error_len() {
                            input = &after_valid[invalid_sequence_length..];
                        } else {
                            break;
                        }
                    }
                }
            }
            buf.into()
        }
    }

    pub fn push_str(&mut self, s: &str) {
        match self {
            Self::Inline(a) => {
                if a.try_push_str(s).is_err() {
                    let mut buf: String = a.as_str().into();
                    buf.push_str(s);
                    let mut new_a = Self::Boxed(buf);
                    mem::swap(self, &mut new_a);
                }
            }
            Self::Boxed(a) => a.push_str(s),
        }
    }

    /// Split the string into two at the given index.
    ///
    /// Returns the content to the right of the index as a new string, and
    /// removes it from the original.
    ///
    /// If the index doesn't fall on a UTF-8 character boundary, this method
    /// panics.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn split_off(&mut self, index: usize) -> Self {
        match self {
            Self::Boxed(s) => s.split_off(index).into(),
            Self::Inline(s) => {
                let st = s.as_str();
                assert!(st.is_char_boundary(index));
                let result = st[index..].into();
                s.truncate(index);
                result
            }
        }
    }

    /// # Panics
    /// `from_display` panics if a formatting trait implementation returns an
    /// error. This indicates an incorrect implementation
    /// since `fmt::Write for String` never returns an error itself.
    pub fn from_display(buf: impl fmt::Display) -> Self {
        let mut s = Self::new();
        write!(s, "{buf}").unwrap();
        s
    }

    #[must_use]
    pub fn into_smallstring<const CAP1: usize>(self) -> SmallString<CAP1> {
        if self.len() > CAP1 {
            match self {
                SmallString::Boxed(s) => SmallString::Boxed(s),
                SmallString::Inline(s) => s.as_str().into(),
            }
        } else {
            self.as_str().into()
        }
    }
}

impl<const CAP: usize> From<&str> for SmallString<CAP> {
    fn from(item: &str) -> Self {
        ArrayString::from(item).map_or_else(|e| Self::Boxed(e.element().into()), Self::Inline)
    }
}

impl<const CAP: usize> Serialize for SmallString<CAP> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de, const CAP: usize> Deserialize<'de> for SmallString<CAP> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer
            .deserialize_string(SmartStringVisitor(PhantomData))
            .map(SmallString::from)
    }
}

struct SmartStringVisitor<const CAP: usize>(PhantomData<*const SmallString<CAP>>);

impl<'de, const CAP: usize> Visitor<'de> for SmartStringVisitor<CAP> {
    type Value = SmallString<CAP>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(SmallString::from(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(SmallString::from(v))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match str::from_utf8(v) {
            Ok(s) => Ok(s.into()),
            Err(_) => Err(Error::invalid_value(Unexpected::Bytes(v), &self)),
        }
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match String::from_utf8(v) {
            Ok(s) => Ok(s.into()),
            Err(e) => Err(Error::invalid_value(
                Unexpected::Bytes(&e.into_bytes()),
                &self,
            )),
        }
    }
}

impl<const CAP: usize> From<String> for SmallString<CAP> {
    fn from(item: String) -> Self {
        if item.len() > CAP {
            Self::Boxed(item)
        } else {
            SmallString::from(item.as_str())
        }
    }
}

impl<const CAP: usize> From<&String> for SmallString<CAP> {
    fn from(item: &String) -> Self {
        item.as_str().into()
    }
}

impl<const CAP: usize> From<ArrayString<CAP>> for SmallString<CAP> {
    fn from(item: ArrayString<CAP>) -> Self {
        Self::Inline(item)
    }
}

impl<const CAP: usize> From<SmallString<CAP>> for String {
    fn from(item: SmallString<CAP>) -> Self {
        match item {
            SmallString::Inline(s) => s.to_string(),
            SmallString::Boxed(s) => s,
        }
    }
}

impl<const CAP: usize> From<&SmallString<CAP>> for String {
    fn from(item: &SmallString<CAP>) -> Self {
        item.to_string()
    }
}

impl<const CAP: usize> From<&SmallString<CAP>> for SmallString<CAP> {
    fn from(item: &SmallString<CAP>) -> Self {
        item.clone()
    }
}

impl<'a, const CAP: usize> From<&'a SmallString<CAP>> for &'a str {
    fn from(item: &SmallString<CAP>) -> &str {
        item.as_str()
    }
}

impl<'a, const CAP: usize> From<Cow<'a, str>> for SmallString<CAP> {
    fn from(item: Cow<'a, str>) -> Self {
        match item {
            Cow::Borrowed(s) => s.into(),
            Cow::Owned(s) => s.into(),
        }
    }
}

impl<const CAP: usize> From<SmallString<CAP>> for Cow<'_, str> {
    fn from(item: SmallString<CAP>) -> Self {
        Cow::Owned(item.into())
    }
}

impl<const CAP: usize> From<StackString> for SmallString<CAP> {
    fn from(item: StackString) -> Self {
        if item.len() > CAP {
            let s: String = item.into();
            Self::Boxed(s)
        } else {
            Self::Inline(ArrayString::from(item.as_str()).unwrap())
        }
    }
}

impl<const CAP: usize> From<&StackString> for SmallString<CAP> {
    fn from(item: &StackString) -> Self {
        SmallString::from(item.as_str())
    }
}

impl<const CAP: usize> From<SmallString<CAP>> for StackString {
    fn from(item: SmallString<CAP>) -> Self {
        if item.len() > MAX_INLINE {
            let s: String = item.into();
            s.into()
        } else {
            StackString::from(item.as_str())
        }
    }
}

impl<const CAP: usize> Borrow<str> for SmallString<CAP> {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<const CAP: usize> BorrowMut<str> for SmallString<CAP> {
    fn borrow_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<const CAP: usize> fmt::Display for SmallString<CAP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl<const CAP: usize> fmt::Write for SmallString<CAP> {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        self.push_str(s);
        Ok(())
    }
}

impl<const CAP: usize> AsRef<str> for SmallString<CAP> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<const CAP: usize> AsRef<[u8]> for SmallString<CAP> {
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_ref()
    }
}

impl<const CAP: usize> AsRef<OsStr> for SmallString<CAP> {
    fn as_ref(&self) -> &OsStr {
        self.as_str().as_ref()
    }
}

impl<const CAP: usize> AsRef<Path> for SmallString<CAP> {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl<const CAP: usize> FromStr for SmallString<CAP> {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.into())
    }
}

impl<const CAP: usize> Deref for SmallString<CAP> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<const CAP: usize> DerefMut for SmallString<CAP> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_str()
    }
}

impl<'a, const CAP: usize> PartialEq<Cow<'a, str>> for SmallString<CAP> {
    #[inline]
    fn eq(&self, other: &Cow<'a, str>) -> bool {
        PartialEq::eq(&self[..], &other[..])
    }
}

impl<const CAP: usize> PartialEq<String> for SmallString<CAP> {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        PartialEq::eq(&self[..], &other[..])
    }
}

impl<const CAP: usize> PartialEq<str> for SmallString<CAP> {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        PartialEq::eq(self.as_str(), other)
    }
}

impl<const CAP: usize> PartialEq<&str> for SmallString<CAP> {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        PartialEq::eq(&self.as_str(), other)
    }
}

#[cfg(feature = "postgres_types")]
impl<'a, const CAP: usize> FromSql<'a> for SmallString<CAP> {
    fn from_sql(
        ty: &Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let s = <&'a str as FromSql>::from_sql(ty, raw)?;
        Ok(s.into())
    }

    fn accepts(ty: &Type) -> bool {
        <&'a str as FromSql>::accepts(ty)
    }
}

#[cfg(feature = "postgres_types")]
impl<const CAP: usize> ToSql for SmallString<CAP> {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>>
    where
        Self: Sized,
    {
        ToSql::to_sql(&self.as_str(), ty, out)
    }

    fn accepts(ty: &Type) -> bool
    where
        Self: Sized,
    {
        <String as ToSql>::accepts(ty)
    }

    fn to_sql_checked(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        self.as_str().to_sql_checked(ty, out)
    }
}

#[cfg(feature = "rweb-openapi")]
impl<const CAP: usize> Entity for SmallString<CAP> {
    fn type_name() -> Cow<'static, str> {
        str::type_name()
    }

    #[inline]
    fn describe(comp_d: &mut ComponentDescriptor) -> ComponentOrInlineSchema {
        str::describe(comp_d)
    }
}

#[cfg(feature = "rweb-openapi")]
impl<const CAP: usize> ResponseEntity for SmallString<CAP> {
    #[inline]
    fn describe_responses(comp_d: &mut ComponentDescriptor) -> Responses {
        String::describe_responses(comp_d)
    }
}

#[cfg(feature = "rweb-openapi")]
impl<const CAP: usize> From<SmallString<CAP>> for Body {
    #[inline]
    fn from(s: SmallString<CAP>) -> Body {
        let s: String = s.into();
        Body::from(s)
    }
}

impl<const CAP: usize> FromIterator<char> for SmallString<CAP> {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (min, max) = iter.size_hint();
        let size = if let Some(x) = max { x } else { min };
        let mut s = if size > CAP {
            Self::Boxed(String::with_capacity(size))
        } else {
            Self::Inline(ArrayString::<CAP>::new())
        };
        for c in iter {
            s.write_char(c).unwrap();
        }
        s
    }
}

/// Allow SmallString to be used as graphql scalar value
#[cfg(feature = "async_graphql")]
#[Scalar]
impl<const CAP: usize> ScalarType for SmallString<CAP> {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(s) = value {
            let s: Self = s.into();
            Ok(s)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn is_valid(value: &Value) -> bool {
        matches!(value, Value::String(_))
    }

    fn to_value(&self) -> Value {
        Value::String(self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use arrayvec::ArrayString;
    use rand::{thread_rng, Rng};
    use std::fmt::Write;

    use crate::{small_string::SmallString, stack_string::StackString};

    #[test]
    fn test_default() {
        assert_eq!(SmallString::<1>::new(), SmallString::<1>::default());
    }

    #[test]
    fn test_sizeof() {
        if std::mem::size_of::<String>() == 24 {
            assert_eq!(std::mem::size_of::<StackString>(), 24);
            assert_eq!(std::mem::size_of::<SmallString<32>>(), 40);
            assert_eq!(std::mem::size_of::<SmallString<30>>(), 40);
            assert_eq!(std::mem::size_of::<ArrayString<32>>(), 36);
            assert_eq!(std::mem::size_of::<[u8; 32]>(), 32);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_small_string_split_off() {
        let mut s0 = "hello there".to_string();
        let s1 = s0.split_off(3);
        let mut s2: SmallString<20> = "hello there".into();
        let s3 = s2.split_off(3);
        assert_eq!(s0.as_str(), s2.as_str());
        assert_eq!(s1.as_str(), s3.as_str());
        assert!(s2.is_inline());
        assert!(s3.is_inline());
    }

    #[test]
    fn test_from_utf8() {
        let mut rng = thread_rng();
        let v: Vec<_> = (0..20).map(|_| rng.gen::<u8>() & 0x7f).collect();
        let s0 = std::str::from_utf8(&v).unwrap();
        let s1 = SmallString::<20>::from_utf8(&v).unwrap();
        assert_eq!(s0, s1.as_str());
        assert!(s1.is_inline());

        let v: Vec<_> = (0..20).map(|_| rng.gen::<u8>()).collect();
        let s0 = std::str::from_utf8(&v);
        let s1 = SmallString::<20>::from_utf8(&v);

        match s0 {
            Ok(s) => assert_eq!(s, s1.unwrap().as_str()),
            Err(e) => assert_eq!(e, s1.unwrap_err()),
        }
    }

    #[test]
    fn test_string_from_smallstring() {
        let s0 = SmallString::<20>::from("Hello there");
        let s1: String = s0.clone().into();
        assert_eq!(s0.as_str(), s1.as_str());
    }

    #[test]
    fn test_smallstring_from_string() {
        let s0 = String::from("Hello there");
        let s1: SmallString<20> = s0.clone().into();
        assert_eq!(s0.as_str(), s1.as_str());
        let s1: SmallString<20> = (&s0).into();
        assert_eq!(s0.as_str(), s1.as_str());
    }

    #[test]
    fn test_borrow() {
        use std::borrow::Borrow;
        let s = SmallString::<20>::from("Hello");
        let st: &str = s.borrow();
        assert_eq!(st, "Hello");
    }

    #[test]
    fn test_as_ref() {
        use std::path::Path;
        let s = SmallString::<20>::from("Hello");
        let st: &str = s.as_ref();
        assert_eq!(st, s.as_str());
        let bt: &[u8] = s.as_ref();
        assert_eq!(bt, s.as_bytes());
        let pt: &Path = s.as_ref();
        assert_eq!(pt, Path::new("Hello"));
    }

    #[test]
    fn test_from_str() {
        let s = SmallString::<20>::from("Hello");
        let st: SmallString<20> = "Hello".parse().unwrap();
        assert_eq!(s, st);
    }

    #[test]
    fn test_partialeq_cow() {
        use std::path::Path;
        let p = Path::new("Hello");
        let ps = p.to_string_lossy();
        let s = SmallString::<20>::from("Hello");
        assert_eq!(s, ps);
    }

    #[test]
    fn test_partial_eq_string() {
        assert_eq!(SmallString::<20>::from("Hello"), String::from("Hello"));
        assert_eq!(SmallString::<20>::from("Hello"), "Hello");
        assert_eq!(&SmallString::<20>::from("Hello"), "Hello");
    }

    #[test]
    fn test_from_iterator_char() {
        let mut rng = thread_rng();
        let v: Vec<char> = (0..20).map(|_| rng.gen::<char>()).collect();
        let s0: SmallString<20> = v.iter().map(|x| *x).collect();
        let s1: String = v.iter().map(|x| *x).collect();
        assert_eq!(s0, s1);
    }

    #[test]
    fn test_contains_smallstring() {
        let a: SmallString<20> = "hey there".into();
        let b: SmallString<20> = "hey".into();
        assert!(a.contains(b.as_str()));
    }

    #[test]
    fn test_contains_char() {
        let a: SmallString<20> = "hey there".into();
        assert!(a.contains(' '));
    }

    #[test]
    fn test_equality() {
        let s: SmallString<20> = "hey".into();
        assert_eq!(Some(&s).map(Into::into), Some("hey"));
    }

    #[cfg(feature = "postgres_types")]
    use bytes::BytesMut;
    #[cfg(feature = "postgres_types")]
    use postgres_types::{FromSql, IsNull, ToSql, Type};

    #[cfg(feature = "postgres_types")]
    #[test]
    fn test_from_sql() {
        let raw = b"Hello There";
        let t = Type::TEXT;
        let s = SmallString::<20>::from_sql(&t, raw).unwrap();
        assert_eq!(s, SmallString::<20>::from("Hello There"));

        assert!(<SmallString<20> as FromSql>::accepts(&t));
    }

    #[cfg(feature = "postgres_types")]
    #[test]
    fn test_to_sql() {
        let s = SmallString::<20>::from("Hello There");
        let t = Type::TEXT;
        assert!(<SmallString<20> as ToSql>::accepts(&t));
        let mut buf = BytesMut::new();
        match s.to_sql(&t, &mut buf).unwrap() {
            IsNull::Yes => assert!(false),
            IsNull::No => {}
        }
        assert_eq!(buf.as_ref(), b"Hello There");
    }

    #[test]
    fn test_from_display() {
        use std::fmt::Display;

        struct Test {}

        impl Display for Test {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("THIS IS A TEST")
            }
        }

        let t = Test {};
        let s = SmallString::<20>::from_display(t);
        assert_eq!(s, SmallString::<20>::from("THIS IS A TEST"));
    }

    #[test]
    fn test_write_smallstring() {
        let mut s = SmallString::<5>::new();
        write!(&mut s, "12345").unwrap();
        assert_eq!(s.as_str(), "12345");
        assert!(s.is_inline());

        let mut s = SmallString::<5>::new();
        write!(&mut s, "123456789").unwrap();
        assert_eq!(s.as_str(), "123456789");
        assert!(s.is_boxed());
    }

    #[test]
    fn test_into_smallstring() {
        let mut s = SmallString::<10>::new();
        write!(&mut s, "123456789").unwrap();
        assert!(s.is_inline());
        let s = s.into_smallstring::<20>();
        assert!(s.is_inline());
        let s = s.into_smallstring::<5>();
        assert!(s.is_boxed());
    }

    #[test]
    fn test_serde() {
        use serde::Deserialize;

        let s = SmallString::<30>::from("HELLO");
        let t = "HELLO";
        let s = serde_json::to_vec(&s).unwrap();
        let t = serde_json::to_vec(t).unwrap();
        assert_eq!(s, t);

        let s = r#"{"a": "b"}"#;

        #[derive(Deserialize)]
        struct A {
            a: SmallString<30>,
        }

        #[derive(Deserialize)]
        struct B {
            a: String,
        }

        let a: A = serde_json::from_str(s).unwrap();
        let b: B = serde_json::from_str(s).unwrap();
        assert_eq!(a.a.as_str(), b.a.as_str());
    }

    #[cfg(feature = "async_graphql")]
    #[test]
    fn test_smallstring_async_graphql() {
        use async_graphql::{
            dataloader::{DataLoader, Loader},
            Context, EmptyMutation, EmptySubscription, Object, Schema,
        };
        use async_trait::async_trait;
        use std::{collections::HashMap, convert::Infallible};

        struct SmallStringLoader;

        impl SmallStringLoader {
            fn new() -> Self {
                Self
            }
        }

        #[async_trait]
        impl<const CAP: usize> Loader<SmallString<CAP>> for SmallStringLoader {
            type Value = SmallString<CAP>;
            type Error = Infallible;

            async fn load(
                &self,
                _: &[SmallString<CAP>],
            ) -> Result<HashMap<SmallString<CAP>, Self::Value>, Self::Error> {
                let mut m = HashMap::new();
                m.insert("HELLO".into(), "WORLD".into());
                Ok(m)
            }
        }

        struct QueryRoot<const CAP: usize>;

        #[Object]
        impl<const CAP: usize, 'a> QueryRoot<CAP> {
            async fn hello(
                &self,
                ctx: &Context<'a>,
            ) -> Result<Option<SmallString<CAP>>, Infallible> {
                let hello = ctx
                    .data::<DataLoader<SmallStringLoader>>()
                    .unwrap()
                    .load_one("hello".into())
                    .await
                    .unwrap();
                Ok(hello)
            }
        }

        let expected_sdl = include_str!("../tests/data/sdl_file_smallstring.txt");

        let schema = Schema::build(QueryRoot::<5>, EmptyMutation, EmptySubscription)
            .data(DataLoader::new(
                SmallStringLoader::new(),
                tokio::task::spawn,
            ))
            .finish();
        let sdl = schema.sdl();
        assert_eq!(&sdl, expected_sdl);
    }
}
