use focus_planner_lib::core::time::now_iso8601;

#[test]
fn now_iso8601_is_utc_second_precision_iso8601() {
    let ts = now_iso8601();
    // exact shape: YYYY-MM-DDTHH:MM:SSZ (20 chars), per spec §2
    assert_eq!(ts.len(), 20, "got {ts}");
    let bytes = ts.as_bytes();
    assert_eq!(bytes[4], b'-');
    assert_eq!(bytes[7], b'-');
    assert_eq!(bytes[10], b'T');
    assert_eq!(bytes[13], b':');
    assert_eq!(bytes[16], b':');
    assert_eq!(bytes[19], b'Z');
    assert!(ts[0..4].chars().all(|c| c.is_ascii_digit()));
}
