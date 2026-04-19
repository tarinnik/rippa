use crate::{
    command::{AbiResponse, AppString, ItemAttribute, MakeMkvCommand, MakeMkvHeader},
    drive::{DriveInfo, DriveState},
    error::MakeMkvError,
    language_data::LanguageData,
    title::TitleList,
    util::{u32s_to_u64, u64_to_le_u32},
};
use anyhow::{Context, anyhow, bail, ensure};
use flate2::bufread::ZlibDecoder;
use log::debug;
use std::{
    io::{self, Read},
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, Command},
    time::{sleep, timeout},
};
use zerocopy::{FromBytes, IntoBytes};

const MAGIC_CMD_NUMBER: u8 = 0xf0;
const PROGRAM_NAME: &str = "makemkvcon";
const ABI_VERSION: &str = "A0001";
const TRANSPORT: &str = "std"; // pipe transport
const AP_APP_LOC_MAX: u32 = 7000;

pub struct MakeMkv {
    mmkv: Option<Child>,
    drive: Option<DriveInfo>,
    titles: Option<TitleList>,
    language_data: Option<LanguageData>,
    current_info: Vec<Option<String>>,
    current_bar: u32,
    total_bar: u32,
    job_mode: bool,
}

impl MakeMkv {
    pub fn new() -> Self {
        Self {
            mmkv: None,
            drive: None,
            titles: None,
            language_data: None,
            current_info: vec![None; 10],
            current_bar: 0,
            total_bar: 0,
            job_mode: false,
        }
    }

    /// Spawns the makemkv process
    pub async fn init(&mut self) -> Result<(), MakeMkvError> {
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
        self.update_available_drives(None).await?;

        Ok(())
    }

    pub async fn wait_for_disc_inserted(&mut self) -> Result<(), MakeMkvError> {
        loop {
            let drive = self.drive.as_ref().ok_or(MakeMkvError::DriveNotDetected)?;
            if drive.drive_state == DriveState::Inserted {
                self.open_disk(drive.drive_id, None).await?;
                return Ok(());
            }

            self.idle().await?;
            sleep(Duration::from_millis(250)).await;
        }
    }

    pub async fn get_disc_data(&mut self) -> Result<(), MakeMkvError> {
        while self.titles.is_none() {
            self.idle().await?;
            sleep(Duration::from_millis(250)).await;
        }

        if let Some(mut titles) = self.titles.take() {
            titles.get_data(self).await?;
            self.titles = Some(titles);
        }

        Ok(())
    }

    pub async fn set_output_folder(&mut self, folder: &str) -> Result<(), MakeMkvError> {
        let mut data = folder.as_bytes().to_vec();
        data.push(0);
        self.transact(MakeMkvCommand::CallSetOutputFolder, None, Some(data))
            .await?;
        Ok(())
    }

    pub async fn rip_all_selected(&mut self) -> Result<(), MakeMkvError> {
        self.transact(MakeMkvCommand::CallSaveAllSelectedTitlesToMkv, None, None)
            .await?;
        Ok(())
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

        let data = String::from_utf8_lossy(&response.data);

        Ok(data.to_string())
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
            return Err(MakeMkvError::InvalidResponse(
                "Returned data size does not match expected data size".into(),
            ));
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

    pub(crate) async fn get_ui_item_info(
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

    pub(crate) async fn get_item_state(&mut self, handle: u64) -> Result<u32, MakeMkvError> {
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

    async fn set_item_state(&mut self, handle: u64, state: u32) -> Result<(), MakeMkvError> {
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

        debug!("Sending message: {:?}", &buf);
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
            } else {
                bail!("{} is not a valid header length", n);
            }

            debug!("Received cmd: {:?}", cmd);

            let mut args: Vec<u32> = Vec::with_capacity(arg_len as usize);
            for _ in 0..arg_len {
                let mut buf = [0_u8; 4];
                timeout(Duration::from_secs(1), stdout.read_exact(&mut buf))
                    .await
                    .context("Timeout waiting for message")?
                    .context("Unable to read arg bytes from mmkv")?;
                args.push(u32::from_le_bytes(buf));
            }

            let mut data_buf = Vec::with_capacity(data_size as usize);
            let n = timeout(Duration::from_secs(1), stdout.read(&mut data_buf))
                .await
                .context("Timeout waiting for data")?
                .context("Unable to read data bytes from mmkv")?;
            let data = &data_buf[..n];

            match cmd {
                MakeMkvCommand::Noop => {}
                MakeMkvCommand::BackUpdateDrive => {
                    let drive = DriveInfo::try_from_update(&args, data)?;
                    debug!("Drive info: {:?}", &drive);
                    self.drive = Some(drive);
                }
                MakeMkvCommand::BackSetTitleCollInfo => {
                    if args.len() != 3 {
                        continue;
                    }
                    let handle = u32s_to_u64(args[1], args[0]);
                    let size = args[2];
                    self.titles = Some(TitleList::new(handle, size));
                    debug!("Setting titles");
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
                    if args.len() != 4 {
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
                MakeMkvCommand::BackUpdateCurrentInfo => {
                    // TODO: Handle more cases
                    if !args.is_empty() && args[0] < 10 {
                        self.current_info[args[0] as usize] =
                            Some(String::from_utf8_lossy(data).to_string());
                    }
                }
                MakeMkvCommand::BackEnterJobMode => self.job_mode = true,
                MakeMkvCommand::BackLeaveJobMode => self.job_mode = false,
                MakeMkvCommand::BackUpdateCurrentBar => {
                    if !args.is_empty() {
                        self.current_bar = args[0];
                    }
                }
                MakeMkvCommand::BackUpdateTotalBar => {
                    if !args.is_empty() {
                        self.total_bar = args[0];
                    }
                }
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
