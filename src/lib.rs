#![allow(clippy::must_use_candidate)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::similar_names)]
#![allow(clippy::shadow_unrelated)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::used_underscore_binding)]

#[cfg(feature = "diesel_types")]
#[macro_use]
extern crate diesel;

pub mod stack_string;
// #[cfg(features = "stack-cow")]
pub mod stack_cow;

pub use crate::stack_string::StackString;
// #[cfg(features = "stack-cow")]
pub use crate::stack_cow::StackCow;