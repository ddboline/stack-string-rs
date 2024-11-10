#![allow(clippy::too_many_lines)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::similar_names)]
#![allow(clippy::used_underscore_binding)]
#![allow(clippy::unsafe_derive_deserialize)]

pub mod stack_string;
pub mod small_string;
pub mod stack_cow;
pub mod smart_string;

pub use crate::{
    small_string::SmallString, stack_cow::StackCow,
    stack_string::StackString,
    smart_string::SmartString,
};

pub use smartstring::MAX_INLINE;
