pub fn write_mask(&mut self, val: u8) {
    self.set_last_bus_value(val);
    self.mask = val;
}
