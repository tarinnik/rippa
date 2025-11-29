use crate::{
    error::RippaError,
    makemkv::command::{MakeMkvCommand, MakeMkvHeader},
};
use anyhow::{Context, anyhow, bail, ensure};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Child,
};
use zerocopy::FromBytes;

const MAGIC_CMD_NUMBER: u8 = 0xf0;

pub struct MakeMkv {
    mmkv: Option<Child>,
}

impl MakeMkv {
    pub fn new() -> Self {
        Self { mmkv: None }
    }

    pub fn init(&mut self) -> Result<(), RippaError> {
        Ok(())
    }

    async fn transact(&mut self, cmd: MakeMkvCommand) -> anyhow::Result<()> {
        ensure!(self.mmkv.is_some(), "MakeMKV not initialised");

        self.send_command(cmd).await?;

        Ok(())
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
            cmd = cmd_num.try_into()?;
        } else {
            bail!("{} is not a valid header length", n);
        }

        Err(anyhow!("not implemented"))
    }
}
