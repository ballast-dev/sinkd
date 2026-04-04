//! Test and local-debug hooks driven by environment variables.
//!
//! | Variable | Effect |
//! |----------|--------|
//! | `SINKD_TEST_RSYNC_FAIL` | When `1` or `true`, rsync returns an error without running. |
//! | `SINKD_TEST_RSYNC_DELAY_MS` | Sleep before rsync spawn (milliseconds). |
//! | `SINKD_TEST_PUBLISH_DELAY_MS` | Delay each outbound Zenoh publish (milliseconds). |
//! | `SINKD_TEST_DROP_EVERY_N` | Drop every N-th outbound Zenoh publish. |
//! | `SINKD_TEST_REORDER_PAIRS` | When `1` or `true`, swap adjacent publishes pairwise (test hook). |

#[must_use]
pub fn env_flag_true(var: &'static str) -> bool {
    std::env::var(var)
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

#[must_use]
pub fn env_u64(var: &'static str) -> Option<u64> {
    std::env::var(var).ok().and_then(|v| v.parse().ok())
}
