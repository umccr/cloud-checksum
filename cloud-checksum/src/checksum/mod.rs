//! Checksum calculation and logic.
//!

pub mod aws_etag;
pub mod file;
pub mod standard;

use crate::checksum::aws_etag::AWSETagCtx;
use crate::checksum::standard::StandardCtx;
use crate::error::{Error, Result};
use futures_util::{pin_mut, Stream, StreamExt};
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::str::FromStr;
use std::sync::Arc;

/// The checksum calculator.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Ctx {
    Regular(StandardCtx),
    AWSEtag(AWSETagCtx),
}

impl Ctx {
    /// Update a checksum with some data.
    pub fn update(&mut self, data: Arc<[u8]>) -> Result<()> {
        match self {
            Ctx::Regular(ctx) => ctx.update(data),
            Ctx::AWSEtag(ctx) => ctx.update(data),
        }
    }

    /// Finalize the checksum.
    pub fn finalize(&mut self) -> Result<Vec<u8>> {
        match self {
            Ctx::Regular(ctx) => ctx.finalize(),
            Ctx::AWSEtag(ctx) => ctx.finalize(),
        }
    }

    /// Generate a checksum from a stream of bytes.
    pub async fn generate(
        &mut self,
        stream: impl Stream<Item = Result<Arc<[u8]>>>,
    ) -> Result<Vec<u8>> {
        pin_mut!(stream);

        while let Some(chunk) = stream.next().await {
            self.update(chunk?)?;
        }

        self.finalize()
    }

    /// Get the digest output.
    pub fn digest_to_string(&self, digest: &[u8]) -> String {
        match self {
            Ctx::Regular(ctx) => ctx.digest_to_string(digest),
            Ctx::AWSEtag(ctx) => ctx.digest_to_string(digest),
        }
    }

    /// Get the part size if this is an AWS checksum ctx.
    pub fn part_size(&self) -> Option<u64> {
        match self {
            Ctx::Regular(_) => None,
            Ctx::AWSEtag(ctx) => Some(ctx.part_size()),
        }
    }

    /// Get the encoded part checksums if this is an AWS checksum ctx.
    pub fn part_checksums(&self) -> Option<Vec<String>> {
        match self {
            Ctx::Regular(_) => None,
            Ctx::AWSEtag(ctx) => Some(ctx.part_checksums()),
        }
    }
}

impl Display for Ctx {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Ctx::Regular(ctx) => Display::fmt(ctx, f),
            Ctx::AWSEtag(ctx) => Display::fmt(ctx, f),
        }
    }
}

impl FromStr for Ctx {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let aws_etag = AWSETagCtx::from_str(s);
        if aws_etag.is_err() {
            Ok(Self::Regular(StandardCtx::from_str(s)?))
        } else {
            Ok(Self::AWSEtag(aws_etag?))
        }
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::reader::channel::test::channel_reader;
    use crate::reader::SharedReader;
    use crate::test::TestFileBuilder;
    use anyhow::Result;
    use tokio::fs::File;
    use tokio::join;

    pub(crate) async fn test_checksum(checksum: &str, expected: &str) -> Result<()> {
        let test_file = TestFileBuilder::default().generate_test_defaults()?;
        let mut reader = channel_reader(File::open(test_file).await?).await;

        let mut checksum = Ctx::from_str(checksum)?;

        let stream = reader.as_stream();
        let task = tokio::spawn(async move { reader.read_task().await });

        let (digest, _) = join!(checksum.generate(stream), task);

        assert_eq!(expected, checksum.digest_to_string(&digest?));

        Ok(())
    }
}
