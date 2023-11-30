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
