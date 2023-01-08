pub trait PacketHandler {
    fn handle(id: &u8, data: [u8]);
}

pub trait SerialPlugin {
    fn tick_serial(&self);
}
