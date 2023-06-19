/// See https://shields.io/badges/static-badge
pub(crate) fn shieldsio_static_sanitise(s: String) -> String {
    s.replace('_', "__").replace('-', "--")
}
