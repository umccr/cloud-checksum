use clap::Parser;
use cloud_checksum::error::Result;
use cloud_checksum::reader::channel::ChannelReader;
use cloud_checksum::task::generate::GenerateTask;
use cloud_checksum::{Commands, Subcommands};
use tokio::fs::File;
use tokio::io::stdin;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Commands::parse();

    match args.commands {
        Subcommands::Generate { input, .. } => match input {
            None => {
                let reader = ChannelReader::new_with_capacity(
                    stdin(),
                    args.optimization.channel_reader_capacity,
                );

                GenerateTask::default()
                    .add_generate_tasks(args.checksums, &reader)
                    .add_reader_task(reader)?
                    .run()
                    .await?;
            }
            Some(input) => {
                let reader = ChannelReader::new_with_capacity(
                    File::open(input).await?,
                    args.optimization.channel_reader_capacity,
                );

                GenerateTask::default()
                    .add_generate_tasks(args.checksums, &reader)
                    .add_reader_task(reader)?
                    .run()
                    .await?;
            }
        },
        Subcommands::Check { .. } => todo!(),
    };

    Ok(())
}
