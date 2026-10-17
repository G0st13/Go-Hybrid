// ============================================================================
// 53-block-scoped-alias-at-spawn — Checker corpus #53
//
// PURPOSE: negative control for block-scoped aliasing. A `may_share` edge is
// materialized when a binding is assigned an aliasing value, but `scope_pop`
// restored `may_share` from the snapshot taken at `scope_push`, which dropped
// every edge created inside the block — including edges between two bindings
// that were declared OUTSIDE the block and both outlive it.
//
// Nothing is shadowed here and there are no composites: this is the smallest
// form of the escape. Moving `alias` must freeze `shared`, because both name
// the same map.
//
// ---- Tier-0 rule set applied to each value ----
// Layer 1 (Go-side GC): `shared` is a mutable map; `alias = shared` inside the
//   `if` block makes `alias` an alias of it. `move alias` at the spawn must
//   therefore freeze the live alias `shared` (O5.2).
// Layer 2/3: none.
//
// ---- Sendability / check sites ----
//   [S1] `task worker(move alias)` — owned move of an alias of `shared`.
//   [S2] `shared["main"]++` after the spawn — a WRITE through the frozen alias.
//
// ---- Expected verdict ----
// EXPECTED-VERDICT: REJECT
//   REJECT — A1b/E3003 (write through a live alias of a moved value).
//   Accepting this program is a P0: the very same program with the assignment
//   moved OUT of the block rejects, and the emitted Go writes one map from two
//   goroutines (`fatal error: concurrent map writes`, reproduced 3/3).
//
// ---- Annotations / managed ----
//   lifetime annotations: 0. `managed` blocks: 0.
// ============================================================================

fn worker(m map[string]int) {
    m["task"]++
}

fn main() {
    shared := map[string]int{}
    var alias map[string]int

    if len(shared) == 0 {
        alias = shared              // the edge created inside this block
    }

    task worker(move alias)         // [S1] spawn site

    shared["main"]++                // [S2] alias write after the move
}
