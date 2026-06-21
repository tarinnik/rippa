use crate::error::MakeMkvError;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, byteorder::little_endian::U16};

#[derive(Copy, Clone, Debug)]
pub enum MakeMkvCommand {
    Noop = 0,
    Return = 1,
    ClientDone = 2,
    CallSignalExit = 3,
    CallOnIdle = 4,
    // CallCancelAllJobs = 4,
    CallSetOutputFolder = 16,
    CallUpdateAvailableDrives = 17,
    CallOpenFile = 18,
    CallOpenDisk = 19,
    CallOpenTitleCollection = 20,
    CallCloseDisk = 21,
    CallEjectDisk = 22,
    CallSaveAllSelectedTitlesToMkv = 23,
    CallGetUiItemState = 24,
    CallSetUiItemState = 25,
    CallGetUiItemInfo = 26,
    CallGetSettingInt = 27,
    CallGetSettingString = 28,
    CallSetSettingInt = 29,
    CallSetSettingString = 30,
    CallSaveSettings = 31,
    CallAppGetString = 32,
    CallBackupDisc = 33,
    CallGetInterfaceLanguageData = 34,
    CallSetUiItemInfo = 35,
    CallSetProfile = 36,
    CallInitMMBD = 37,
    CallOpenMMBD = 38,
    CallDiscInfoMMBD = 39,
    CallDecryptUnitMMBD = 40,
    CallSetExternAppFlags = 41,
    CallManageState = 42,
    CallAppSetString = 43,
    BackEnterJobMode = 192,
    BackLeaveJobMode = 193,
    BackUpdateDrive = 194,
    BackUpdateCurrentBar = 195,
    BackUpdateTotalBar = 196,
    BackUpdateLayout = 197,
    BackSetTotalName = 198,
    BackUpdateCurrentInfo = 199,
    BackReportUiMessage = 200,
    BackExit = 201,
    BackSetTitleCollInfo = 202,
    BackSetTitleInfo = 203,
    BackSetTrackInfo = 204,
    BackSetChapterInfo = 205,
    BackReportUiDialog = 206,
    BackFatalCommError = 224,
    BackOutOfMem = 225,
    Unknown = 239,
}

impl TryFrom<u8> for MakeMkvCommand {
    type Error = MakeMkvError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Noop),
            1 => Ok(Self::Return),
            2 => Ok(Self::ClientDone),
            3 => Ok(Self::CallSignalExit),
            4 => Ok(Self::CallOnIdle),
            16 => Ok(Self::CallSetOutputFolder),
            17 => Ok(Self::CallUpdateAvailableDrives),
            18 => Ok(Self::CallOpenFile),
            19 => Ok(Self::CallOpenDisk),
            20 => Ok(Self::CallOpenTitleCollection),
            21 => Ok(Self::CallCloseDisk),
            22 => Ok(Self::CallEjectDisk),
            23 => Ok(Self::CallSaveAllSelectedTitlesToMkv),
            24 => Ok(Self::CallGetUiItemState),
            25 => Ok(Self::CallSetUiItemState),
            26 => Ok(Self::CallGetUiItemInfo),
            27 => Ok(Self::CallGetSettingInt),
            28 => Ok(Self::CallGetSettingString),
            29 => Ok(Self::CallSetSettingInt),
            30 => Ok(Self::CallSetSettingString),
            31 => Ok(Self::CallSaveSettings),
            32 => Ok(Self::CallAppGetString),
            33 => Ok(Self::CallBackupDisc),
            34 => Ok(Self::CallGetInterfaceLanguageData),
            35 => Ok(Self::CallSetUiItemInfo),
            36 => Ok(Self::CallSetProfile),
            37 => Ok(Self::CallInitMMBD),
            38 => Ok(Self::CallOpenMMBD),
            39 => Ok(Self::CallDiscInfoMMBD),
            40 => Ok(Self::CallDecryptUnitMMBD),
            41 => Ok(Self::CallSetExternAppFlags),
            42 => Ok(Self::CallManageState),
            43 => Ok(Self::CallAppSetString),
            192 => Ok(Self::BackEnterJobMode),
            193 => Ok(Self::BackLeaveJobMode),
            194 => Ok(Self::BackUpdateDrive),
            195 => Ok(Self::BackUpdateCurrentBar),
            196 => Ok(Self::BackUpdateTotalBar),
            197 => Ok(Self::BackUpdateLayout),
            198 => Ok(Self::BackSetTotalName),
            199 => Ok(Self::BackUpdateCurrentInfo),
            200 => Ok(Self::BackReportUiMessage),
            201 => Ok(Self::BackExit),
            202 => Ok(Self::BackSetTitleCollInfo),
            203 => Ok(Self::BackSetTitleInfo),
            204 => Ok(Self::BackSetTrackInfo),
            205 => Ok(Self::BackSetChapterInfo),
            206 => Ok(Self::BackReportUiDialog),
            224 => Ok(Self::BackFatalCommError),
            225 => Ok(Self::BackOutOfMem),
            239 => Ok(Self::Unknown),
            _ => Err(MakeMkvError::InvalidCommand(format!(
                "{} is not a valid command",
                value
            ))),
        }
    }
}

#[derive(FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct MakeMkvHeader {
    pub data_size: U16,
    pub arg_len: u8,
    pub cmd: u8,
}

impl MakeMkvHeader {
    pub fn new(data_size: u16, arg_len: u8, cmd: MakeMkvCommand) -> Self {
        Self {
            data_size: data_size.into(),
            arg_len,
            cmd: cmd as u8,
        }
    }
}

pub struct AbiResponse {
    pub _cmd: MakeMkvCommand,
    pub args: Vec<u32>,
    pub data: Vec<u8>,
}

impl AbiResponse {
    pub fn new(cmd: MakeMkvCommand, args: Vec<u32>, data: Vec<u8>) -> Self {
        Self {
            _cmd: cmd,
            args,
            data,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MakeMkvInfo {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub interface_language: String,
    pub build: String,
    pub key_type: String,
    pub key_features: String,
    pub key_expiration: String,
    pub eval_state: String,
    pub prog_expiration: String,
    pub latest_version: String,
    pub restart_required: String,
    pub expert_mode: String,
    pub profile_count: String,
    pub prog_expired: String,
    // pub output_folder_name: String,
    // pub output_base_name: String,
    pub current_profile: String,
    pub open_file_filter: String,
    pub website_url: String,
    pub open_dvd_file_filter: String,
    pub default_selection_string: String,
    pub default_output_file_name: String,
    pub external_app_item: String,
    pub profile_string: String,
    pub key_string: String,
}

#[derive(Copy, Clone, Debug)]
pub enum AppString {
    Name = 0,
    Version = 1,
    Platform = 2,
    Build = 3,
    KeyType = 4,
    KeyFeatures = 5,
    KeyExpiration = 6,
    EvalState = 7,
    ProgExpiration = 8,
    LatestVersion = 9,
    RestartRequired = 10,
    ExpertMode = 11,
    ProfileCount = 12,
    ProgExpired = 13,
    OutputFolderName = 14,
    OutputBaseName = 15,
    CurrentProfile = 16,
    OpenFileFilter = 17,
    WebsiteURL = 18,
    OpenDVDFileFilter = 19,
    DefaultSelectionString = 20,
    DefaultOutputFileName = 21,
    ExternalAppItem = 22,
    InterfaceLanguage = 23,
    ProfileString = 24,
    KeyString = 25,
}

#[derive(Copy, Clone, Debug)]
pub enum ItemAttribute {
    Unknown = 0,
    Type = 1,
    Name = 2,
    LangCode = 3,
    LangName = 4,
    CodecId = 5,
    CodecShort = 6,
    CodecLong = 7,
    ChapterCount = 8,
    Duration = 9,
    DiskSize = 10,
    DiskSizeBytes = 11,
    StreamTypeExtension = 12,
    Bitrate = 13,
    AudioChannelsCount = 14,
    AngleInfo = 15,
    SourceFileName = 16,
    AudioSampleRate = 17,
    AudioSampleSize = 18,
    VideoSize = 19,
    VideoAspectRatio = 20,
    VideoFrameRate = 21,
    StreamFlags = 22,
    DateTime = 23,
    OriginalTitleId = 24,
    SegmentsCount = 25,
    SegmentsMap = 26,
    OutputFileName = 27,
    MetadataLanguageCode = 28,
    MetadataLanguageName = 29,
    TreeInfo = 30,
    PanelTitle = 31,
    VolumeName = 32,
    OrderWeight = 33,
    OutputFormat = 34,
    OutputFormatDescription = 35,
    SeamlessInfo = 36,
    PanelText = 37,
    MkvFlags = 38,
    MkvFlagsText = 39,
    AudioChannelLayoutName = 40,
    OutputCodecShort = 41,
    OutputConversionType = 42,
    OutputAudioSampleRate = 43,
    OutputAudioSampleSize = 44,
    OutputAudioChannelsCount = 45,
    OutputAudioChannelLayoutName = 46,
    OutputAudioChannelLayout = 47,
    OutputAudioMixDescription = 48,
    Comment = 49,
    OffsetSequenceId = 50,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct MakeMkvProgress {
    pub current: f32,
    pub total: f32,
}
