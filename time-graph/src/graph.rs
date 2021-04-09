use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::BTreeMap;
use std::time::Duration;
use std::cell::RefCell;

use once_cell::sync::Lazy;
use quanta::Clock;
use petgraph::graph::{Graph, NodeIndex};

use crate::{CallSite, CallSiteId};

/// Global clock to record start/end times
static CLOCK: Lazy<Clock> = Lazy::new(Clock::new);

/// Global call graph, including recorded timings and calls count
static CALL_GRAPH: Lazy<Mutex<LightCallGraph>> = Lazy::new(|| {
    Mutex::new(LightCallGraph::new())
});

/// Should we collect data?
static COLLECTION_ENABLED: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// For each thread, which span is currently executing? This will become the
    /// parent of new spans.
    pub static LOCAL_CURRENT_SPAN: RefCell<Option<CallSiteId>> = RefCell::new(None);
}

/// A [`Span`] records a single execution of code associated with a
/// [`CallSite`].
///
/// This is not usually constructed manually but with either the
/// [`macro@spanned`] or [`instrument`](attr.instrument.html) macros.
pub struct Span {
    callsite: &'static CallSite,
}

impl Span {
    /// Create a new [`Span`] associated with the given `callsite`.
    pub fn new(callsite: &'static CallSite) -> Span {
        Span {
            callsite: callsite,
        }
    }

    /// Enter the span, the span will automatically be exited when the
    /// [`SpanGuard`] is dropped.
    #[must_use]
    pub fn enter(&self) -> SpanGuard<'_> {
        if !COLLECTION_ENABLED.load(Ordering::Acquire) {
            return SpanGuard {
                span: &self,
                parent: None,
                start: 0,
            };
        }

        let id = self.callsite.id();
        let parent = LOCAL_CURRENT_SPAN.with(|parent| {
            let mut parent = parent.borrow_mut();

            let previous = *parent;
            *parent = Some(id);
            return previous;
        });

        SpanGuard {
            span: &self,
            parent: parent,
            start: CLOCK.start(),
        }
    }
}

/// When a [`SpanGuard`] is dropped, it saves the execution time of the
/// corresponding span in the global call graph.
pub struct SpanGuard<'a> {
    span: &'a Span,
    parent: Option<CallSiteId>,
    start: u64,
}

impl<'a> Drop for SpanGuard<'a>  {
    fn drop(&mut self) {
        if !COLLECTION_ENABLED.load(Ordering::Acquire) {
            return;
        }
        let elapsed = CLOCK.delta(self.start, CLOCK.end());

        LOCAL_CURRENT_SPAN.with(|parent| {
            let mut parent = parent.borrow_mut();
            *parent = self.parent;
        });


        let mut graph = CALL_GRAPH.lock().expect("poisoned mutex");
        let callsite = self.span.callsite.id();
        graph.add_node(callsite);
        graph.increase_timing(callsite, elapsed);

        if let Some(parent) = self.parent {
            graph.add_node(parent);
            graph.increase_call_count(parent, callsite);
        }
    }
}

/// Call graph node identifying their call site with its `CallSiteId`.
struct LightGraphNode {
    callsite: CallSiteId,
    elapsed: Duration,
    called: u32,
}

impl LightGraphNode {
    fn new(callsite: CallSiteId) -> LightGraphNode {
        LightGraphNode {
            callsite: callsite,
            elapsed: Duration::new(0, 0),
            called: 0,
        }
    }
}

/// Simple Call graph, identifying call site with their `CallSiteId`.
///
/// The graph nodes are spans with associated timings, while the edges represent
/// the number of calls from one node to the other.
struct LightCallGraph {
    graph: Graph<LightGraphNode, usize>
}

impl LightCallGraph {
    fn new() -> LightCallGraph {
        LightCallGraph {
            graph: Graph::new(),
        }
    }

    pub fn clear(&mut self) {
        self.graph.clear()
    }

    /// Find a node in the graph with its `CallSiteId`.
    fn find(&mut self, callsite: CallSiteId) -> Option<NodeIndex> {
        for id in self.graph.node_indices() {
            if self.graph[id].callsite == callsite {
                return Some(id);
            }
        }
        return None;
    }

    /// Add a node for the given callsite to the graph, do nothing if there is
    /// already such a node
    pub fn add_node(&mut self, callsite: CallSiteId) {
        if self.find(callsite).is_none() {
            self.graph.add_node(LightGraphNode::new(callsite));
        }
    }

    /// Increase the number of time the `parent` span called the `child` span
    /// by one.
    pub fn increase_call_count(&mut self, parent: CallSiteId, child: CallSiteId) {
        let parent = self.find(parent).expect("missing node for parent");
        let child = self.find(child).expect("missing node for child");
        if let Some(edge) = self.graph.find_edge(parent, child) {
            let count = self
                .graph
                .edge_weight_mut(edge)
                .expect("failed to get edge weights");
            *count += 1;
        } else {
            // initialize edge count to 1
            self.graph.add_edge(parent, child, 1);
        }
    }

    /// Increase the timing associated with a span by `time`, and the number of
    /// time this span has been called by one.
    pub fn increase_timing(&mut self, span: CallSiteId, time: Duration) {
        let id = self.find(span).expect("missing node");
        self.graph[id].elapsed += time;
        self.graph[id].called += 1;
    }
}

/// Clear the global call graph from all data
pub fn clear_collected_data() {
    CALL_GRAPH.lock().expect("poisoned mutex").clear();
}

/// Enable/disable data collection
pub fn enable_data_collection(enabled: bool) {
    COLLECTION_ENABLED.store(enabled, Ordering::Release);
}

/// Get a copy of the call graph as currently known
pub fn get_full_graph() -> FullCallGraph {
    let graph = CALL_GRAPH.lock().expect("poisoned mutex");

    let mut all_callsites = BTreeMap::new();
    crate::traverse_registered_callsite(|callsite| {
        all_callsites.insert(callsite.id(), callsite);
    });

    let graph = graph.graph.map(|index, node| {
        TimedSpan::new(node, index.index(), all_callsites[&node.callsite])
    }, |_, &edge| edge);

    return FullCallGraph {
        graph: graph
    };
}

/// [`TimedSpan`] contains all data related to a single function or span inside
/// the global call graph.
pub struct TimedSpan {
    /// Unique identifier of this function/span in the call graph
    pub id: usize,
    /// [`CallSite`] associated with this function/span
    pub callsite: &'static CallSite,
    /// Total elapsed time inside this function/span
    pub elapsed: Duration,
    /// Number of times this function/span have been called
    pub called: u32,
}

impl TimedSpan {
    fn new(node: &LightGraphNode, id: usize, callsite: &'static CallSite) -> TimedSpan {
        TimedSpan {
            id: id,
            callsite: callsite,
            elapsed: node.elapsed,
            called: node.called,
        }
    }
}

impl std::fmt::Display for TimedSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ran for {:?}, called {} times",
            self.callsite.full_name(), self.elapsed, self.called
        )
    }
}

/// Full call graph including execution time and number of calls between
/// functions/spans.
///
/// This graph is a directed graph linking different `SpanTiming` by the
/// number of time a given span was the child of another one.
///
/// # Examples
///
/// Code that looks like this
/// ```no_run
/// #[time_graph::instrument]
/// fn start() {
///     inside();
///     inside();
///     inner();
/// }
///
/// #[time_graph::instrument]
/// fn inside() {
///    inner();
/// }
///
/// #[time_graph::instrument]
/// fn inner() {
///     // do stuff
/// }
/// ```
///
/// Will result in a graph like this, where the number near the edge correspond
/// to the number of time a given span called another one.
/// ```bash no_run
///             | start, called 1 |
///                /           |
///              /  2          |
///            /               |  1
///   | inside, called 2 |     |
///                 \          |
///                 2 \        |
///                     \      |
///                  | inner, called 3 |
/// ```
pub struct FullCallGraph {
    graph: Graph<TimedSpan, usize>
}

/// A set of calls from one function/span to another
pub struct Calls {
    /// the outer/calling function/span
    pub caller: usize,
    /// the inner/called function/span
    pub callee: usize,
    /// number of time the inner function/span have been called by the outer one
    pub count: usize,
}

impl FullCallGraph {
    /// Get the full list of spans/functions known by this graph
    pub fn spans(&self) -> impl Iterator<Item = &TimedSpan> {
        self.graph.raw_nodes().iter().map(|node| &node.weight)
    }

    /// Get the list of calls between spans in this graph
    pub fn calls(&self) -> impl Iterator<Item = Calls> + '_ {
        self.graph.raw_edges().iter().map(|edge| Calls {
            caller: edge.target().index(),
            callee: edge.source().index(),
            count: edge.weight,
        })
    }

    /// Get the full graph in [graphviz](https://graphviz.org/) dot format.
    ///
    /// The exact output is unstable and should not be relied on.
    pub fn as_dot(&self) -> String {
        petgraph::dot::Dot::new(&self.graph).to_string()
    }

    /// Get a per span summary table of this graph.
    ///
    /// The exact output is unstable and should not be relied on.
    ///
    /// This function is only available if the `"table"` cargo feature is enabled
    ///
    /// # Panic
    ///
    /// This function will panic if the graph is cyclical, i.e. if two or more
    /// span are mutually recursive.
    #[cfg(feature = "table")]
    pub fn as_table(&self) -> String {
        use petgraph::Direction;

        use term_table::row::Row;
        use term_table::table_cell::{Alignment, TableCell};

        let mut table = term_table::Table::new();
        table.style = term_table::TableStyle::extended();

        table.add_row(Row::new(vec![
            "id",
            // pad "span name" to make the table look nicer with short names
            "span name                                   ",
            "call count",
            "called by",
            "total",
            "mean",
        ]));

        for &node_id in petgraph::algo::kosaraju_scc(&self.graph)
            .iter()
            .rev()
            .flatten()
        {
            let node = &self.graph[node_id];

            let mut called_by = vec![];
            for other in self.graph.neighbors_directed(node_id, Direction::Incoming) {
                called_by.push(self.graph[other].id.to_string());
            }
            let called_by = if !called_by.is_empty() {
                called_by.join(", ")
            } else {
                "—".into()
            };

            let mean = node.elapsed / node.called;
            let warn = if mean < Duration::from_nanos(1500) { " ⚠️ " } else { "" };

            table.add_row(Row::new(vec![
                TableCell::new_with_alignment(self.graph[node_id].id, 1, Alignment::Right),
                TableCell::new(&node.callsite.full_name()),
                TableCell::new_with_alignment(node.called, 1, Alignment::Right),
                TableCell::new_with_alignment(called_by, 1, Alignment::Right),
                TableCell::new_with_alignment(
                    &format!("{:.2?}", node.elapsed),
                    1,
                    Alignment::Right,
                ),
                TableCell::new_with_alignment(
                    &format!("{:.2?}{}", mean, warn),
                    1,
                    Alignment::Right,
                ),
            ]));
        }

        return table.render();
    }

    /// Get all the data in this graph in JSON.
    ///
    /// The exact output is unstable and should not be relied on.
    ///
    /// This function is only available if the `"json"` cargo feature is enabled
    #[cfg(feature = "json")]
    pub fn as_json(&self) -> String {
        let mut spans = json::JsonValue::new_object();
        for span in self.spans() {
            spans[&span.callsite.full_name()] = json::object! {
                "id" => span.id,
                "elapsed" => format!("{:?}", span.elapsed),
                "called" => span.called,
            };
        }

        let mut all_calls = json::JsonValue::new_array();
        for call in self.calls() {
            all_calls.push(json::object! {
                "caller" => call.caller,
                "callee" => call.caller,
                "count" => call.count,
            }).expect("failed to add edge information to JSON");
        }

        return json::stringify(json::object! {
            "timings" => spans,
            "calls" => all_calls,
        });
    }
}
