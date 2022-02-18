# stack-string-rs
[![codecov](https://codecov.io/gh/ddboline/stack-string-rs/branch/master/graph/badge.svg)](https://codecov.io/gh/ddboline/stack-string-rs)

This started out as a wrapper around smartstring::SmartString, adding support for diesel and tokio-postgres types.  It has since expanded somewhat and now includes both a wrapper around smartcow (which is a cow type that combines SmartString and str) and SmallString an enum that is either an ArrayString with a const generic length or a heap String.
