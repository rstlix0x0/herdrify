//! Writes herdrify files into the Herdr home and drives herdr-app and herdr-core.
//!
//! This crate root holds no logic, so it carries no unit tests.
//!
//! Everything herdrify does to Herdr goes through one crate, so the files herdrify generates and
//! the commands it sends have a single owner.
//!
//! Responsibilities:
//!
//! - writing generated layouts, themes and key sections into the Herdr home
//! - driving herdr-app and herdr-core on herdrify's behalf
//!
//! Non-responsibilities:
//!
//! - the wire protocols themselves; the herdr-core and herdr-app clients own those
