//! File-based sums file logic.
//!

use crate::checksum::file::SumsFile;
use crate::error::Result;
use crate::io::copy::{CopyContent, MultiPartOptions, ObjectCopy};
use crate::io::Provider;
use std::io::SeekFrom;
use tokio::fs::copy;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt};
use tokio::{fs, io};

/// Build a file based sums object.
#[derive(Debug, Default)]
pub struct FileBuilder;

impl FileBuilder {
    /// Build using the file name.
    pub fn build(self) -> File {
        File
    }
}

/// A file object.
#[derive(Debug, Default)]
pub struct File;

impl File {
    /// Copy the file to the destination.
    pub async fn copy(&self, source: String, destination: String) -> Result<u64> {
        Ok(copy(&source, destination).await?)
    }

    /// Read the source into memory.
    pub async fn read(
        &self,
        source: String,
        multi_part_options: Option<MultiPartOptions>,
    ) -> Result<CopyContent> {
        let mut file = fs::File::open(source).await?;
        let size = file.metadata().await?.len();

        // Read only the specified range if multipart is being used.
        let file: Box<dyn AsyncRead + Send + Sync + Unpin> =
            if let Some(multipart) = multi_part_options {
                file.seek(SeekFrom::Start(multipart.start)).await?;
                Box::new(file.take(multipart.end - multipart.start))
            } else {
                Box::new(file)
            };

        Ok(CopyContent::new(file, Some(size), None, None))
    }

    /// Write the data to the destination.
    pub async fn write(&self, destination: String, mut data: CopyContent) -> Result<Option<u64>> {
        // Append to an existing file or create a new one.
        let mut file = if fs::try_exists(&destination)
            .await
            .is_ok_and(|exists| exists)
        {
            fs::OpenOptions::new()
                .append(true)
                .write(true)
                .open(destination)
                .await?
        } else {
            fs::File::create(destination).await?
        };

        let total = io::copy(&mut data.data, &mut file).await?;

        Ok(Some(total))
    }
}

#[async_trait::async_trait]
impl ObjectCopy for File {
    async fn copy_object(
        &mut self,
        provider_source: Provider,
        provider_destination: Provider,
        _multipart: Option<MultiPartOptions>,
    ) -> Result<Option<u64>> {
        let source = SumsFile::format_target_file(&provider_source.into_file()?);
        let destination = SumsFile::format_target_file(&provider_destination.into_file()?);

        // There's no point copying using multiple parts on the filesystem so just ignore the option
        Ok(Some(self.copy(source, destination).await?))
    }

    async fn download(
        &mut self,
        source: Provider,
        multipart: Option<MultiPartOptions>,
    ) -> Result<CopyContent> {
        let source = source.into_file()?;
        let source = SumsFile::format_target_file(&source);

        self.read(source, multipart).await
    }

    async fn upload(
        &mut self,
        destination: Provider,
        data: CopyContent,
        _multipart: Option<MultiPartOptions>,
    ) -> Result<Option<u64>> {
        let destination = destination.into_file()?;
        let destination = SumsFile::format_target_file(&destination);

        // It doesn't matter what the part number is for filesystem operations, just append to the
        // end of the file.
        self.write(destination, data).await
    }
}
