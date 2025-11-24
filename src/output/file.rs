use std::{
    fs,
    io::{self, Write},
};

use anyhow::Result;
use log::info;

use super::Output;
use crate::args::{Parse, Parser};

#[derive(Default, Debug)]
pub struct Args {
    path: Option<String>,
    overwrite: bool,
}

impl Parse for Args {
    fn parse(&mut self, parser: &mut Parser) -> Result<()> {
        parser.parse_opt_cfg(&mut self.path, "-r", "record")?;
        parser.parse_switch(&mut self.overwrite, "--overwrite")?;

        Ok(())
    }
}

pub struct File {
    file: fs::File,
}

impl Output for File {
    fn set_header(&mut self, header: &[u8]) -> io::Result<()> {
        self.file.write_all(header)
    }
}

impl Write for File {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        unreachable!();
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.file.write_all(buf)
    }
}

impl File {
    pub fn new(args: &Args, channel: &str) -> Result<Option<Self>> {
        let Some(path) = &args.path else {
            return Ok(None);
        };

        // Generate timestamp (e.g., "2025-11-24_15-30-45")
        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        
        // Insert channel and timestamp before file extension
        let file_path = if let Some(dot_pos) = path.rfind('.') {
            format!(
                "{}_{}_{}{}",
                &path[..dot_pos],
                channel,
                timestamp,
                &path[dot_pos..]
            )
        } else {
            format!("{}_{}_{}",path, channel, timestamp)
        };

        info!("Recording to: {}", file_path);
        if args.overwrite {
            return Ok(Some(Self {
                file: fs::File::create(&file_path)?,
            }));
        }

        Ok(Some(Self {
            file: fs::File::create_new(&file_path)?,
        }))
    }
}
