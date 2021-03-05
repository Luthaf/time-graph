use std::num::NonZeroU64;
use std::sync::atomic::{Ordering, AtomicU64, AtomicPtr};

use once_cell::sync::Lazy;

/// Store the id to be assigned to the next call site created.
static NEXT_CALL_SITE_ID: AtomicU64 = AtomicU64::new(1);
/// Store the global registry of call sites
static REGISTRY: Lazy<Registry> = Lazy::new(|| {
    Registry {
        head: AtomicPtr::new(std::ptr::null_mut()),
    }
});

/// Unique identifier of a [`CallSite`], attributed the first time the call site
/// is entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CallSiteId(NonZeroU64);

impl CallSiteId {
    pub(crate) fn new(value: u64) -> CallSiteId {
        CallSiteId(std::num::NonZeroU64::new(value).expect("got a zero value for span id"))
    }
}

/// A [`CallSite`] identify uniquely a location in the source code, and record
/// multiple attributes associated with this location.
///
/// The only way to create a [`CallSite`] is with the [`macro@callsite`] macro,
/// which also takes care of registering the call site globally.
pub struct CallSite {
    /// Unique identifier of this call site
    id: CallSiteId,
    /// The name of the call site
    name: &'static str,
    /// The name of the Rust module where the call site occurred
    module_path: &'static str,
    /// The name of the source code file where the call site occurred
    file: &'static str,
    /// The line number in the source code file where the call site occurred
    line: u32,
    /// Call sites are registered using an atomic, append only intrusive linked
    /// list. If more than one call site are registered, this will be set to the
    /// last registered call site.
    next: AtomicPtr<CallSite>,
}

impl CallSite {
    /// Create a new `CallSite` with the given metadata. This function is
    /// private to this crate, and is only marked `pub` to be able to call it
    /// from inside macros.
    #[doc(hidden)]
    pub fn new(name: &'static str, module_path: &'static str, file: &'static str, line: u32) -> CallSite {
        let id = CallSiteId::new(NEXT_CALL_SITE_ID.fetch_add(1, Ordering::SeqCst));
        let next = AtomicPtr::new(std::ptr::null_mut());
        CallSite { id, name, module_path, file, line, next }
    }

    pub(crate) fn id(&self) -> CallSiteId {
        self.id
    }

    /// Get the user-provided name for this call site
    pub fn name(&self) -> &str {
        self.name
    }

    /// Get the rust module path to the source code location of this call site
    pub fn module_path(&self) -> &str {
        self.module_path
    }

    /// Get the path to the file containing this call site
    pub fn file(&self) -> &str {
        self.file
    }

    /// Get the line of the source file containing this call site
    pub fn line(&self) -> u32 {
        self.line
    }

    /// Get the full name of this call site, containing both the name and the
    /// module path.
    pub fn full_name(&self) -> String {
        let mut name = self.module_path.to_owned();
        name += "::";

        if self.name.contains(' ') {
            name += "{";
            name += self.name;
            name += "}";
        } else {
            name += self.name;
        }

        return name;
    }
}

/// Registry of CallSite, as the head pointer of an atomic, append-only linked
/// list.
struct Registry {
    head: AtomicPtr<CallSite>,
}

impl Registry {
    /// Register a new callsite within the list
    fn register(&self, callsite: &'static CallSite) {
        let mut head = self.head.load(Ordering::Acquire);

        loop {
            callsite.next.store(head, Ordering::Release);

            assert_ne!(
                callsite as *const _, head,
                "Attempted to register a `Callsite` that already exists! \
                This will cause an infinite loop when attempting to read from the \
                callsite registry."
            );

            match self.head.compare_exchange(
                head,
                callsite as *const _ as *mut _,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    break;
                }
                Err(current) => head = current,
            }
        }
    }

    /// Execute the provided function on all elements of the list
    fn for_each(&self, mut f: impl FnMut(&'static CallSite)) {
        let mut head = self.head.load(Ordering::Acquire);

        while let Some(registered) = unsafe { head.as_ref() } {
            f(registered);
            head = registered.next.load(Ordering::Acquire);
        }
    }
}

/// Register a call site. This function is a private function of this crate. It
/// is only marked `pub` to be able to call it from inside macros.
#[doc(hidden)]
pub fn register_callsite(callsite: &'static CallSite) {
    REGISTRY.register(callsite);
}

/// Execute the given function on all call sites we know about.
///
/// # Examples
/// ```
/// # use time_graph::traverse_registered_callsite;
///
/// traverse_registered_callsite(|callsite| {
///     println!("got a callsite at {}:{}", callsite.file(), callsite.line());
/// })
/// ```
pub fn traverse_registered_callsite(function: impl FnMut(&'static CallSite)) {
    REGISTRY.for_each(function);
}
