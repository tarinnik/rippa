use crate::{
    command::{
        AbiResponse, AppString, ItemAttribute, MakeMkvCommand, MakeMkvHeader, MakeMkvInfo,
        MakeMkvProgress,
    },
    drive::{DriveInfo, DriveState},
    error::MakeMkvError,
    language_data::LanguageData,
    title::{Rippable, TitleList},
    util::{u32s_to_u64, u64_to_le_u32},
};
use anyhow::{Context, anyhow, bail, ensure};
use flate2::bufread::ZlibDecoder;
use log::{debug, info, trace, warn};
use std::{
    io::{self, Read},
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, Command},
    sync::watch::Sender,
    time::{sleep, timeout},
};
use zerocopy::{FromBytes, IntoBytes};

const MAGIC_CMD_NUMBER: u8 = 0xf0;
const PROGRAM_NAME: &str = "makemkvcon";
const ABI_VERSION: &str = "A0001";
const TRANSPORT: &str = "std"; // pipe transport
const AP_APP_LOC_MAX: u32 = 7000;

#[derive(Default)]
pub struct MakeMkv {
    mmkv: Option<Child>,
    pub drive: Option<DriveInfo>,
    pub titles: Option<TitleList>,
    language_data: Option<LanguageData>,
    pub current_info: Vec<Option<String>>,
    progress: MakeMkvProgress,
    job_mode: bool,
    progress_channel: Option<Sender<MakeMkvProgress>>,
}

impl MakeMkv {
    pub fn new() -> Self {
        Self {
            current_info: vec![None; 10],
            ..Default::default()
        }
    }

    /// Spawns the makemkv process
    pub async fn init(&mut self) -> Result<MakeMkvInfo, MakeMkvError> {
        let mut mmkv = Command::new(PROGRAM_NAME)
            .arg("guiserver")
            .arg(format!("{}+{}", ABI_VERSION, TRANSPORT))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = mmkv
            .stdin
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "no stdin pipe connected"))?;

        let stdout = mmkv
            .stdout
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "no stdout pipe connected"))?;

        // Check the ABI version matches
        let mut buf = [0_u8; ABI_VERSION.len()];
        timeout(Duration::from_secs(1), stdout.read_exact(&mut buf)).await??;
        let abi_version = String::from_utf8_lossy(&buf);

        if abi_version != ABI_VERSION {
            return Err(MakeMkvError::MakeMkv(format!(
                "ABI version mismatch, received: {}, expected: {}",
                abi_version, ABI_VERSION,
            )));
        }

        let mut buf = [0_u8; 4];
        while buf[3] != 0xAA {
            timeout(Duration::from_secs(1), stdout.read_exact(&mut buf)).await??;
        }

        stdin.write_u8(0xBB).await?;
        stdin.flush().await?;

        self.mmkv = Some(mmkv);

        self.load_interface_language_data().await?;
        self.get_makemkv_info().await
    }

    async fn wait_for_disc_inserted(&mut self) -> Result<(), MakeMkvError> {
        self.update_available_drives(None).await?;
        loop {
            trace!("Waiting for disc inserted");
            if let Some(drive) = &self.drive
                && drive.drive_state == DriveState::Inserted
            {
                self.open_disk(drive.drive_id, None).await?;
                return Ok(());
            }

            sleep(Duration::from_millis(250)).await;
            self.idle().await?;
        }
    }

    pub async fn get_disc_data(&mut self) -> Result<TitleList, MakeMkvError> {
        self.wait_for_disc_inserted().await?;

        while self.titles.is_none() {
            trace!("Waiting for disc data");
            sleep(Duration::from_millis(250)).await;
            self.idle().await?;
        }

        trace!("Getting title info");
        if let Some(mut titles) = self.titles.take() {
            titles.get_data(self).await?;
            self.titles = Some(titles.clone());
            Ok(titles)
        } else {
            Err(MakeMkvError::MakeMkv("No title info".into()))
        }
    }

    pub async fn set_output_folder(&mut self, folder: &str) -> Result<(), MakeMkvError> {
        let mut data = folder.as_bytes().to_vec();
        data.push(0);
        self.transact(MakeMkvCommand::CallSetOutputFolder, None, Some(data))
            .await?;
        Ok(())
    }

    /// Enables or disables the item to be ripped
    pub async fn enable<R: Rippable>(
        &mut self,
        item: &mut R,
        enable: bool,
    ) -> Result<(), MakeMkvError> {
        let state = 0xfffffffe | enable as u32;
        self.set_item_state(item.handle(), state).await?;
        item.set_enabled(enable);
        Ok(())
    }

    pub async fn rip_all_selected(
        &mut self,
        progress: Sender<MakeMkvProgress>,
    ) -> Result<(), MakeMkvError> {
        self.progress_channel = Some(progress);
        self.transact(MakeMkvCommand::CallSaveAllSelectedTitlesToMkv, None, None)
            .await?;

        while self.job_mode {
            self.idle().await?;
            sleep(Duration::from_millis(250)).await;
        }
        Ok(())
    }

    async fn get_makemkv_info(&mut self) -> Result<MakeMkvInfo, MakeMkvError> {
        let name = self.get_app_string(AppString::Name, None, None).await?;
        let version = self.get_app_string(AppString::Version, None, None).await?;
        let platform = self.get_app_string(AppString::Platform, None, None).await?;
        let interface_language = self
            .get_app_string(AppString::InterfaceLanguage, None, None)
            .await?;
        Ok(MakeMkvInfo {
            name,
            version,
            platform,
            interface_language,
            key_type: self.get_app_string(AppString::KeyType, None, None).await?,
            key_features: self
                .get_app_string(AppString::KeyFeatures, None, None)
                .await?,
            key_expiration: self
                .get_app_string(AppString::KeyExpiration, None, None)
                .await?,
            eval_state: self
                .get_app_string(AppString::EvalState, None, None)
                .await?,
            prog_expiration: self
                .get_app_string(AppString::ProgExpiration, None, None)
                .await?,
            latest_version: self
                .get_app_string(AppString::LatestVersion, None, None)
                .await?,
            restart_required: self
                .get_app_string(AppString::RestartRequired, None, None)
                .await?,
            expert_mode: self
                .get_app_string(AppString::ExpertMode, None, None)
                .await?,
            profile_count: self
                .get_app_string(AppString::ProfileCount, None, None)
                .await?,
            prog_expired: self
                .get_app_string(AppString::ProgExpired, None, None)
                .await?,
            // output_folder_name: self
            //     .get_app_string(AppString::OutputFolderName, None, None)
            //     .await?,
            // output_base_name: self
            //     .get_app_string(AppString::OutputBaseName, None, None)
            //     .await?,
            current_profile: self
                .get_app_string(AppString::CurrentProfile, None, None)
                .await?,
            open_file_filter: self
                .get_app_string(AppString::OpenFileFilter, None, None)
                .await?,
            website_url: self
                .get_app_string(AppString::WebsiteURL, None, None)
                .await?,
            open_dvd_file_filter: self
                .get_app_string(AppString::OpenDVDFileFilter, None, None)
                .await?,
            default_selection_string: self
                .get_app_string(AppString::DefaultSelectionString, None, None)
                .await?,
            default_output_file_name: self
                .get_app_string(AppString::DefaultOutputFileName, None, None)
                .await?,
            external_app_item: self
                .get_app_string(AppString::ExternalAppItem, None, None)
                .await?,
            profile_string: self
                .get_app_string(AppString::ProfileString, None, None)
                .await?,
            key_string: self
                .get_app_string(AppString::KeyString, None, None)
                .await?,
            build: self.get_app_string(AppString::Build, None, None).await?,
        })
    }

    async fn idle(&mut self) -> Result<(), MakeMkvError> {
        self.transact(MakeMkvCommand::CallOnIdle, None, None)
            .await?;
        Ok(())
    }

    async fn update_available_drives(&mut self, flags: Option<u32>) -> Result<(), MakeMkvError> {
        let args = flags.unwrap_or(0);
        self.transact(
            MakeMkvCommand::CallUpdateAvailableDrives,
            Some(vec![args]),
            None,
        )
        .await?;
        Ok(())
    }

    async fn get_app_string(
        &mut self,
        key: AppString,
        index1: Option<u32>,
        index2: Option<u32>,
    ) -> Result<String, MakeMkvError> {
        let args = vec![key as u32, index1.unwrap_or(0), index2.unwrap_or(0)];
        let response = self
            .transact(MakeMkvCommand::CallAppGetString, Some(args), None)
            .await?;

        if !response.data.is_empty() {
            let data = String::from_utf8_lossy(&response.data[..response.data.len() - 1]);
            Ok(data.to_string())
        } else {
            Ok(String::new())
        }
    }

    async fn open_disk(&mut self, index: u32, flags: Option<u32>) -> Result<(), MakeMkvError> {
        let flags = flags.unwrap_or(0);
        self.transact(MakeMkvCommand::CallOpenDisk, Some(vec![index, flags]), None)
            .await?;
        Ok(())
    }

    async fn load_interface_language_data(&mut self) -> Result<(), MakeMkvError> {
        let result = self
            .transact(
                MakeMkvCommand::CallGetInterfaceLanguageData,
                Some(vec![AP_APP_LOC_MAX]),
                None,
            )
            .await?;

        if result.args.len() < 2 {
            return Err(MakeMkvError::InvalidResponse(
                "Load language interface returned less than two args".into(),
            ));
        }
        let unpacked_size = result.args[0];
        let packed_size = result.args[1];
        if packed_size as usize != result.data.len() {
            return Err(MakeMkvError::InvalidResponse(format!(
                "Returned data size {} does not match expected data size {}, unpacked size: {}",
                result.data.len(),
                packed_size,
                unpacked_size
            )));
        }

        let mut unpacked_data = Vec::with_capacity(unpacked_size as usize);
        let mut decompresser = ZlibDecoder::new(&result.data[..]);
        let n = decompresser.read_to_end(&mut unpacked_data)?;

        if n != unpacked_size as usize {
            return Err(MakeMkvError::InvalidResponse(
                "Decompressed data size does not match expected data size".into(),
            ));
        }

        self.language_data = Some(LanguageData::new(&unpacked_data[..n]));

        Ok(())
    }

    pub(crate) async fn is_enabled<R: Rippable>(&mut self, item: &R) -> Result<bool, MakeMkvError> {
        Ok(self.get_item_state(item.handle()).await? & 0x01 == 1)
    }

    pub(crate) async fn get_item_info(
        &mut self,
        handle: u64,
        item_attribute: ItemAttribute,
    ) -> Result<Option<String>, MakeMkvError> {
        let mut args = u64_to_le_u32(handle).to_vec();
        args.push(item_attribute as u32);

        let result = self
            .transact(MakeMkvCommand::CallGetUiItemInfo, Some(args), None)
            .await?;

        if result.args.is_empty() {
            return Err(MakeMkvError::InvalidResponse("Args is empty".into()));
        }

        if result.args[0] != 0
            && let Some(language_data) = &self.language_data
        {
            return Ok(language_data.get(result.args[0]));
        }

        if result.args.len() > 1 && result.args[1] != 0 {
            return Ok(Some(
                String::from_utf8_lossy(&result.data[..result.data.len() - 1]).to_string(),
            ));
        }

        Err(MakeMkvError::InvalidResponse(
            "Invalid get item info response, no data".into(),
        ))
    }

    async fn get_item_state(&mut self, handle: u64) -> Result<u32, MakeMkvError> {
        let args = u64_to_le_u32(handle).to_vec();
        let response = self
            .transact(MakeMkvCommand::CallGetUiItemState, Some(args), None)
            .await?;

        if !response.args.is_empty() {
            Ok(response.args[0])
        } else {
            Err(MakeMkvError::MakeMkv("No arg received in response".into()))
        }
    }

    pub(crate) async fn set_item_state(
        &mut self,
        handle: u64,
        state: u32,
    ) -> Result<(), MakeMkvError> {
        let mut args = u64_to_le_u32(handle).to_vec();
        args.push(state);
        self.transact(MakeMkvCommand::CallSetUiItemState, Some(args), None)
            .await?;
        Ok(())
    }

    async fn transact(
        &mut self,
        cmd: MakeMkvCommand,
        args: Option<Vec<u32>>,
        data: Option<Vec<u8>>,
    ) -> anyhow::Result<AbiResponse> {
        self.send_command(cmd, args, data).await?;
        self.receive_response().await
    }

    /// Send a command to MakeMKV
    async fn send_command(
        &mut self,
        cmd: MakeMkvCommand,
        args: Option<Vec<u32>>,
        data: Option<Vec<u8>>,
    ) -> anyhow::Result<()> {
        ensure!(self.mmkv.is_some(), "MakeMKV not initialised");
        let mmkv = self.mmkv.as_mut().unwrap();
        let stdin = mmkv
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("MakeMKV stdin is None"))?;

        let mut buf = Vec::new();

        let mut data = data.unwrap_or_default();
        let mut args: Vec<u8> = args
            .unwrap_or_default()
            .into_iter()
            .flat_map(|x| x.to_le_bytes().to_vec())
            .collect();

        let mut header = MakeMkvHeader::new(data.len() as u16, (args.len() / 4) as u8, cmd)
            .as_bytes()
            .to_vec();

        buf.append(&mut header);
        buf.append(&mut args);
        buf.append(&mut data);

        trace!("Sending message: {:?}", &buf);
        stdin
            .write_all(&buf)
            .await
            .context("Unable to write to makemkv")?;

        Ok(())
    }

    /// Receive a message from MakeMKV
    async fn receive_response(&mut self) -> anyhow::Result<AbiResponse> {
        ensure!(self.mmkv.is_some(), "MakeMKV not initialised");

        loop {
            let mmkv = self.mmkv.as_mut().unwrap();
            let stdout = mmkv.stdout.as_mut().context("MakeMKV stdout is None")?;

            trace!("Waiting for header data");
            let mut buf = [0_u8; 4];
            let n = stdout.read(&mut buf).await?;

            let cmd: MakeMkvCommand;
            let data_size;
            let arg_len;
            if n == 4 {
                let header = MakeMkvHeader::read_from_bytes(&buf[..])
                    .map_err(|e| anyhow!("Unable to deserialise makemkv header: {}", e))?;
                data_size = header.data_size.get();
                arg_len = header.arg_len;
                cmd = header.cmd.try_into()?;
            } else if n == 1 {
                let cmd_num = buf[0];
                ensure!(
                    cmd_num >= MAGIC_CMD_NUMBER,
                    "Received invalid command: {}",
                    cmd_num
                );
                cmd = (cmd_num - MAGIC_CMD_NUMBER).try_into()?;
                data_size = 0;
                arg_len = 0;
            // } else if n == 0 {
            //     warn!("Header size of 0 received");
            //     continue;
            } else {
                bail!("{} is not a valid header length", n);
            }

            trace!(
                "Received cmd: {:?}, arg_len: {}, data_size: {}",
                cmd, arg_len, data_size
            );

            trace!("Waiting for arg data");
            let mut args: Vec<u32> = Vec::with_capacity(arg_len as usize);
            for _ in 0..arg_len {
                let mut buf = [0_u8; 4];
                timeout(Duration::from_secs(1), stdout.read_exact(&mut buf))
                    .await
                    .context("Timeout waiting for message")?
                    .context("Unable to read arg bytes from mmkv")?;
                args.push(u32::from_le_bytes(buf));
            }
            trace!("Got args: {:?}", &args);

            let mut data_buf = vec![0_u8; data_size as usize];
            let n = timeout(Duration::from_secs(1), stdout.read_exact(&mut data_buf))
                .await
                .context("Timeout waiting for data")?
                .context("Unable to read data bytes from mmkv")?;
            let data = &data_buf[..n];
            trace!("Got data: {:?}", &data);

            match cmd {
                MakeMkvCommand::Noop => {}
                MakeMkvCommand::BackUpdateDrive => {
                    match DriveInfo::try_from_update(&args, data) {
                        Ok(drive) => {
                            trace!("Drive info: {:?}", &drive);
                            self.drive = Some(drive);
                        }
                        Err(e) => warn!("Unable to parse DriveInfo: {}", e),
                    };
                }
                MakeMkvCommand::BackSetTitleCollInfo => {
                    if args.len() != 3 {
                        continue;
                    }
                    let handle = u32s_to_u64(args[1], args[0]);
                    let size = args[2];
                    self.titles = Some(TitleList::new(handle, size));
                    trace!("Setting titles");
                }
                MakeMkvCommand::BackSetTitleInfo => {
                    if args.len() != 7 || self.titles.is_none() {
                        continue;
                    }
                    let handle = u32s_to_u64(args[2], args[1]);
                    let chapter_handle = u32s_to_u64(args[6], args[5]);
                    if let Some(titles) = &mut self.titles {
                        titles.add_title(args[0], handle, chapter_handle, args[4], args[3]);
                    }
                }
                MakeMkvCommand::BackSetTrackInfo => {
                    if args.len() < 4 {
                        continue;
                    }
                    let handle = u32s_to_u64(args[3], args[2]);
                    if let Some(title_list) = &mut self.titles {
                        title_list.add_track(args[0], args[1], handle);
                    }
                }
                MakeMkvCommand::BackSetChapterInfo => {
                    if args.len() != 4 {
                        continue;
                    }
                    let handle = u32s_to_u64(args[3], args[2]);
                    if let Some(title_list) = &mut self.titles {
                        title_list.add_chapter(args[0], args[1], handle);
                    }
                }
                MakeMkvCommand::BackUpdateCurrentInfo if !args.is_empty() && args[0] < 10 => {
                    self.current_info[args[0] as usize] =
                        Some(String::from_utf8_lossy(data).to_string());
                }
                MakeMkvCommand::BackUpdateCurrentInfo => {}
                MakeMkvCommand::BackEnterJobMode => self.job_mode = true,
                MakeMkvCommand::BackLeaveJobMode => self.job_mode = false,
                MakeMkvCommand::BackUpdateCurrentBar if !args.is_empty() => {
                    let current_progress = args[0];
                    debug!("Current progress = {}", current_progress);
                    self.progress.current = current_progress as f32 / u16::MAX as f32 * 100.0;
                    if let Some(channel) = &self.progress_channel {
                        debug!("Sending: {}%", self.progress.current);
                        let _ = channel.send(self.progress);
                    }
                }
                MakeMkvCommand::BackUpdateCurrentBar => {}
                MakeMkvCommand::BackUpdateTotalBar if !args.is_empty() => {
                    let total_progress = args[0];
                    debug!("Total progress = {}", total_progress);
                    self.progress.total = total_progress as f32 / u16::MAX as f32 * 100.0;
                    if let Some(channel) = &self.progress_channel {
                        debug!("Sending: {}%", self.progress.total);
                        let _ = channel.send(self.progress);
                    }
                }
                MakeMkvCommand::BackUpdateTotalBar => {}
                MakeMkvCommand::Return => {
                    return Ok(AbiResponse::new(cmd, args, data.to_vec()));
                }
                _ => {}
            }

            self.send_command(MakeMkvCommand::ClientDone, None, None)
                .await?;
        }
    }
}
