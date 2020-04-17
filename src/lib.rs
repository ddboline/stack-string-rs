#[cfg(feature="diesel_types")]
#[macro_use]
extern crate diesel;


use anyhow::Error;
use derive_more::{Display, From, Into};
use inlinable_string::InlinableString;
pub use inlinable_string::StringExt;
use serde::{Deserialize, Serialize};
use std::{
    borrow::{Borrow, Cow},
    ops::{Deref, DerefMut},
    str::FromStr,
    string::{FromUtf16Error, FromUtf8Error},
};

#[cfg(feature = "postgres_types")]
use postgres_types::{FromSql, IsNull, ToSql, Type};
#[cfg(feature = "postgres_types")]
use std::io::Write;

#[cfg(feature = "diesel_types")]
use bytes::BytesMut;
#[cfg(feature = "diesel_types")]
use diesel::{
    backend::Backend,
    deserialize::{FromSql as DeFromSql, Result as DeResult},
    serialize::{Output, Result as SerResult, ToSql as DeToSql},
    sql_types::Text,
};

#[cfg(diesel_types)]
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Into,
    From,
    Display,
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
#[serde(into = "String", from = "&str")]
pub struct StackString(InlinableString);

#[cfg(not(diesel_types))]
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Into,
    From,
    Display,
    PartialEq,
    Eq,
    Hash,
    Default,
    PartialOrd,
    Ord,
)]
#[serde(into = "String", from = "&str")]
pub struct StackString(InlinableString);

impl StackString {
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<StackString> for String {
    fn from(item: StackString) -> Self {
        match item.0 {
            InlinableString::Heap(s) => s,
            InlinableString::Inline(s) => s.to_string(),
        }
    }
}

impl From<String> for StackString {
    fn from(item: String) -> Self {
        Self(item.into())
    }
}

impl From<&String> for StackString {
    fn from(item: &String) -> Self {
        Self(item.as_str().into())
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

#[cfg(diesel_types)]
impl<'a> FromSql<'a> for StackString {
    fn from_sql(
        ty: &Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let s = <String as FromSql>::from_sql(ty, raw)?;
        Ok(s.into())
    }

    fn accepts(ty: &Type) -> bool {
        <String as FromSql>::accepts(ty)
    }
}

#[cfg(diesel_types)]
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
        self.to_string().to_sql_checked(ty, out)
    }
}

#[cfg(postgres_types)]
impl<DB> DeToSql<Text, DB> for StackString
where
    DB: Backend,
    str: DeToSql<Text, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> SerResult {
        self.as_str().to_sql(out)
    }
}

#[cfg(postgres_types)]
impl<ST, DB> DeFromSql<ST, DB> for StackString
where
    DB: Backend,
    *const str: DeFromSql<ST, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> DeResult<Self> {
        let str_ptr = <*const str as DeFromSql<ST, DB>>::from_sql(bytes)?;
        let string = unsafe { &*str_ptr };
        Ok(string.into())
    }
}

impl Deref for StackString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.0.as_ref()
    }
}

impl DerefMut for StackString {
    fn deref_mut(&mut self) -> &mut str {
        self.0.as_mut()
    }
}

impl AsRef<str> for StackString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for StackString {
    type Err = Error;
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

impl<'a> StringExt<'a> for StackString {
    #[inline]
    fn new() -> Self {
        StackString(InlinableString::new())
    }

    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        StackString(InlinableString::with_capacity(capacity))
    }

    #[inline]
    fn from_utf8(vec: Vec<u8>) -> Result<Self, FromUtf8Error> {
        InlinableString::from_utf8(vec).map(StackString)
    }

    #[inline]
    fn from_utf16(v: &[u16]) -> Result<Self, FromUtf16Error> {
        InlinableString::from_utf16(v).map(StackString)
    }

    #[inline]
    fn from_utf16_lossy(v: &[u16]) -> Self {
        StackString(InlinableString::from_utf16_lossy(v))
    }

    #[inline]
    unsafe fn from_raw_parts(buf: *mut u8, length: usize, capacity: usize) -> Self {
        StackString(InlinableString::from_raw_parts(buf, length, capacity))
    }

    #[inline]
    unsafe fn from_utf8_unchecked(bytes: Vec<u8>) -> Self {
        StackString(InlinableString::from_utf8_unchecked(bytes))
    }

    #[inline]
    fn into_bytes(self) -> Vec<u8> {
        InlinableString::into_bytes(self.0)
    }

    #[inline]
    fn push_str(&mut self, string: &str) {
        InlinableString::push_str(&mut self.0, string)
    }

    #[inline]
    fn capacity(&self) -> usize {
        InlinableString::capacity(&self.0)
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        InlinableString::reserve(&mut self.0, additional)
    }

    #[inline]
    fn reserve_exact(&mut self, additional: usize) {
        InlinableString::reserve_exact(&mut self.0, additional)
    }

    #[inline]
    fn shrink_to_fit(&mut self) {
        InlinableString::shrink_to_fit(&mut self.0)
    }

    #[inline]
    fn push(&mut self, ch: char) {
        InlinableString::push(&mut self.0, ch)
    }

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        InlinableString::as_bytes(&self.0)
    }

    #[inline]
    fn truncate(&mut self, new_len: usize) {
        InlinableString::truncate(&mut self.0, new_len)
    }

    #[inline]
    fn pop(&mut self) -> Option<char> {
        InlinableString::pop(&mut self.0)
    }

    #[inline]
    fn remove(&mut self, idx: usize) -> char {
        InlinableString::remove(&mut self.0, idx)
    }

    #[inline]
    fn insert(&mut self, idx: usize, ch: char) {
        InlinableString::insert(&mut self.0, idx, ch)
    }

    #[inline]
    unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
        InlinableString::as_mut_slice(&mut self.0)
    }

    #[inline]
    fn len(&self) -> usize {
        InlinableString::len(&self.0)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
