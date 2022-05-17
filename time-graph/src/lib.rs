//! [`time-graph`] provides always-on profiling for your code, allowing to
//! record the execution time of functions, spans inside these functions and the
//! full call graph of spans and functions at run-time.
//!
//! # Core concepts
//!
//! There are two main concepts in this crate: [`CallSite`] identify a single
//! call site in the source, usually a full function. One can then create a
//! [`Span`] from any callsite, representing a single execution of the code.
//! When executed, the [`Span`] will its elapsed time, and store it in the
//! global call graph.
//!
//! # Controlling data collection
//!
//! By default, no data is collected until you call [`enable_data_collection`]
//! to start collecting timing data. Once you are done running your code, you
//! can extract collected data with [`get_full_graph`], and possibly clear all
//! collected data using [`clear_collected_data`].
//!
//! [`time-graph`]: https://crates.io/crates/time-graph
//!
//! # Overhead and limitations
//!
//! When data collection is disabled, this crate adds an overheard around 10 ns
//! when calling a function or entering a span. With data collection enabled,
//! this crate adds an overhead around 100 ns when calling a function or
//! entering a span.
//!
//! This makes this crate only useful for gathering profiling data on
//! function/spans taking at least 1 µs to execute.
//!
//! # Crate features
//!
//! This crate has two cargo features:
//!
//! - **json**: enables json output format for the full call graph
//! - **table**: enables pretty-printing the full call graph to a table using
//!   [term-table](https://crates.io/crates/term-table)

#![allow(clippy::redundant_field_names, clippy::needless_return)]

pub use time_graph_macros::instrument;

#[doc(hidden)]
pub use once_cell::sync::Lazy;

/// Create a new [`CallSite`] with the given name at the current source
/// location.
///
/// # Examples
/// ```
/// use time_graph::{CallSite, callsite};
///
/// let callsite: &'static CallSite = callsite!("here");
/// assert_eq!(callsite.name(), "here");
/// ```
#[macro_export]
macro_rules! callsite {
    ($name: expr) => {
        {
            static CALL_SITE: $crate::Lazy<$crate::CallSite> = $crate::Lazy::new(|| {
                $crate::CallSite::new(
                    $name.into(),
                    module_path!(),
                    file!(),
                    line!(),
                )
            });
            static REGISTRATION: $crate::Lazy<()> = $crate::Lazy::new(|| {
                $crate::register_callsite(&*CALL_SITE)
            });
            $crate::Lazy::force(&REGISTRATION);

            &*CALL_SITE
        }
    };
}

/// Run a block of code inside a new span
///
/// This macro creates a new [`CallSite`] with the given name at the current
/// source location, and record the provided code execution by running it inside
/// a [`Span`].
///
/// # Examples
/// ```
/// use time_graph::spanned;
///
/// let result = spanned!("named", {
///     let first = 30;
///     let second = 12;
///     first + second
/// });
///
/// assert_eq!(result, 42);
///
/// let result = spanned!(format!("dynamic name: {}", "is nice"), {
///     let first = 30;
///     let second = 12;
///     first - second
/// });
///
/// assert_eq!(result, 18);
/// ```
#[macro_export]
macro_rules! spanned {
    ($name: expr, $block: expr) => {
        {
            let __tfg_callsite = $crate::callsite!($name);
            let __tfg_span = $crate::Span::new(__tfg_callsite);
            let __tfg_guard = __tfg_span.enter();

            $block
        }
    }
}

mod callsite;
pub use self::callsite::CallSite;
pub(crate) use self::callsite::CallSiteId;
pub use self::callsite::{register_callsite, traverse_registered_callsite};

mod graph;
pub use self::graph::{Span, SpanGuard};
pub use self::graph::{get_full_graph, clear_collected_data, enable_data_collection};
pub use self::graph::{FullCallGraph, TimedSpan};
