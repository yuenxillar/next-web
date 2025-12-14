#![doc = include_str!("./docs/lib.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]


pub use rudi_core::*;
#[cfg_attr(docsrs, doc(cfg(feature = "rudi-macro")))]
#[cfg(feature = "rudi-macro-dev")]
pub use rudi_macro_dev::*;
