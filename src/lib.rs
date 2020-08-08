#[cfg(feature = "diesel_types")]
#[macro_use]
extern crate diesel;

use std::convert::Infallible;
use derive_more::{Deref, DerefMut, Display, From, Index, IndexMut, Into};
use serde::{Deserialize, Serialize};
use smartstring::alias::String as SmartString;
use std::{
    borrow::{Borrow, Cow},
    iter::FromIterator,
    path::Path,
    str::FromStr,
    string::FromUtf8Error,
};

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
use tokio_postgres::types::{FromSql, IsNull, ToSql, Type};

#[cfg(feature = "diesel_types")]
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
    FromSqlRow,
    AsExpression,
)]
#[sql_type = "Text"]
pub struct StackString(SmartString);

#[cfg(not(feature = "diesel_types"))]
#[derive(
    Display,
    Serialize,
    Deserialize,
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
    Deref,
    DerefMut,
    Index,
    IndexMut,
)]
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

    pub fn from_utf8(vec: Vec<u8>) -> Result<Self, FromUtf8Error> {
        String::from_utf8(vec).map(Into::into)
    }

    pub fn contains<T: AsRef<str>>(&self, s: T) -> bool {
        self.as_str().contains(s.as_ref())
    }
}

impl From<StackString> for String {
    fn from(item: StackString) -> Self {
        item.0.into()
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

impl Borrow<str> for StackString {
    fn borrow(&self) -> &str {
        self.0.borrow()
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

impl AsRef<Path> for StackString {
    fn as_ref(&self) -> &Path {
        Path::new(self.0.as_str())
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
        PartialEq::eq(&self[..], &other[..])
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
        let mut buf = Self::new();
        buf.0.extend(iter);
        buf
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


#[cfg(test)]
mod tests {
    use crate::StackString;

    #[test]
    fn test_contains() {
        let a: StackString = "hey there".into();
        let b: StackString = "hey".into();
        assert!(a.contains(&b));
    }
}