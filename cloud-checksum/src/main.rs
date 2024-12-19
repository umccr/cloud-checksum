use clap::Parser;
use cloud_checksum::error::Result;
use cloud_checksum::reader::channel::ChannelReader;
use cloud_checksum::task::generate::{file_size, GenerateTaskBuilder};
use cloud_checksum::{Commands, Subcommands};
use std::collections::HashSet;
use tokio::fs::File;
use tokio::io::stdin;
use cloud_checksum::task::check::CheckTaskBuilder;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Commands::parse();

    match args.commands {
        Subcommands::Generate { input, .. } => {
            if input == "-" {
                let mut reader = ChannelReader::new(stdin(), args.optimization.channel_capacity);

                let output = GenerateTaskBuilder::default()
                    .with_overwrite(args.force_overwrite)
                    .with_verify(args.verify)
                    .build()
                    .await?
                    .add_generate_tasks(HashSet::from_iter(args.checksum), &mut reader, None)?
                    .add_reader_task(reader)?
                    .run()
                    .await?
                    .to_json_string()?;

                println!("{}", output);
            } else {
                let file = File::open(&input).await?;
                let file_size = file_size(&file).await;
                let mut reader = ChannelReader::new(file, args.optimization.channel_capacity);

                GenerateTaskBuilder::default()
                    .with_overwrite(args.force_overwrite)
                    .with_verify(args.verify)
                    .with_input_file_name(input)
                    .build()
                    .await?
                    .add_generate_tasks(HashSet::from_iter(args.checksum), &mut reader, file_size)?
                    .add_reader_task(reader)?
                    .run()
                    .await?
                    .write()
                    .await?
            }
        }
        Subcommands::Check { input } => {
            let is_same = CheckTaskBuilder::default().with_input_files(input).build().await?.run().await?;

            if is_same {
                println!("All files match");
            } else {
                println!("Files do not match");
            }
        },
    };

    Ok(())
}
