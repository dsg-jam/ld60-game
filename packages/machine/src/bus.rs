/// Everything outside the CPU: ROM, ports and the wake signal for halt.
pub trait Bus {
    fn port_in(&mut self, port: u8) -> u8;
    fn port_out(&mut self, port: u8, value: u8);
    fn rom(&self, addr: u16) -> u16;
    fn wake_requested(&self) -> bool;
}
