use derive_more::{Deref, DerefMut, Display, From, Index, IndexMut, Into};
use serde::{Deserialize, Serialize};
use smartstring::alias::String as SmartStringInner;
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

#[cfg(feature = "postgres_types")]
use bytes::BytesMut;
#[cfg(feature = "postgres_types")]
use postgres_types::{FromSql, IsNull, ToSql, Type};

#[cfg(feature = "rweb-openapi")]
use rweb::openapi::{
    ComponentDescriptor, ComponentOrInlineSchema, Entity, ResponseEntity, Responses,
};

#[cfg(feature = "rweb-openapi")]
use rweb::hyper::Body;

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
pub struct SmartString(SmartStringInner);

impl SmartString {
    #[must_use]
    pub fn new() -> Self {
        Self(SmartStringInner::new())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[must_use]
    pub fn split_off(&mut self, index: usize) -> Self {
        Self(self.0.split_off(index))
    }

    /// Construct a `SmartString` from a `&[u8]`
    /// # Errors
    ///
    /// Will return an Error if the byte slice is not utf8 compliant
    pub fn from_utf8(v: &[u8]) -> Result<Self, Utf8Error> {
        str::from_utf8(v).map(Into::into)
    }

    /// Construct a `SmartString` from a `Vec<u8>`
    /// # Errors
    ///
    /// Will return an Error if the byte slice is not utf8 compliant
    pub fn from_utf8_vec(vec: Vec<u8>) -> Result<Self, FromUtf8Error> {
        String::from_utf8(vec).map(Into::into)
    }

    #[must_use]
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
            let mut buf = SmartString::new();
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

impl From<SmartString> for String {
    fn from(item: SmartString) -> Self {
        item.0.into()
    }
}

impl From<&SmartString> for String {
    fn from(item: &SmartString) -> Self {
        item.as_str().into()
    }
}

impl From<&SmartString> for SmartString {
    fn from(item: &SmartString) -> Self {
        item.clone()
    }
}

impl From<String> for SmartString {
    fn from(item: String) -> Self {
        Self(item.into())
    }
}

impl From<&String> for SmartString {
    fn from(item: &String) -> Self {
        Self(item.into())
    }
}

impl From<&str> for SmartString {
    fn from(item: &str) -> Self {
        Self(item.into())
    }
}

impl<'a> From<&'a SmartString> for &'a str {
    fn from(item: &SmartString) -> &str {
        item.as_str()
    }
}

impl<'a> From<Cow<'a, str>> for SmartString {
    fn from(item: Cow<'a, str>) -> Self {
        match item {
            Cow::Borrowed(s) => s.into(),
            Cow::Owned(s) => s.into(),
        }
    }
}

impl From<SmartString> for Cow<'_, str> {
    fn from(item: SmartString) -> Self {
        Cow::Owned(item.into())
    }
}

impl Borrow<str> for SmartString {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl BorrowMut<str> for SmartString {
    fn borrow_mut(&mut self) -> &mut str {
        self.0.borrow_mut()
    }
}

impl AsRef<str> for SmartString {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<[u8]> for SmartString {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsRef<OsStr> for SmartString {
    fn as_ref(&self) -> &OsStr {
        self.as_str().as_ref()
    }
}

impl AsRef<Path> for SmartString {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl FromStr for SmartString {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.into())
    }
}

impl<'a> PartialEq<Cow<'a, str>> for SmartString {
    #[inline]
    fn eq(&self, other: &Cow<'a, str>) -> bool {
        PartialEq::eq(&self[..], &other[..])
    }
}

impl<'a> PartialOrd<Cow<'a, str>> for SmartString {
    fn partial_cmp(&self, other: &Cow<'a, str>) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(&self[..], &other[..])
    }
}

impl PartialEq<String> for SmartString {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        PartialEq::eq(&self[..], &other[..])
    }
}

impl PartialOrd<String> for SmartString {
    fn partial_cmp(&self, other: &String) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(&self[..], &other[..])
    }
}

impl PartialEq<str> for SmartString {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        PartialEq::eq(&self.0, other)
    }
}

impl PartialOrd<str> for SmartString {
    fn partial_cmp(&self, other: &str) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(&self[..], other)
    }
}

impl<'a> PartialEq<&'a str> for SmartString {
    #[inline]
    fn eq(&self, other: &&'a str) -> bool {
        PartialEq::eq(&self[..], &other[..])
    }
}

impl<'a> PartialOrd<&'a str> for SmartString {
    fn partial_cmp(&self, other: &&'a str) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(&self[..], &other[..])
    }
}

impl PartialEq<SmartString> for str {
    fn eq(&self, other: &SmartString) -> bool {
        PartialEq::eq(self, &other[..])
    }
}

impl PartialOrd<SmartString> for str {
    fn partial_cmp(&self, other: &SmartString) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(self, &other[..])
    }
}

impl<'a> PartialEq<SmartString> for &'a str {
    fn eq(&self, other: &SmartString) -> bool {
        PartialEq::eq(&self[..], &other[..])
    }
}

impl<'a> PartialOrd<SmartString> for &'a str {
    fn partial_cmp(&self, other: &SmartString) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(&self[..], &other[..])
    }
}

impl FromIterator<char> for SmartString {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        let s = SmartStringInner::from_iter(iter);
        Self(s)
    }
}

#[cfg(feature = "postgres_types")]
impl<'a> FromSql<'a> for SmartString {
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
impl ToSql for SmartString {
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
impl Entity for SmartString {
    fn type_name() -> Cow<'static, str> {
        <str as Entity>::type_name()
    }

    #[inline]
    fn describe(comp_d: &mut ComponentDescriptor) -> ComponentOrInlineSchema {
        str::describe(comp_d)
    }
}

#[cfg(feature = "rweb-openapi")]
impl ResponseEntity for SmartString {
    #[inline]
    fn describe_responses(comp_d: &mut ComponentDescriptor) -> Responses {
        String::describe_responses(comp_d)
    }
}

#[cfg(feature = "rweb-openapi")]
impl From<SmartString> for Body {
    #[inline]
    fn from(s: SmartString) -> Body {
        let s: String = s.into();
        Body::from(s)
    }
}

#[macro_export]
macro_rules! format_sstr {
    ($($arg:tt)*) => {{
        use std::fmt::Write;
        let mut buf = $crate::SmartString::new();
        std::write!(buf, "{}", std::format_args!($($arg)*)).unwrap();
        buf
    }}
}

/// Allow SmartString to be used as graphql scalar value
#[cfg(feature = "async_graphql")]
#[Scalar]
impl ScalarType for SmartString {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(s) = value {
            let s: SmartString = s.into();
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

    #[cfg(feature = "async_graphql")]
    use std::future::Future;

    use crate::SmartString;

    #[test]
    fn test_default() {
        assert_eq!(SmartString::new(), SmartString::default());
    }

    #[test]
    fn test_split_off() {
        let mut s0 = "hello there".to_string();
        let s1 = s0.split_off(3);
        let mut s2: SmartString = "hello there".into();
        let s3 = s2.split_off(3);
        assert_eq!(s0.as_str(), s2.as_str());
        assert_eq!(s1.as_str(), s3.as_str());
    }

    #[test]
    fn test_from_utf8() {
        let mut rng = thread_rng();
        let v: Vec<_> = (0..20).map(|_| rng.gen::<u8>() & 0x7f).collect();
        let s0 = String::from_utf8(v.clone()).unwrap();
        let s1 = SmartString::from_utf8(&v).unwrap();
        assert_eq!(s0.as_str(), s1.as_str());

        let v: Vec<_> = (0..20).map(|_| rng.gen::<u8>()).collect();
        let s0 = String::from_utf8(v.clone());
        let s1 = SmartString::from_utf8(&v);

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
        let s1 = SmartString::from_utf8_vec(v).unwrap();
        assert_eq!(s0.as_str(), s1.as_str());

        let v: Vec<_> = (0..20).map(|_| rng.gen::<u8>()).collect();
        let s0 = String::from_utf8(v.clone());
        let s1 = SmartString::from_utf8_vec(v);

        match s0 {
            Ok(s) => assert_eq!(s.as_str(), s1.unwrap().as_str()),
            Err(e) => assert_eq!(e, s1.unwrap_err()),
        }
    }

    #[test]
    fn test_string_from_stackstring() {
        let s0 = SmartString::from("Hello there");
        let s1: String = s0.clone().into();
        assert_eq!(s0.as_str(), s1.as_str());
    }

    #[test]
    fn test_stackstring_from_string() {
        let s0 = String::from("Hello there");
        let s1: SmartString = s0.clone().into();
        assert_eq!(s0.as_str(), s1.as_str());
        let s1: SmartString = (&s0).into();
        assert_eq!(s0.as_str(), s1.as_str());
    }

    #[test]
    fn test_borrow() {
        use std::borrow::Borrow;
        let s = SmartString::from("Hello");
        let st: &str = s.borrow();
        assert_eq!(st, "Hello");
    }

    #[test]
    fn test_as_ref() {
        use std::path::Path;

        let s = SmartString::from("Hello");
        let st: &str = s.as_ref();
        assert_eq!(st, s.as_str());
        let bt: &[u8] = s.as_ref();
        assert_eq!(bt, s.as_bytes());
        let pt: &Path = s.as_ref();
        assert_eq!(pt, Path::new("Hello"));
    }

    #[test]
    fn test_from_str() {
        let s = SmartString::from("Hello");
        let st: SmartString = "Hello".parse().unwrap();
        assert_eq!(s, st);
    }

    #[test]
    fn test_partialeq_cow() {
        use std::path::Path;
        let p = Path::new("Hello");
        let ps = p.to_string_lossy();
        let s = SmartString::from("Hello");
        assert_eq!(s, ps);
    }

    #[test]
    fn test_partial_eq_string() {
        assert_eq!(SmartString::from("Hello"), String::from("Hello"));
        assert_eq!(SmartString::from("Hello"), "Hello");
        assert_eq!(&SmartString::from("Hello"), "Hello");
        assert!(SmartString::from("alpha") < "beta");
        assert!("beta" > SmartString::from("alpha"));
    }

    #[test]
    fn test_from_iterator_char() {
        let mut rng = thread_rng();
        let v: Vec<char> = (0..20).map(|_| rng.gen::<char>()).collect();
        let s0: SmartString = v.iter().map(|x| *x).collect();
        let s1: String = v.iter().map(|x| *x).collect();
        assert_eq!(s0, s1);
    }

    #[test]
    fn test_contains_stackstring() {
        let a: SmartString = "hey there".into();
        let b: SmartString = "hey".into();
        assert!(a.contains(b.as_str()));
    }

    #[test]
    fn test_contains_char() {
        let a: SmartString = "hey there".into();
        assert!(a.contains(' '));
    }

    #[test]
    fn test_equality() {
        let s: SmartString = "hey".into();
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
        let s = SmartString::from_sql(&t, raw).unwrap();
        assert_eq!(s, SmartString::from("Hello There"));

        assert!(<SmartString as FromSql>::accepts(&t));
    }

    #[cfg(feature = "postgres_types")]
    #[test]
    fn test_to_sql() {
        let s = SmartString::from("Hello There");
        let t = Type::TEXT;
        assert!(<SmartString as ToSql>::accepts(&t));
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
        let s = SmartString::from_display(t);
        assert_eq!(s, SmartString::from("THIS IS A TEST"));
    }

    #[test]
    fn test_format_sstr() {
        use crate::format_sstr;

        let s = format_sstr!("This is a test {}", 22);
        assert_eq!(s, SmartString::from("This is a test 22"));
    }

    #[test]
    fn test_from_utf8_lossy() {
        let mut v = Vec::new();
        v.extend_from_slice("this is a test".as_bytes());
        v.push(0xff);
        v.extend_from_slice("yes".as_bytes());
        let s = SmartString::from_utf8_lossy(&v);
        assert_eq!(s.len(), 20);
        assert_eq!(s.is_inline(), true);
    }

    #[test]
    fn test_serde() {
        use serde::Deserialize;

        let s = SmartString::from("HELLO");
        let t = "HELLO";
        let s = serde_json::to_vec(&s).unwrap();
        let t = serde_json::to_vec(t).unwrap();
        assert_eq!(s, t);

        let s = r#"{"a": "b"}"#;

        #[derive(Deserialize)]
        struct A {
            a: SmartString,
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

        struct SmartStringLoader;

        impl SmartStringLoader {
            fn new() -> Self {
                Self
            }
        }

        #[async_trait]
        impl Loader<SmartString> for SmartStringLoader {
            type Value = SmartString;
            type Error = Infallible;

            fn load(
                &self,
                _: &[SmartString],
            ) -> impl Future<Output = Result<HashMap<SmartString, Self::Value>, Self::Error>>
            {
                async move {
                    let mut m = HashMap::new();
                    m.insert("HELLO".into(), "WORLD".into());
                    Ok(m)
                }
            }
        }

        struct QueryRoot;

        #[Object]
        impl QueryRoot {
            async fn hello<'a>(
                &self,
                ctx: &Context<'a>,
            ) -> Result<Option<SmartString>, Infallible> {
                let hello = ctx
                    .data::<DataLoader<SmartStringLoader>>()
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
                SmartStringLoader::new(),
                tokio::task::spawn,
            ))
            .finish();
        let sdl = schema.sdl();

        assert_eq!(&sdl, expected_sdl);
    }
}
