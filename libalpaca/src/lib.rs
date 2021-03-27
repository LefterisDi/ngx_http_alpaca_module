//! ALPaCA
//!
//! A library to implement the ALPaCA defense to Website Fingerprinting
//! attacks.
extern crate base64;
extern crate html5ever;
extern crate image;
extern crate kuchiki;
extern crate rand;
extern crate rand_distr;
extern crate libc;

pub mod deterministic;
pub mod distribution;
pub mod dom;
pub mod inlining;
pub mod morphing;
pub mod pad;
pub mod parse;
pub mod utils;
