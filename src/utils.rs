pub(crate) trait SpaceapiStatusExt {
    fn ensure_sensors_struct_exists(&mut self);
}

impl SpaceapiStatusExt for spaceapi::Status {
    fn ensure_sensors_struct_exists(&mut self) {
        if self.sensors.is_none() {
            self.sensors = Some(spaceapi::sensors::Sensors::default());
        }
    }
}

/// See https://shields.io/badges/static-badge
pub(crate) fn shieldsio_static_sanitise(s: String) -> String {
    s.replace('_', "__").replace('-', "--")
}
