//! Block-scoped may_share edges.
//!
//! `scope_pop` used to restore `may_share` from the snapshot taken at
//! `scope_push`, discarding every edge created inside the block — including
//! edges between bindings declared OUTSIDE the block. An assignment inside an
//! `if` therefore laundered the alias: `move alias` did not freeze `shared`,
//! and the emitted Go wrote one map from two goroutines.

use hy_check::{check_file, Verdict};
use hy_diag::Severity;
use hy_syntax::parse_file;

fn check(src: &str) -> hy_check::CheckResult {
    let ast = parse_file("test.hy", src).expect("snippet parses");
    check_file(&ast)
}

fn first_error_rule(r: &hy_check::CheckResult) -> Option<String> {
    r.diagnostics
        .iter()
        .find(|d| d.severity == Severity::Error)
        .and_then(|d| d.tier0_rule.clone())
}

#[test]
fn block_scoped_alias_freezes_on_move_e3003() {
    // The edge is created inside the `if` block; both bindings are declared
    // outside and outlive it. The post-spawn write must be caught.
    let r = check(
        "fn worker(m map[string]int) {\n\
           m[\"task\"]++\n\
         }\n\
         fn main() {\n\
           shared := map[string]int{}\n\
           var alias map[string]int\n\
           if len(shared) == 0 {\n\
             alias = shared\n\
           }\n\
           task worker(move alias)\n\
           shared[\"main\"]++\n\
         }",
    );
    assert_eq!(r.verdict, Verdict::Reject, "{:?}", r.diagnostics);
    assert_eq!(first_error_rule(&r).as_deref(), Some("A1b"));
    assert!(
        r.diagnostics.iter().any(|d| d.code == "HYB-E3003"),
        "expected HYB-E3003, got {:?}",
        r.diagnostics
    );
}

#[test]
fn block_scoped_alias_without_post_use_stays_accept() {
    // Positive control: the same block-scoped assignment, but the source is
    // never used after the move, so nothing races. The fix must not turn this
    // into an eager rejection.
    let r = check(
        "fn worker(m map[string]int) {\n\
           m[\"task\"]++\n\
         }\n\
         fn main() {\n\
           shared := map[string]int{}\n\
           var alias map[string]int\n\
           if len(shared) == 0 {\n\
             alias = shared\n\
           }\n\
           task worker(move alias)\n\
           fmt.println(\"spawned\")\n\
         }",
    );
    assert_eq!(r.verdict, Verdict::Accept, "{:?}", r.diagnostics);
}

#[test]
fn block_local_map_does_not_create_a_false_edge() {
    // A map declared INSIDE the block must not survive the pop and must not
    // link the outer bindings to anything.
    let r = check(
        "fn worker(m map[string]int) {\n\
           m[\"task\"]++\n\
         }\n\
         fn main() {\n\
           shared := map[string]int{}\n\
           var alias map[string]int\n\
           alias = shared\n\
           if len(shared) == 0 {\n\
             local := map[string]int{}\n\
             local[\"x\"] = 1\n\
             fmt.println(\"{len(local)}\")\n\
           }\n\
           task worker(move alias)\n\
           fmt.println(\"spawned\")\n\
         }",
    );
    assert_eq!(r.verdict, Verdict::Accept, "{:?}", r.diagnostics);
}

#[test]
fn block_scoped_rebind_of_struct_still_rejects() {
    // Same defect through a composite: the rebind happens inside the block,
    // and the post-spawn write is still a race.
    let r = check(
        "struct holder {\n\
           m map[string]int\n\
         }\n\
         fn worker(h holder) {\n\
           h.m[\"task\"]++\n\
         }\n\
         fn main() {\n\
           shared := map[string]int{}\n\
           var h holder\n\
           if len(shared) == 0 {\n\
             h = holder{m: shared}\n\
           }\n\
           task worker(move h)\n\
           shared[\"main\"]++\n\
         }",
    );
    assert_eq!(r.verdict, Verdict::Reject, "{:?}", r.diagnostics);
    assert!(
        r.diagnostics.iter().any(|d| d.code == "HYB-E3003"),
        "expected HYB-E3003, got {:?}",
        r.diagnostics
    );
}
