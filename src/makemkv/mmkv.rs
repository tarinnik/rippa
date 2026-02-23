use std::time::Duration;

use crate::{
    error::RippaError,
    makemkv::{
        command::{MakeMkvCommand, MakeMkvHeader},
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
use zerocopy::FromBytes;

const MAGIC_CMD_NUMBER: u8 = 0xf0;

pub struct MakeMkv {
    mmkv: Option<Child>,
    drive: Option<DriveInfo>,
    titles: Option<TitleList>,
}

impl MakeMkv {
    pub fn new() -> Self {
        Self {
            mmkv: None,
            drive: None,
            titles: None,
        }
    }

    pub fn init(&mut self) -> Result<(), RippaError> {
        Ok(())
    }

    async fn transact(&mut self, cmd: MakeMkvCommand) -> anyhow::Result<MakeMkvCommand> {
        self.send_command(cmd).await?;
        self.receive_response().await
    }

    /// Send a command to MakeMKV
    async fn send_command(&mut self, cmd: MakeMkvCommand) -> anyhow::Result<()> {
        ensure!(self.mmkv.is_some(), "MakeMKV not initialised");
        let mmkv = self.mmkv.as_mut().unwrap();
        let stdin = mmkv
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("MakeMKV stdin is None"))?;

        Ok(())
    }

    /// Receive a message from MakeMKV
    async fn receive_response(&mut self) -> anyhow::Result<MakeMkvCommand> {
        ensure!(self.mmkv.is_some(), "MakeMKV not initialised");
        let mmkv = self.mmkv.as_mut().unwrap();
        let stdout = mmkv
            .stdout
            .as_mut()
            .ok_or_else(|| anyhow!("MakeMKV stdout is None"))?;

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
                _ => {}
            }
        }
    }
}
