/// ISO 8601 UTC "now" with second precision — the single timestamp source
/// for every mutation (spec §2: `YYYY-MM-DDTHH:MM:SSZ`).
pub fn now_iso8601() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
