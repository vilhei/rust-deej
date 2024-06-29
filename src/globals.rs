pub const DISPLAY_UPDATE_PERIOD: u32 = 50;
pub const SERIAL_UPDATE_PERIOD: u32 = 500;
pub const MAX_ANALOG_VALUE: u16 = 770;
/// Analog input never really is zero. This value is cutoff, meaning everything under it is interpreted as zero volume
pub const ZERO_CUTOFF: u16 = 35;
pub const INPUT_COUNT: usize = 4;
