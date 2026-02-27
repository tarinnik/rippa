use std::time::Duration;

use crate::{
    error::RippaError,
    makemkv::{
        command::{AbiResponse, MakeMkvCommand, MakeMkvHeader},
        drive::DriveInfo,
        title::TitleList,
        util::u32s_to_u64,
    },
};
use anyhow::{Context, anyhow, bail, ensure};
use log::debug;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Child,
    time::timeout,
};
use zerocopy::{FromBytes, IntoBytes};

const MAGIC_CMD_NUMBER: u8 = 0xf0;

pub struct MakeMkv {
    mmkv: Option<Child>,
    drive: Option<DriveInfo>,
    titles: Option<TitleList>,
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
            current_info: vec![None; 10],
            current_bar: 0,
            total_bar: 0,
            job_mode: false,
        }
    }

    pub fn init(&mut self) -> Result<(), RippaError> {
        Ok(())
    }

    async fn transact(
        &mut self,
        cmd: MakeMkvCommand,
        args: Option<Vec<u8>>,
        data: Option<Vec<u8>>,
    ) -> anyhow::Result<AbiResponse> {
        self.send_command(cmd, args, data).await?;
        self.receive_response().await
    }

    /// Send a command to MakeMKV
    async fn send_command(
        &mut self,
        cmd: MakeMkvCommand,
        args: Option<Vec<u8>>,
        data: Option<Vec<u8>>,
    ) -> anyhow::Result<()> {
        ensure!(self.mmkv.is_some(), "MakeMKV not initialised");
        let mmkv = self.mmkv.as_mut().unwrap();
        let stdin = mmkv
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("MakeMKV stdin is None"))?;

        let mut buf = Vec::new();

        let mut data = data.unwrap_or_else(|| Vec::new());
        let mut args = args.unwrap_or_else(|| Vec::new());

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
        let mmkv = self.mmkv.as_mut().unwrap();
        let stdout = mmkv.stdout.as_mut().context("MakeMKV stdout is None")?;

        loop {
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
                    let drive = DriveInfo::try_from_update(&args, data);
                    if let Some(d) = drive {
                        self.drive = Some(d);
                    }
                }
                MakeMkvCommand::BackSetTitleCollInfo => {
                    if args.len() != 3 {
                        continue;
                    }
                    let handle = u32s_to_u64(args[1], args[0]);
                    let size = args[2];
                    self.titles = Some(TitleList::new(handle, size));
                }
                MakeMkvCommand::BackSetTitleInfo => {
                    if args.len() != 7 || self.titles.is_none() {
                        continue;
                    }
                    let handle = u32s_to_u64(args[2], args[1]);
                    let chapter_handle = u32s_to_u64(args[6], args[5]);
                    self.titles.as_mut().unwrap().add_title(
                        args[0],
                        handle,
                        chapter_handle,
                        args[4],
                        args[3],
                    );
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
                    if args.len() > 0 && args[0] < 10 {
                        self.current_info[args[0] as usize] =
                            Some(String::from_utf8_lossy(&data).to_string());
                    }
                }
                MakeMkvCommand::BackEnterJobMode => self.job_mode = true,
                MakeMkvCommand::BackLeaveJobMode => self.job_mode = false,
                MakeMkvCommand::BackUpdateCurrentBar => {
                    if args.len() > 0 {
                        self.current_bar = args[0];
                    }
                }
                MakeMkvCommand::BackUpdateTotalBar => {
                    if args.len() > 0 {
                        self.total_bar = args[0];
                    }
                }
                MakeMkvCommand::Return => {
                    return Ok(AbiResponse::new(cmd, args, data.to_vec()));
                }
                _ => {}
            }
        }
    }
}
