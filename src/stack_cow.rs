use derive_more::Display;
use serde::{self, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    borrow::{Borrow, Cow},
    convert::Infallible,
    ffi::OsStr,
    fmt::{self, Write as FmtWrite},
    iter::FromIterator,
    ops::Deref,
    path::Path,
    str::FromStr,
    string::FromUtf8Error,
};

use crate::MAX_INLINE;

use crate::stack_string::StackString;

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

#[derive(Display, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "diesel_types", derive(FromSqlRow, AsExpression))]
#[cfg_attr(feature = "diesel_types", sql_type = "Text")]
pub enum StackCow<'a> {
    Borrowed(&'a str),
    Owned(StackString),
}

impl Default for StackCow<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> StackCow<'a> {
    pub fn new() -> Self {
        Self::Owned(StackString::new())
    }

    pub fn to_owned(&self) -> StackCow<'static> {
        self.clone().into_owned()
    }

    pub fn into_owned(self) -> StackCow<'static> {
        match self {
            Self::Borrowed(b) => StackCow::Owned(b.into()),
            Self::Owned(o) => StackCow::Owned(o),
        }
    }

    pub fn is_borrowed(&self) -> bool {
        match self {
            Self::Borrowed(_) => true,
            Self::Owned(_) => false,
        }
    }

    pub fn is_owned(&self) -> bool {
        !self.is_borrowed()
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Borrowed(s) => *s,
            Self::Owned(o) => o.as_str(),
        }
    }

    pub fn from_utf8(vec: Vec<u8>) -> Result<Self, FromUtf8Error> {
        String::from_utf8(vec).map(Into::into)
    }

    pub fn from_utf8_lossy(v: &'a [u8]) -> Self {
        if v.len() > MAX_INLINE {
            String::from_utf8_lossy(v).into()
        } else {
            StackString::from_utf8_lossy(v).into()
        }
    }

    /// # Panics
    /// `from_display` panics if a formatting trait implementation returns an
    /// error. This indicates an incorrect implementation
    /// since `fmt::Write for String` never returns an error itself.
    pub fn from_display(buf: impl fmt::Display) -> Self {
        let mut s = StackString::new();
        write!(s, "{buf}").unwrap();
        s.into()
    }
}

impl Deref for StackCow<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(b) => *b,
            Self::Owned(o) => o,
        }
    }
}

impl<'a> PartialOrd<Self> for StackCow<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl<'a> Ord for StackCow<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl<'a> Serialize for StackCow<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for StackCow<'_> {
    fn deserialize<D>(deserializer: D) -> Result<StackCow<'static>, D::Error>
    where
        D: Deserializer<'de>,
    {
        StackString::deserialize(deserializer).map(Into::into)
    }
}

impl<'a> From<StackString> for StackCow<'a> {
    fn from(item: StackString) -> Self {
        Self::Owned(item)
    }
}

impl<'a> From<StackCow<'a>> for StackString {
    fn from(item: StackCow<'a>) -> Self {
        match item {
            StackCow::Borrowed(s) => s.into(),
            StackCow::Owned(s) => s,
        }
    }
}

impl<'a> From<Cow<'a, str>> for StackCow<'a> {
    fn from(item: Cow<'a, str>) -> Self {
        match item {
            Cow::Borrowed(s) => Self::Borrowed(s),
            Cow::Owned(s) => Self::Owned(s.into()),
        }
    }
}

impl<'a> From<StackCow<'a>> for String {
    fn from(item: StackCow) -> Self {
        match item {
            StackCow::Borrowed(s) => s.into(),
            StackCow::Owned(s) => s.into(),
        }
    }
}

impl<'a> From<&StackCow<'a>> for String {
    fn from(item: &StackCow) -> Self {
        item.as_str().into()
    }
}

impl<'a> From<String> for StackCow<'a> {
    fn from(item: String) -> Self {
        Self::Owned(item.into())
    }
}

impl<'a> From<&'a String> for StackCow<'a> {
    fn from(item: &'a String) -> Self {
        Self::Borrowed(item.as_str())
    }
}

impl<'a> From<&'a str> for StackCow<'a> {
    fn from(item: &'a str) -> Self {
        StackCow::Borrowed(item)
    }
}

impl<'a> From<&'a StackCow<'a>> for &'a str {
    fn from(item: &'a StackCow) -> &'a str {
        item.as_str()
    }
}

impl<'a> Borrow<str> for StackCow<'a> {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<'a> AsRef<str> for StackCow<'a> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'a> AsRef<[u8]> for StackCow<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl<'a> AsRef<OsStr> for StackCow<'a> {
    fn as_ref(&self) -> &OsStr {
        self.as_str().as_ref()
    }
}

impl<'a> AsRef<Path> for StackCow<'a> {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl<'a> FromStr for StackCow<'a> {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::Owned(s.into()))
    }
}

impl<'a> PartialEq<Cow<'a, str>> for StackCow<'a> {
    #[inline]
    fn eq(&self, other: &Cow<'a, str>) -> bool {
        PartialEq::eq(&self[..], &other[..])
    }
}

impl<'a> PartialEq<String> for StackCow<'a> {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        PartialEq::eq(&self[..], &other[..])
    }
}

impl<'a> PartialEq<str> for StackCow<'a> {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        let s: &str = self.as_ref();
        PartialEq::eq(s, other)
    }
}

impl<'a> PartialEq<&'a str> for StackCow<'a> {
    #[inline]
    fn eq(&self, other: &&'a str) -> bool {
        PartialEq::eq(&self[..], &other[..])
    }
}

impl<'a> FromIterator<char> for StackCow<'a> {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        Self::Owned(StackString::from_iter(iter))
    }
}

#[cfg(feature = "postgres_types")]
impl<'a> FromSql<'a> for StackCow<'a> {
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
impl<'a> ToSql for StackCow<'a> {
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
impl<'a, ST, DB> DeFromSql<ST, DB> for StackCow<'a>
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
impl<'a, DB> DeToSql<Text, DB> for StackCow<'a>
where
    DB: Backend,
    str: DeToSql<Text, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> SerResult {
        self.as_str().to_sql(out)
    }
}

#[cfg(feature = "rweb-openapi")]
impl<'a> Entity for StackCow<'a> {
    fn type_name() -> Cow<'static, str> {
        str::type_name()
    }

    #[inline]
    fn describe(comp_d: &mut ComponentDescriptor) -> ComponentOrInlineSchema {
        str::describe(comp_d)
    }
}

#[cfg(feature = "rweb-openapi")]
impl<'a> ResponseEntity for StackCow<'a> {
    #[inline]
    fn describe_responses(comp_d: &mut ComponentDescriptor) -> Responses {
        String::describe_responses(comp_d)
    }
}

#[cfg(feature = "rweb-openapi")]
impl<'a> From<StackCow<'a>> for Body {
    #[inline]
    fn from(s: StackCow) -> Body {
        let s: String = s.into();
        Body::from(s)
    }
}

#[cfg(test)]
mod tests {
    use rand::{thread_rng, Rng};
    use serde::Deserialize;

    use crate::{StackCow, StackString};

    #[test]
    fn test_default() {
        assert_eq!(StackCow::new(), StackCow::default());
    }

    #[test]
    fn test_from_utf8() {
        let mut rng = thread_rng();
        let v: Vec<_> = (0..20).map(|_| rng.gen::<u8>() & 0x7f).collect();
        let s0 = String::from_utf8(v.clone()).unwrap();
        let s1 = StackCow::from_utf8(v).unwrap();
        assert_eq!(s0.as_str(), s1.as_str());

        let v: Vec<_> = (0..20).map(|_| rng.gen::<u8>()).collect();
        let s0 = String::from_utf8(v.clone());
        let s1 = StackCow::from_utf8(v);

        match s0 {
            Ok(s) => assert_eq!(s.as_str(), s1.unwrap().as_str()),
            Err(e) => assert_eq!(e, s1.unwrap_err()),
        }
    }

    #[test]
    fn test_string_from_stack_cow() {
        let s0 = StackCow::from("Hello there");
        let s1: String = s0.clone().into();
        assert_eq!(s0.as_str(), s1.as_str());
    }

    #[test]
    fn test_stack_cow_from_string() {
        let s0 = String::from("Hello there");
        let s1: StackCow = s0.clone().into();
        assert_eq!(s0.as_str(), s1.as_str());
        let s1: StackCow = (&s0).into();
        assert_eq!(s0.as_str(), s1.as_str());
    }

    #[test]
    fn test_borrow() {
        use std::borrow::Borrow;
        let s = StackCow::from("Hello");
        let st: &str = s.borrow();
        assert_eq!(st, "Hello");
    }

    #[test]
    fn test_as_ref() {
        use std::path::Path;

        let s = StackCow::from("Hello");
        let st: &str = s.as_ref();
        assert_eq!(st, s.as_str());
        let bt: &[u8] = s.as_ref();
        assert_eq!(bt, s.as_bytes());
        let pt: &Path = s.as_ref();
        assert_eq!(pt, Path::new("Hello"));
    }

    #[test]
    fn test_from_str() {
        let s = StackCow::from("Hello");
        assert_eq!(s, StackCow::Borrowed("Hello"));
    }

    #[test]
    fn test_partialeq_cow() {
        use std::path::Path;
        let p = Path::new("Hello");
        let ps = p.to_string_lossy();
        let s = StackCow::from("Hello");
        assert_eq!(s, ps);
    }

    #[test]
    fn test_partial_eq_string() {
        assert_eq!(StackCow::from("Hello"), String::from("Hello"));
        assert_eq!(StackCow::from("Hello"), "Hello");
        assert_eq!(&StackCow::from("Hello"), "Hello");
    }

    #[test]
    fn test_from_iterator_char() {
        let mut rng = thread_rng();
        let v: Vec<char> = (0..20).map(|_| rng.gen::<char>()).collect();
        let s0: StackCow = v.iter().map(|x| *x).collect();
        let s1: String = v.iter().map(|x| *x).collect();
        assert_eq!(s0, s1);
    }

    #[test]
    fn test_contains_stack_cow() {
        let a: StackCow = "hey there".into();
        let b: StackCow = "hey".into();
        assert!(a.contains(b.as_str()));
    }

    #[test]
    fn test_contains_char() {
        let a: StackCow = "hey there".into();
        assert!(a.contains(' '));
    }

    #[test]
    fn test_equality() {
        let s: StackCow = "hey".into();
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
        let s = StackCow::from_sql(&t, raw).unwrap();
        assert_eq!(s, StackCow::from("Hello There"));

        assert!(<StackCow as FromSql>::accepts(&t));
    }

    #[cfg(feature = "postgres_types")]
    #[test]
    fn test_to_sql() {
        let s = StackCow::from("Hello There");
        let t = Type::TEXT;
        assert!(<StackCow as ToSql>::accepts(&t));
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
        let s = StackCow::from_display(t);
        assert_eq!(s, StackCow::from(StackString::from("THIS IS A TEST")));
    }

    #[test]
    fn test_from_utf8_lossy() {
        let mut v = Vec::new();
        v.extend_from_slice("this is a test".as_bytes());
        v.push(0xff);
        v.extend_from_slice("yes".as_bytes());
        let s = StackCow::from_utf8_lossy(&v);
        assert_eq!(s.len(), 20);
        assert_eq!(s.is_owned(), true);
        let s: StackString = s.into();
        assert_eq!(s.len(), 20);
        assert_eq!(s.is_inline(), true);
    }

    #[test]
    fn test_serde() {
        let s = StackCow::from("HELLO");
        let t = "HELLO";
        let s = serde_json::to_vec(&s).unwrap();
        let t = serde_json::to_vec(t).unwrap();
        assert_eq!(s, t);

        let s = r#"{"a": "b"}"#;

        #[derive(Deserialize)]
        struct A<'a> {
            a: StackCow<'a>,
        }

        #[derive(Deserialize)]
        struct B {
            a: String,
        }

        let a: A = serde_json::from_str(s).unwrap();
        let b: B = serde_json::from_str(s).unwrap();
        assert_eq!(a.a.as_str(), b.a.as_str());
    }
}
