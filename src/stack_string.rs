use derive_more::{Deref, DerefMut, Display, From, Index, IndexMut, Into};
use serde::{Deserialize, Serialize};
use smartstring::alias::String as SmartString;
use std::{
    borrow::{Borrow, BorrowMut, Cow},
    convert::Infallible,
    ffi::OsStr,
    fmt::{self, Write as FmtWrite},
    iter::FromIterator,
    path::Path,
    str,
    str::{FromStr, Utf8Error},
    string::FromUtf8Error,
};

use crate::MAX_INLINE;

#[cfg(feature = "diesel_types")]
use diesel::{
    backend::Backend,
    deserialize::{FromSql as DeFromSql, Result as DeResult},
    serialize::{Output, Result as SerResult, ToSql as DeToSql},
    sql_types::Text,
};

#[cfg(feature = "diesel_types")]
use std::io::Write;

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

#[derive(
    Display,
    Serialize,
    Deserialize,
    Deref,
    DerefMut,
    Index,
    IndexMut,
    Debug,
    Clone,
    Into,
    From,
    PartialEq,
    Eq,
    Hash,
    Default,
    PartialOrd,
    Ord,
)]
#[cfg_attr(feature = "diesel_types", derive(FromSqlRow, AsExpression))]
#[cfg_attr(feature = "diesel_types", sql_type = "Text")]
pub struct StackString(SmartString);

impl StackString {
    pub fn new() -> Self {
        Self(SmartString::new())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn split_off(&mut self, index: usize) -> Self {
        Self(self.0.split_off(index))
    }

    pub fn from_utf8(v: &[u8]) -> Result<Self, Utf8Error> {
        str::from_utf8(v).map(Into::into)
    }

    pub fn from_utf8_vec(vec: Vec<u8>) -> Result<Self, FromUtf8Error> {
        String::from_utf8(vec).map(Into::into)
    }

    pub fn from_utf8_lossy(v: &[u8]) -> Self {
        if v.len() > MAX_INLINE {
            match String::from_utf8_lossy(v) {
                Cow::Borrowed(s) => s.into(),
                Cow::Owned(s) => s.into(),
            }
        } else {
            let (v, up_to, error_len) = match str::from_utf8(v) {
                Ok(s) => return s.into(),
                Err(error) => (v, error.valid_up_to(), error.error_len()),
            };
            let mut buf = StackString::new();
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
            buf
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
}

impl From<StackString> for String {
    fn from(item: StackString) -> Self {
        item.0.into()
    }
}

impl From<&StackString> for String {
    fn from(item: &StackString) -> Self {
        item.as_str().into()
    }
}

impl From<&StackString> for StackString {
    fn from(item: &StackString) -> Self {
        item.clone()
    }
}

impl From<String> for StackString {
    fn from(item: String) -> Self {
        Self(item.into())
    }
}

impl From<&String> for StackString {
    fn from(item: &String) -> Self {
        Self(item.into())
    }
}

impl From<&str> for StackString {
    fn from(item: &str) -> Self {
        Self(item.into())
    }
}

impl<'a> From<&'a StackString> for &'a str {
    fn from(item: &StackString) -> &str {
        item.as_str()
    }
}

impl<'a> From<Cow<'a, str>> for StackString {
    fn from(item: Cow<'a, str>) -> Self {
        match item {
            Cow::Borrowed(s) => s.into(),
            Cow::Owned(s) => s.into(),
        }
    }
}

impl From<StackString> for Cow<'_, str> {
    fn from(item: StackString) -> Self {
        Cow::Owned(item.into())
    }
}

impl Borrow<str> for StackString {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl BorrowMut<str> for StackString {
    fn borrow_mut(&mut self) -> &mut str {
        self.0.borrow_mut()
    }
}

impl AsRef<str> for StackString {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<[u8]> for StackString {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsRef<OsStr> for StackString {
    fn as_ref(&self) -> &OsStr {
        self.as_str().as_ref()
    }
}

impl AsRef<Path> for StackString {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl FromStr for StackString {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.into())
    }
}

impl<'a> PartialEq<Cow<'a, str>> for StackString {
    #[inline]
    fn eq(&self, other: &Cow<'a, str>) -> bool {
        PartialEq::eq(&self[..], &other[..])
    }
}

impl<'a> PartialEq<String> for StackString {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        PartialEq::eq(&self[..], &other[..])
    }
}

impl<'a> PartialEq<str> for StackString {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        PartialEq::eq(&self.0, other)
    }
}

impl<'a> PartialEq<&'a str> for StackString {
    #[inline]
    fn eq(&self, other: &&'a str) -> bool {
        PartialEq::eq(&self[..], &other[..])
    }
}

impl FromIterator<char> for StackString {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        let s = SmartString::from_iter(iter);
        Self(s)
    }
}

#[cfg(feature = "postgres_types")]
impl<'a> FromSql<'a> for StackString {
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
impl ToSql for StackString {
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

#[cfg(feature = "diesel_types")]
impl<ST, DB> DeFromSql<ST, DB> for StackString
where
    DB: Backend,
    *const str: DeFromSql<ST, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> DeResult<Self> {
        let str_ptr = <*const str as DeFromSql<ST, DB>>::from_sql(bytes)?;
        let s = unsafe { &*str_ptr };
        Ok(s.into())
    }
}

#[cfg(feature = "diesel_types")]
impl<DB> DeToSql<Text, DB> for StackString
where
    DB: Backend,
    str: DeToSql<Text, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> SerResult {
        self.as_str().to_sql(out)
    }
}

#[cfg(feature = "rweb-openapi")]
impl Entity for StackString {
    fn type_name() -> Cow<'static, str> {
        <str as Entity>::type_name()
    }

    #[inline]
    fn describe(comp_d: &mut ComponentDescriptor) -> ComponentOrInlineSchema {
        str::describe(comp_d)
    }
}

#[cfg(feature = "rweb-openapi")]
impl ResponseEntity for StackString {
    #[inline]
    fn describe_responses(comp_d: &mut ComponentDescriptor) -> Responses {
        String::describe_responses(comp_d)
    }
}

#[cfg(feature = "rweb-openapi")]
impl From<StackString> for Body {
    #[inline]
    fn from(s: StackString) -> Body {
        let s: String = s.into();
        Body::from(s)
    }
}

#[macro_export]
macro_rules! format_sstr {
    ($($arg:tt)*) => {{
        let mut buf = $crate::StackString::new();
        std::write!(buf, "{}", std::format_args!($($arg)*)).unwrap();
        buf
    }}
}

/// Allow StackString to be used as graphql scalar value
#[cfg(feature = "async_graphql")]
#[Scalar]
impl ScalarType for StackString {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(s) = value {
            let s: StackString = s.into();
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
    use rand::{thread_rng, Rng};

    use crate::StackString;

    #[test]
    fn test_default() {
        assert_eq!(StackString::new(), StackString::default());
    }

    #[test]
    fn test_split_off() {
        let mut s0 = "hello there".to_string();
        let s1 = s0.split_off(3);
        let mut s2: StackString = "hello there".into();
        let s3 = s2.split_off(3);
        assert_eq!(s0.as_str(), s2.as_str());
        assert_eq!(s1.as_str(), s3.as_str());
    }

    #[test]
    fn test_from_utf8() {
        let mut rng = thread_rng();
        let v: Vec<_> = (0..20).map(|_| rng.gen::<u8>() & 0x7f).collect();
        let s0 = String::from_utf8(v.clone()).unwrap();
        let s1 = StackString::from_utf8(&v).unwrap();
        assert_eq!(s0.as_str(), s1.as_str());

        let v: Vec<_> = (0..20).map(|_| rng.gen::<u8>()).collect();
        let s0 = String::from_utf8(v.clone());
        let s1 = StackString::from_utf8(&v);

        match s0 {
            Ok(s) => assert_eq!(s.as_str(), s1.unwrap().as_str()),
            Err(e) => assert_eq!(e.utf8_error(), s1.unwrap_err()),
        }
    }

    #[test]
    fn test_from_utf8_vec() {
        let mut rng = thread_rng();
        let v: Vec<_> = (0..20).map(|_| rng.gen::<u8>() & 0x7f).collect();
        let s0 = String::from_utf8(v.clone()).unwrap();
        let s1 = StackString::from_utf8_vec(v).unwrap();
        assert_eq!(s0.as_str(), s1.as_str());

        let v: Vec<_> = (0..20).map(|_| rng.gen::<u8>()).collect();
        let s0 = String::from_utf8(v.clone());
        let s1 = StackString::from_utf8_vec(v);

        match s0 {
            Ok(s) => assert_eq!(s.as_str(), s1.unwrap().as_str()),
            Err(e) => assert_eq!(e, s1.unwrap_err()),
        }
    }

    #[test]
    fn test_string_from_stackstring() {
        let s0 = StackString::from("Hello there");
        let s1: String = s0.clone().into();
        assert_eq!(s0.as_str(), s1.as_str());
    }

    #[test]
    fn test_stackstring_from_string() {
        let s0 = String::from("Hello there");
        let s1: StackString = s0.clone().into();
        assert_eq!(s0.as_str(), s1.as_str());
        let s1: StackString = (&s0).into();
        assert_eq!(s0.as_str(), s1.as_str());
    }

    #[test]
    fn test_borrow() {
        use std::borrow::Borrow;
        let s = StackString::from("Hello");
        let st: &str = s.borrow();
        assert_eq!(st, "Hello");
    }

    #[test]
    fn test_as_ref() {
        use std::path::Path;

        let s = StackString::from("Hello");
        let st: &str = s.as_ref();
        assert_eq!(st, s.as_str());
        let bt: &[u8] = s.as_ref();
        assert_eq!(bt, s.as_bytes());
        let pt: &Path = s.as_ref();
        assert_eq!(pt, Path::new("Hello"));
    }

    #[test]
    fn test_from_str() {
        let s = StackString::from("Hello");
        let st: StackString = "Hello".parse().unwrap();
        assert_eq!(s, st);
    }

    #[test]
    fn test_partialeq_cow() {
        use std::path::Path;
        let p = Path::new("Hello");
        let ps = p.to_string_lossy();
        let s = StackString::from("Hello");
        assert_eq!(s, ps);
    }

    #[test]
    fn test_partial_eq_string() {
        assert_eq!(StackString::from("Hello"), String::from("Hello"));
        assert_eq!(StackString::from("Hello"), "Hello");
        assert_eq!(&StackString::from("Hello"), "Hello");
    }

    #[test]
    fn test_from_iterator_char() {
        let mut rng = thread_rng();
        let v: Vec<char> = (0..20).map(|_| rng.gen::<char>()).collect();
        let s0: StackString = v.iter().map(|x| *x).collect();
        let s1: String = v.iter().map(|x| *x).collect();
        assert_eq!(s0, s1);
    }

    #[test]
    fn test_contains_stackstring() {
        let a: StackString = "hey there".into();
        let b: StackString = "hey".into();
        assert!(a.contains(b.as_str()));
    }

    #[test]
    fn test_contains_char() {
        let a: StackString = "hey there".into();
        assert!(a.contains(' '));
    }

    #[test]
    fn test_equality() {
        let s: StackString = "hey".into();
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
        let s = StackString::from_sql(&t, raw).unwrap();
        assert_eq!(s, StackString::from("Hello There"));

        assert!(<StackString as FromSql>::accepts(&t));
    }

    #[cfg(feature = "postgres_types")]
    #[test]
    fn test_to_sql() {
        let s = StackString::from("Hello There");
        let t = Type::TEXT;
        assert!(<StackString as ToSql>::accepts(&t));
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
        let s = StackString::from_display(t);
        assert_eq!(s, StackString::from("THIS IS A TEST"));
    }

    #[test]
    fn test_format_sstr() {
        use crate::format_sstr;
        use std::fmt::Write;

        let s = format_sstr!("This is a test {}", 22);
        assert_eq!(s, StackString::from("This is a test 22"));
    }

    #[test]
    fn test_from_utf8_lossy() {
        let mut v = Vec::new();
        v.extend_from_slice("this is a test".as_bytes());
        v.push(0xff);
        v.extend_from_slice("yes".as_bytes());
        let s = StackString::from_utf8_lossy(&v);
        assert_eq!(s.len(), 20);
        assert_eq!(s.is_inline(), true);
    }

    #[test]
    fn test_serde() {
        use serde::Deserialize;

        let s = StackString::from("HELLO");
        let t = "HELLO";
        let s = serde_json::to_vec(&s).unwrap();
        let t = serde_json::to_vec(t).unwrap();
        assert_eq!(s, t);

        let s = r#"{"a": "b"}"#;

        #[derive(Deserialize)]
        struct A {
            a: StackString,
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
    fn test_stackstring_async_graphql() {
        use async_graphql::{
            dataloader::{DataLoader, Loader},
            Context, EmptyMutation, EmptySubscription, Object, Schema,
        };
        use async_trait::async_trait;
        use std::{collections::HashMap, convert::Infallible};

        struct StackStringLoader;

        impl StackStringLoader {
            fn new() -> Self {
                Self
            }
        }

        #[async_trait]
        impl Loader<StackString> for StackStringLoader {
            type Value = StackString;
            type Error = Infallible;

            async fn load(
                &self,
                _: &[StackString],
            ) -> Result<HashMap<StackString, Self::Value>, Self::Error> {
                let mut m = HashMap::new();
                m.insert("HELLO".into(), "WORLD".into());
                Ok(m)
            }
        }

        struct QueryRoot;

        #[Object]
        impl<'a> QueryRoot {
            async fn hello(&self, ctx: &Context<'a>) -> Result<Option<StackString>, Infallible> {
                let hello = ctx
                    .data::<DataLoader<StackStringLoader>>()
                    .unwrap()
                    .load_one("hello".into())
                    .await
                    .unwrap();
                Ok(hello)
            }
        }

        let expected_sdl = include_str!("../tests/data/sdl_file_stackstring.txt");

        let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
            .data(DataLoader::new(
                StackStringLoader::new(),
                tokio::task::spawn,
            ))
            .finish();
        let sdl = schema.sdl();

        assert_eq!(&sdl, expected_sdl);
    }
}
