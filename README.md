# Time-graph

This crate provides a simple way of extracting the number of time a given
function (or spans inside functions) have been called, how much time have been
spent in each function/span, and record the full "call-graph" between
functions/spans. The indented use case is to extract simple profiling data from
actual runs of a software. Importantly, this crate does not consider different
invocation of the same function/span separately, but instead group all
invocation of functions/span together.

## License and contributions

This crate is distributed under the [3 clauses BSD license](LICENSE). By
contributing to this crate, you agree to distribute your contributions under the
same license.
