use crate::error::RippaError;

#[derive(Debug)]
pub struct DriveInfo {
    pub drive_id: u32,
    pub drive_name: String,
    pub disc_name: String,
    pub device_name: String,

    pub driveio_tag: String,
    pub current_profile: String,
    pub libredrive_info: String,

    pub disc_timestamp: String,

    pub disc_has_css: bool,
    pub disc_has_cprm: bool,
    pub disc_has_aacs: bool,
    pub disc_has_bdsvm: bool,

    pub disc_aacs_mkb_version: String,
    pub disc_aacs_version: String,
    pub disc_aacs_category: String,
    pub disc_svm_version: String,
    pub drive_state: String,

    pub drive_serial_number: String,
    pub drive_firmware_date: String,
    pub drive_firmware_string: String,
    pub drive_highest_aacs: String,

    pub disc_capacity: String,
    pub disc_type: String,
    pub disc_size: String,
    pub disc_read_rate: String,
    pub disc_layers: String,
    pub disc_layer_orientation: String,
    pub disc_channel_bit_length: String,
}

#[derive(Copy, Clone, Debug)]
#[repr(u16)]
pub enum DriveState {
    EmptyClosed = 0,
    EmptyOpen = 1,
    Inserted = 2,
    Loaded = 3,
    NoDrive = 256,
    Unmounting = 257,
}

impl TryFrom<u16> for DriveState {
    type Error = RippaError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::EmptyClosed),
            1 => Ok(Self::EmptyOpen),
            2 => Ok(Self::Inserted),
            3 => Ok(Self::Loaded),
            256 => Ok(Self::NoDrive),
            257 => Ok(Self::Unmounting),
            _ => Err(RippaError::InvalidMmkvCommand(format!(
                "{} is not a valid drive state",
                value
            ))),
        }
    }
}
