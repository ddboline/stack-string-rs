use smartcow::SmartCow;
use derive_more::{Deref, DerefMut, Display, From, Index, IndexMut, Into};
use hyper::Body;
use serde::{self, Serialize, Deserialize, Deserializer, Serializer};
use smartstring::alias::String as SmartString;
use std::{
    borrow::{Borrow, Cow},
    convert::Infallible,
    ffi::OsStr,
    fmt::{self, Write as FmtWrite},
    iter::FromIterator,
    path::Path,
    str::FromStr,
    string::FromUtf8Error,
};
pub use smartstring::MAX_INLINE;

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
)]
#[cfg_attr(feature = "diesel_types", derive(FromSqlRow, AsExpression))]
#[cfg_attr(feature = "diesel_types", sql_type = "Text")]
pub struct StackCow<'a>(
    #[serde(serialize_with = "serialize", deserialize_with = "deserialize")]
    SmartCow<'a>
);

impl<'a> StackCow<'a> {
    pub fn new() -> Self {
        Self(SmartCow::Owned(SmartString::new()))
    }

    pub fn is_borrowed(&self) -> bool {
        match self.0 {
            SmartCow::Borrowed(_) => true,
            SmartCow::Owned(_) => false,
        }        
    }

    pub fn is_owned(&self) -> bool {
        !self.is_borrowed()
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
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

pub fn serialize<'a, S>(s: &SmartCow<'a>, serializer: S) -> Result<S::Ok, S::Error>
where S: Serializer,
{
    serializer.serialize_str(s.as_ref())
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<SmartCow<'static>, D::Error>
where D: Deserializer<'de>,
{
    SmartString::deserialize(deserializer).map(Into::into)
}

impl<'a> From<StackString> for StackCow<'a> {
    fn from(item: StackString) -> Self {
        let s: SmartString = item.into();
        let s: SmartCow = s.into();
        Self(s)
    }
}

impl<'a> From<StackCow<'a>> for StackString {
    fn from(item: StackCow<'a>) -> Self {
        match item.0 {
            SmartCow::Borrowed(s) => s.into(),
            SmartCow::Owned(s) => s.into(),
        }
    }
}

impl<'a> From<Cow<'a, str>> for StackCow<'a> {
    fn from(item: Cow<'a, str>) -> Self {
        match item {
            Cow::Borrowed(s) => Self(SmartCow::Borrowed(s)),
            Cow::Owned(s) => Self(SmartCow::Owned(s.into())),
        }
    }
}

impl<'a> From<StackCow<'a>> for String {
    fn from(item: StackCow) -> Self {
        match item.0 {
            SmartCow::Borrowed(s) => s.into(),
            SmartCow::Owned(s) => s.into(),
        }
    }
}

impl<'a> From<&StackCow<'a>> for String {
    fn from(item: &StackCow) -> Self {
        item.as_str().into()
    }
}

impl<'a> From<&StackCow<'a>> for StackCow<'a> {
    fn from(item: &StackCow) -> Self {
        Self(item.0.clone().to_owned())
    }
}

impl<'a> From<String> for StackCow<'a> {
    fn from(item: String) -> Self {
        Self(item.into())
    }
}

impl<'a> From<&String> for StackCow<'a> {
    fn from(item: &String) -> Self {
        Self(SmartCow::Owned(item.into()))
    }
}

impl<'a> From<&'a str> for StackCow<'a> {
    fn from(item: &'a str) -> Self {
        Self(SmartCow::Borrowed(item))
    }
}

impl<'a> From<&'a StackCow<'a>> for &'a str {
    fn from(item: &'a StackCow) -> &'a str {
        item.as_str()
    }
}

impl<'a> Borrow<str> for StackCow<'a> {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl<'a> AsRef<str> for StackCow<'a> {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<'a> AsRef<[u8]> for StackCow<'a> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
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
        let s: SmartString = s.into();
        Ok(Self(SmartCow::Owned(s)))
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
        let s: &str = self.0.as_ref();
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
        let s: SmartString = SmartString::from_iter(iter);
        Self(SmartCow::Owned(s))
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

impl<'a> From<StackCow<'a>> for Body {
    #[inline]
    fn from(s: StackCow) -> Body {
        let s: String = s.into();
        Body::from(s)
    }
}

#[cfg(feature = "sqlx_types")]
impl<'a> sqlx_core::encode::Encode<'_, sqlx_core::postgres::Postgres> for StackCow<'a> {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx_core::postgres::PgArgumentBuffer,
    ) -> sqlx_core::encode::IsNull {
        <&str as sqlx_core::encode::Encode<sqlx_core::postgres::Postgres>>::encode(&**self, buf)
    }
}

#[cfg(feature = "sqlx_types")]
impl<'a> sqlx_core::types::Type<sqlx_core::postgres::Postgres> for StackCow<'a> {
    fn type_info() -> sqlx_core::postgres::PgTypeInfo {
        <&str as sqlx_core::types::Type<sqlx_core::postgres::Postgres>>::type_info()
    }

    fn compatible(ty: &sqlx_core::postgres::PgTypeInfo) -> bool {
        <&str as sqlx_core::types::Type<sqlx_core::postgres::Postgres>>::compatible(ty)
    }
}

#[cfg(feature = "sqlx_types")]
impl<'a> sqlx_core::decode::Decode<'_, sqlx_core::postgres::Postgres> for StackCow<'a> {
    fn decode(
        value: sqlx_core::postgres::PgValueRef<'_>,
    ) -> Result<Self, sqlx_core::error::BoxDynError> {
        <String as sqlx_core::decode::Decode<'_, sqlx_core::postgres::Postgres>>::decode(value)
            .map(|s| s.into())
    }
}

#[cfg(test)]
mod tests {
    use rand::{thread_rng, Rng};

    use crate::StackString;
    use crate::StackCow;

    #[test]
    fn test_smartstring_validate() {
        smartstring::validate();
    }

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
        let st: StackCow = "Hello".parse().unwrap();
        assert_eq!(s, st);
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
        assert_eq!(s, StackCow::from("THIS IS A TEST"));
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
}
