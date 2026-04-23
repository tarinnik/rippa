use crate::{error::MakeMkvError, util::u32_const_slice};
use anyhow::{Context, ensure};
use chrono::{DateTime, TimeZone, Utc, offset::LocalResult};
use log::warn;

// const DRIVE_INFO_CATEGORY_INVALID: u32 = 0;
const DRIVE_INFO_CATEGORY_DRIVE_STANDARD: u32 = 1;
const DRIVE_INFO_CATEGORY_DRIVE_SPECIFIC: u32 = 2;
const DRIVE_INFO_CATEGORY_DISC_STANDARD: u32 = 3;
// const DRIVE_INFO_CATEGORY_DISC_SPECIFIC: u32 = 4;
// const DRIVE_INFO_CATEGORY_USER_PRIVATE: u32 = 5;

#[derive(Debug, Default)]
pub struct DriveInfo {
    pub drive_id: u32,
    pub drive_name: Option<String>,
    pub disc_name: Option<String>,
    pub device_name: Option<String>,

    pub driveio_tag: Option<String>,
    pub current_profile: Option<String>,
    pub libredrive_info: Vec<String>,

    pub disc_timestamp: Option<DateTime<Utc>>,

    pub disc_has_css: bool,
    pub disc_has_cprm: bool,
    pub disc_has_aacs: bool,
    pub disc_has_bdsvm: bool,

    pub disc_aacs_mkb_version: Option<u32>,
    pub disc_aacs_version: Option<String>,
    pub disc_aacs_category: Option<String>,
    pub disc_svm_version: Option<String>,
    pub drive_state: DriveState,

    pub drive_serial_number: Option<String>,
    pub drive_firmware_date: Option<DateTime<Utc>>,
    pub drive_firmware: Option<String>,
    pub drive_highest_aacs: Option<u32>,

    pub disc_capacity: Option<f64>,
    pub disc_type: Option<String>,
    pub disc_size: Option<String>,
    pub disc_read_rate: Option<f64>,
    pub disc_layers: Option<u8>,
    pub disc_layer_orientation: Option<String>,
    pub disc_channel_bit_length: Option<String>,

    // Manufacturer data
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub revision: Option<String>,
}

impl DriveInfo {
    pub fn try_from_update(args: &[u32], data: &[u8]) -> anyhow::Result<Self> {
        ensure!(
            !data.is_empty() && args.len() >= 4,
            "Invalid args or data length"
        );

        let drive_id = args[0];
        let drive_state = DriveState::try_from(args[2])?;

        let disc_fs_flags = args[3];
        let disc_has_aacs = disc_fs_flags & 8 != 0;
        let disc_has_bdsvm = disc_fs_flags & 16 != 0;

        let mut drive_name = None;
        let mut disc_name = None;
        let mut device_name = None;
        let flags = args[1];
        // There are three strings seperated by 0x00 and then the remaining data
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

        let data = data_split
            .next()
            .context("Data didn't contain enough strings")?;
        let mut drive = Self {
            drive_id,
            drive_state,
            disc_has_aacs,
            disc_has_bdsvm,
            drive_name,
            disc_name,
            device_name,
            ..Default::default()
        };

        let mut start_index = 0;
        let mut end_index = 0;
        while end_index < data.len() {
            start_index = end_index;
            ensure!(start_index + 8 <= data.len(), "Not enough data");

            let command_id =
                u32::from_be_bytes(u32_const_slice(&data[start_index..start_index + 4]));
            let data_size =
                u32::from_be_bytes(u32_const_slice(&data[start_index + 4..start_index + 8]))
                    as usize;

            start_index += 8;
            end_index = start_index + data_size;

            ensure!(end_index <= data.len(), "Not enough data");

            match command_id {
                DriveInfoId::DRIVE_IO_TAG => {
                    drive.driveio_tag = bytes_to_string(&data[start_index..end_index - 1])
                }
                DriveInfoId::INQUIRY_DATA => {
                    if data_size >= 36 {
                        drive.manufacturer =
                            bytes_to_string(&data[start_index + 8..start_index + 16]);
                        drive.product = bytes_to_string(&data[start_index + 16..start_index + 32]);
                        drive.revision = bytes_to_string(&data[start_index + 32..start_index + 36]);
                    }
                }
                DriveInfoId::FEATURE_DESCRIPTOR_DRIVE_SERIAL_NUMBER => {
                    ensure!(data_size >= 4, "Not enough drive serial number data");
                    let size = data[start_index + 3] as usize;

                    ensure!(data_size >= 4 + size, "Not enough drive serial number data");
                    drive.drive_serial_number =
                        bytes_to_string(&data[start_index + 4..start_index + 4 + size]);
                }
                DriveInfoId::FEATURE_DESCRIPTOR_FIRMWARE_INFORMATION => {
                    ensure!(data_size == 20, "Not enough firmware information data");
                    drive.drive_firmware_date = parse_timestamp(&data[start_index..end_index]);
                }
                DriveInfoId::FIRMWARE_DETAILS_STRING => {
                    drive.drive_firmware = bytes_to_string(&data[start_index..end_index]);
                }
                DriveInfoId::CURRENT_PROFILE => {
                    ensure!(data_size >= 2, "Not enough data for current profile");
                    let profile_id = u16::from_be_bytes([data[start_index], data[start_index + 1]]);
                    drive.current_profile = Some(get_mmc_profile_string(profile_id));
                }
                DriveInfoId::DISC_CAPACITY => {
                    ensure!(data_size >= 4, "Not enough data for disc capacity");
                    let sec_size =
                        u32::from_be_bytes(u32_const_slice(&data[start_index..start_index + 4]));
                    drive.disc_capacity = Some(sec_size as f64 / 1024.0 / 512.0);
                }
                DriveInfoId::DISC_STRUCTURE_DVD_COPYRIGHT_INFORMATION => {
                    ensure!(
                        data_size >= 5,
                        "Not enough data for DVD copyright information"
                    );
                    match data[start_index + 4] {
                        1 => drive.disc_has_css = true,
                        2 => drive.disc_has_cprm = true,
                        3 | 16 => drive.disc_has_aacs = true,
                        _ => {}
                    }
                }
                DriveInfoId::DISC_STRUCTURE_DVD_PHYSICAL_FORMAT => {
                    ensure!(data_size >= 7, "Not enough data for DVD physical format");
                    drive.disc_type = Some(get_disc_format(data[start_index + 4] >> 4));

                    drive.disc_size = match data[start_index + 5] >> 4 {
                        0 => Some("120 mm".into()),
                        1 => Some("80 mm".into()),
                        _ => None,
                    };

                    drive.disc_read_rate = match data[start_index + 5] & 0x0F {
                        0 => Some(0.25),
                        1 => Some(0.5),
                        2 => Some(1.0),
                        3 => Some(2.0),
                        4 => Some(3.0),
                        _ => None,
                    };

                    drive.disc_layers = Some(1 + ((data[start_index + 6] >> 5) & 3));
                    drive.disc_layer_orientation = Some(if data[start_index + 6] & 16 == 0 {
                        "PTP".into()
                    } else {
                        "OTP".into()
                    });
                }
                DriveInfoId::DISC_STRUCTURE_BD_DISC_INFORMATION => {
                    if data_size < 20 {
                        warn!("Not enough data for BD disc info, data size: {}", data_size);
                        continue;
                    }
                    let data_check1 = &data[start_index + 4..start_index + 7];
                    let data_check2 = &data[start_index + 12..start_index + 15];
                    if data_check1 == b"DI\x01"
                        && matches!(data_check2, b"BDO" | b"BDW" | b"BDR" | b"BDU")
                    {
                        drive.disc_type = match data[start_index + 16] & 0x0F {
                            1 => Some("DB-ROM".into()),
                            2 => Some("BD-R".into()),
                            4 => Some("BD-RE".into()),
                            9 => Some("BD-ROM UHD".into()),
                            _ => None,
                        };

                        drive.disc_layers = Some(data[start_index + 16] >> 4);
                        drive.disc_channel_bit_length = match data[start_index + 17] & 0x0F {
                            1 => Some("74.5 nm".into()),
                            2 => Some("69.0 nm".into()),
                            _ => None,
                        };
                    }
                }
                0x05102201 => {
                    if data_size < 12 || data[start_index] != 0x10 {
                        continue;
                    }
                    drive.disc_aacs_mkb_version = Some(u32::from_be_bytes(u32_const_slice(
                        &data[start_index + 8..start_index + 12],
                    )));
                    drive.disc_aacs_version = match data[start_index + 5] {
                        10 => Some("1.0/II".into()),
                        20 => Some("2.0".into()),
                        21 => Some("2.1".into()),
                        _ => Some("1.0".into()),
                    }
                }
                0x05102202 => {
                    if data_size < 17 {
                        continue;
                    }
                    let svm_year =
                        u16::from_be_bytes([data[start_index + 13], data[start_index + 14]]);
                    let svm_month = data[start_index + 15];
                    drive.disc_svm_version = Some(format!("{}.{}", svm_year, svm_month));
                }
                0x05102203 => {
                    if data_size < 4 {
                        continue;
                    }
                    drive.disc_aacs_category = match data[start_index + 1] {
                        0 => Some("C".into()),
                        1 => Some("B".into()),
                        2 => Some("A".into()),
                        _ => None,
                    }
                }
                0x05102204 => {
                    if data_size != 4 {
                        continue;
                    }
                    drive.drive_highest_aacs = Some(u32::from_be_bytes(u32_const_slice(
                        &data[start_index..end_index],
                    )));
                }
                0x05102205 => {
                    if data_size != 20 {
                        continue;
                    }
                    drive.disc_timestamp = parse_timestamp(&data[start_index..end_index]);
                }
                0x05102210 => {
                    drive.libredrive_info =
                        String::from_utf8_lossy(&data[start_index..end_index - 2])
                            .split('\n')
                            .skip(1)
                            .map(|s| s.to_string())
                            .collect();
                }
                _ => warn!("Unknown command ID: {}", command_id),
            }
        }

        Ok(drive)
    }
}

fn bytes_to_string(bytes: &[u8]) -> Option<String> {
    Some(String::from_utf8_lossy(bytes).trim().to_string())
}

/// Parses a timestamp from bytes.
///
/// Assumes the bytes arguement has a length of 20.
fn parse_timestamp(bytes: &[u8]) -> Option<DateTime<Utc>> {
    let year = int_from_str_bytes(&bytes[4..8])?;
    let month = int_from_str_bytes(&bytes[8..10])?;
    let day = int_from_str_bytes(&bytes[10..12])?;
    let hour = int_from_str_bytes(&bytes[12..14])?;
    let minute = int_from_str_bytes(&bytes[14..16])?;
    let second = if bytes[16] != 0x00 && bytes[16] != 0x20 {
        int_from_str_bytes(&bytes[16..18])?
    } else {
        0
    };

    match Utc.with_ymd_and_hms(year as i32, month, day, hour, minute, second) {
        LocalResult::Single(d) => Some(d),
        LocalResult::Ambiguous(d, _) => Some(d),
        LocalResult::None => None,
    }
}

fn int_from_str_bytes(bytes: &[u8]) -> Option<u32> {
    String::from_utf8_lossy(bytes).parse().ok()
}

fn get_mmc_profile_string(id: u16) -> String {
    match id {
        8 => "CD-ROM".into(),
        9 => "CD-R".into(),
        10 => "CD-RW".into(),
        16 => "DVD-ROM".into(),
        17 => "DVD-R".into(),
        18 => "DVD-RAM".into(),
        19 => "DVD-RW".into(),
        20 => "DVD-RW".into(),
        21 => "DVD-R DL SR".into(),
        22 => "DVD-R DL JR".into(),
        23 => "DVD-RW DL".into(),
        26 => "DVD+RW".into(),
        27 => "DVD+R".into(),
        42 => "DVD+RW DL".into(),
        43 => "DVD+R DL".into(),
        64 => "BD-ROM".into(),
        65 => "BD-R SRM".into(),
        66 => "BD-R RRM".into(),
        67 => "BD-RE".into(),
        80 => "HD DVD-ROM".into(),
        81 => "HD DVD-R".into(),
        82 => "HD DVD-RAM".into(),
        83 => "HD DVD-RW".into(),
        88 => "HD DVD-R DL".into(),
        90 => "HD DVD-RW DL".into(),
        _ => "invalid".into(),
    }
}

fn get_disc_format(id: u8) -> String {
    match id {
        0 => "DVD-ROM".into(),
        1 => "DVD-RAM".into(),
        2 => "DVD-R".into(),
        3 => "DVD-RW".into(),
        4 => "HD DVD-ROM".into(),
        5 => "HD DVD-RAM".into(),
        6 => "HD DVD-R".into(),
        9 => "DVD+RW".into(),
        10 => "DVD+R".into(),
        11 => "DVD+RW DL".into(),
        12 => "DVD+R DL".into(),
        _ => "invalid".into(),
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
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

struct DriveInfoId;
impl DriveInfoId {
    // Invalid
    // const INVALID: u32 = DRIVE_INFO_CATEGORY_INVALID;
    const DRIVE_IO_TAG: u32 = 1 << 16;
    // const DRIVE_IO_PAD: u32 = 2 << 16;

    // Drive Standard
    const INQUIRY_DATA: u32 = (DRIVE_INFO_CATEGORY_DRIVE_STANDARD << 24);
    // const FEATURE_DESCRIPTOR: u32 = (DRIVE_INFO_CATEGORY_DRIVE_STANDARD << 24) + (1 << 16);
    const FEATURE_DESCRIPTOR_DRIVE_SERIAL_NUMBER: u32 =
        (DRIVE_INFO_CATEGORY_DRIVE_STANDARD << 24) + (1 << 16) + 0x108;
    const FEATURE_DESCRIPTOR_FIRMWARE_INFORMATION: u32 =
        (DRIVE_INFO_CATEGORY_DRIVE_STANDARD << 24) + (1 << 16) + 0x10c;
    // const FEATURE_DESCRIPTOR_AACS: u32 =
    //     (DRIVE_INFO_CATEGORY_DRIVE_STANDARD << 24) + (1 << 16) + 0x10d;
    const CURRENT_PROFILE: u32 = (DRIVE_INFO_CATEGORY_DRIVE_STANDARD << 24) + (2 << 16);
    // const DRIVE_CERT: u32 = (DRIVE_INFO_CATEGORY_DRIVE_STANDARD << 24) + (3 << 16) + 0x38;

    //Drive Specific
    const FIRMWARE_DETAILS_STRING: u32 = (DRIVE_INFO_CATEGORY_DRIVE_SPECIFIC << 24) + 1;
    // const FIRMWARE_VENDOR_SPECIFIC_INFO: u32 = (DRIVE_INFO_CATEGORY_DRIVE_SPECIFIC << 24) + 2;
    // const FIRMWARE_FLASH_IMAGE: u32 = (DRIVE_INFO_CATEGORY_DRIVE_SPECIFIC << 24) + 3;

    // Disc Standard
    const DISC_STRUCTURE_DVD_PHYSICAL_FORMAT: u32 = (DRIVE_INFO_CATEGORY_DISC_STANDARD << 24);
    const DISC_STRUCTURE_DVD_COPYRIGHT_INFORMATION: u32 =
        (DRIVE_INFO_CATEGORY_DISC_STANDARD << 24) + 0x001;
    const DISC_STRUCTURE_BD_DISC_INFORMATION: u32 =
        (DRIVE_INFO_CATEGORY_DISC_STANDARD << 24) + 0x100;
    // const TOC: u32 = (DRIVE_INFO_CATEGORY_DISC_STANDARD << 24) + (1 << 16);
    // const DISC_INFORMATION: u32 = (DRIVE_INFO_CATEGORY_DISC_STANDARD << 24) + (2 << 16);
    const DISC_CAPACITY: u32 = (DRIVE_INFO_CATEGORY_DISC_STANDARD << 24) + (3 << 16);

    // Disc Specific
    // const AACS: u32 = (DRIVE_INFO_CATEGORY_DISC_SPECIFIC << 24);
    // const AACS_VID: u32 = (DRIVE_INFO_CATEGORY_DISC_SPECIFIC << 24) + 0x80;
    // const AACS_KCD: u32 = (DRIVE_INFO_CATEGORY_DISC_SPECIFIC << 24) + 0x7f;
    // const AACS_PMSN: u32 = (DRIVE_INFO_CATEGORY_DISC_SPECIFIC << 24) + 0x81;
    // const AACS_MID: u32 = (DRIVE_INFO_CATEGORY_DISC_SPECIFIC << 24) + 0x82;
    // const AACS_DATA_KEYS: u32 = (DRIVE_INFO_CATEGORY_DISC_SPECIFIC << 24) + 0x84;
    // const AACS_BEEXTENTS: u32 = (DRIVE_INFO_CATEGORY_DISC_SPECIFIC << 24) + 0x85;
    // const AACS_BINDING_NONCE: u32 = (DRIVE_INFO_CATEGORY_DISC_SPECIFIC << 24) + 0x7e;
}
