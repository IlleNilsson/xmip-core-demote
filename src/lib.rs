#![forbid(unsafe_code)]

//! Demotion: writing Message Context values back out — into a structured
//! Message, or into an executable artifact's configuration. The standard
//! Demote unit's declaration is compiled always; the demotion's code comes
//! with the default feature `units`, so the configuration knows the unit
//! without linking the path engine.

pub mod declaration;

#[cfg(feature = "units")]
mod demotion;

pub use declaration::DECLARATION;
#[cfg(feature = "units")]
pub use demotion::*;
