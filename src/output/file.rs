use std::{
    fs,
    io::{self, ErrorKind, Write},
    path::{Path, PathBuf},
};

use anyhow::Result;
use log::{debug, info};

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
    base_path: PathBuf,
    channel: String,
    overwrite: bool,
    header: Option<Vec<u8>>,
    current: Option<fs::File>,
    segment_index: u64,
}

impl Output for File {
    fn set_header(&mut self, header: &[u8]) -> io::Result<()> {
        self.header = Some(header.to_vec());
        Ok(())
    }
}

impl Write for File {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        unreachable!();
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(file) = self.current.as_mut() {
            file.flush()?;
        }

        self.current = None;
        Ok(())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.ensure_file()?;
        self.current
            .as_mut()
            .expect("File handle missing after ensure_file")
            .write_all(buf)
    }
}

impl File {
    pub fn new(args: &Args, channel: &str) -> Result<Option<Self>> {
        let Some(path) = &args.path else {
            return Ok(None);
        };

        info!("Recording segments to: {path}");

        Ok(Some(Self {
            base_path: PathBuf::from(path),
            channel: channel.to_owned(),
            overwrite: args.overwrite,
            header: None,
            current: None,
            segment_index: 0,
        }))
    }

    fn ensure_file(&mut self) -> io::Result<()> {
        if self.current.is_some() {
            return Ok(());
        }

        self.current = Some(self.create_segment_file()?);
        Ok(())
    }

    fn create_segment_file(&mut self) -> io::Result<fs::File> {
        let timestamp = Self::timestamp();
        let mut attempt = 0;

        loop {
            let index = self.segment_index + attempt;
            let path = self.segment_path(&timestamp, index);

            let result = if self.overwrite {
                fs::File::create(&path)
            } else {
                fs::File::create_new(&path)
            };

            match result {
                Ok(mut file) => {
                    if let Some(header) = &self.header {
                        file.write_all(header)?;
                    }

                    if self.segment_index == 0 && attempt == 0 {
                        info!("Recording to: {}", path.display());
                    } else {
                        debug!("Recording to: {}", path.display());
                    }

                    self.segment_index = index.saturating_add(1);
                    return Ok(file);
                }
                Err(error) if !self.overwrite && error.kind() == ErrorKind::AlreadyExists => {
                    attempt = attempt.saturating_add(1);
                    continue;
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn segment_path(&self, timestamp: &str, index: u64) -> PathBuf {
        let (stem, ext) = Self::split_stem_ext(&self.base_path);
        let mut filename = format!("{stem}_{}_{}_{index:05}", self.channel, timestamp);
        filename.push('.');
        filename.push_str(&ext);

        if let Some(parent) = self
            .base_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
        {
            parent.join(filename)
        } else {
            PathBuf::from(filename)
        }
    }

    fn split_stem_ext(path: &Path) -> (String, String) {
        let stem = path
            .file_stem()
            .or_else(|| path.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "recording".to_owned());

        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().into_owned())
            .filter(|e| !e.is_empty())
            .unwrap_or_else(|| "ts".to_owned());

        (stem, ext)
    }

    fn timestamp() -> String {
        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string()
    }
}
