//! Modul internal — tidak diekspos ke luar crate.
//! Hanya `lib.rs` yang boleh mengimpor dari sini.

pub(crate) mod repository;
pub(crate) mod service;

