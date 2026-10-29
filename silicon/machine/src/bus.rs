/// Everything outside the CPU: ROM, ports and the wake signal for a wait.
pub trait Bus {
    fn port_in(&mut self, port: u8) -> u8;
    fn port_out(&mut self, port: u8, value: u8);
    fn rom(&self, addr: u16) -> u16;
    /// Whether a new key press came since the last wait. Taking it clears
    /// it, so the next wait waits for the next press.
    fn take_wake(&mut self) -> bool;
}
