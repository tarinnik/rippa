use crate::error::MakeMkvError;

#[derive(Debug, Default)]
pub struct DriveInfo {
    pub drive_id: u32,
    pub drive_name: Option<String>,
    pub disc_name: Option<String>,
    pub device_name: Option<String>,

    pub driveio_tag: Option<String>,
    pub current_profile: Option<String>,
    pub libredrive_info: Option<String>,

    pub disc_timestamp: Option<String>,

    pub disc_has_css: bool,
    pub disc_has_cprm: bool,
    pub disc_has_aacs: bool,
    pub disc_has_bdsvm: bool,

    pub disc_aacs_mkb_version: Option<String>,
    pub disc_aacs_version: Option<String>,
    pub disc_aacs_category: Option<String>,
    pub disc_svm_version: Option<String>,
    pub drive_state: DriveState,

    pub drive_serial_number: Option<String>,
    pub drive_firmware_date: Option<String>,
    pub drive_firmware: Option<String>,
    pub drive_highest_aacs: Option<String>,

    pub disc_capacity: Option<String>,
    pub disc_type: Option<String>,
    pub disc_size: Option<String>,
    pub disc_read_rate: Option<String>,
    pub disc_layers: Option<String>,
    pub disc_layer_orientation: Option<String>,
    pub disc_channel_bit_length: Option<String>,
}

impl DriveInfo {
    pub fn try_from_update(args: &[u32], data: &[u8]) -> Option<Self> {
        if data.len() == 0 || args.len() != 4 {
            return None;
        }

        let drive_id = args[0];
        let drive_state = DriveState::try_from(args[2]).ok()?;

        let disc_fs_flags = args[3];
        let disc_has_aacs = disc_fs_flags & 8 != 0;
        let disc_has_bdsvm = disc_fs_flags & 16 != 0;

        let mut drive_name = None;
        let mut disc_name = None;
        let mut device_name = None;
        let flags = args[1];
        // There are three strings seperated by 0x00.
        let mut data_split = data.splitn(4, |x| *x == 0x00);

        if let Some(flag_data) = data_split.next()
            && flags & 1 == 1
        {
            drive_name = Some(String::from_utf8_lossy(flag_data).into_owned());
        }

        if let Some(flag_data) = data_split.next()
            && flags & 2 == 2
        {
            disc_name = Some(String::from_utf8_lossy(flag_data).into_owned())
        }

        if let Some(flag_data) = data_split.next()
            && flags & 4 == 4
        {
            device_name = Some(String::from_utf8_lossy(flag_data).into_owned())
        }

        // let data = if let Some(d) = data_split.next() {
        //     d
        // } else {
        //     return None;
        // };

        // let mut i = 0;

        Some(Self {
            drive_id,
            drive_state,
            disc_has_aacs,
            disc_has_bdsvm,
            drive_name,
            disc_name,
            device_name,
            ..Default::default()
        })
    }
}

#[derive(Copy, Clone, Debug, Default)]
#[repr(u16)]
pub enum DriveState {
    #[default]
    EmptyClosed = 0,
    EmptyOpen = 1,
    Inserted = 2,
    Loaded = 3,
    NoDrive = 256,
    Unmounting = 257,
}

impl TryFrom<u32> for DriveState {
    type Error = MakeMkvError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::EmptyClosed),
            1 => Ok(Self::EmptyOpen),
            2 => Ok(Self::Inserted),
            3 => Ok(Self::Loaded),
            256 => Ok(Self::NoDrive),
            257 => Ok(Self::Unmounting),
            _ => Err(MakeMkvError::InvalidCommand(format!(
                "{} is not a valid drive state",
                value
            ))),
        }
    }
}
