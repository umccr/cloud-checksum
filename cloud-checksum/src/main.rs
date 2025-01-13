use clap::Parser;
use cloud_checksum::checksum::file::SumsFile;
use cloud_checksum::checksum::Ctx;
use cloud_checksum::error::Result;
use cloud_checksum::reader::channel::ChannelReader;
use cloud_checksum::task::check::{CheckTaskBuilder, GroupBy};
use cloud_checksum::task::generate::{file_size, GenerateTaskBuilder, SumCtxPairs};
use cloud_checksum::{Commands, Subcommands};
use std::collections::HashSet;
use tokio::fs::File;
use tokio::io::stdin;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Commands::parse();

    match args.commands {
        Subcommands::Generate {
            input,
            generate_missing,
            force_overwrite,
            verify,
        } => {
            if input[0] == "-" {
                let mut reader = ChannelReader::new(stdin(), args.optimization.channel_capacity);

                let output = GenerateTaskBuilder::default()
                    .with_overwrite(force_overwrite)
                    .with_verify(verify)
                    .build()
                    .await?
                    .add_generate_tasks(HashSet::from_iter(args.checksum), &mut reader, None)?
                    .add_reader_task(reader)?
                    .run()
                    .await?
                    .to_json_string()?;

                println!("{}", output);
            } else {
                if generate_missing {
                    let ctxs = CheckTaskBuilder::default()
                        .with_input_files(input.clone())
                        .with_group_by(GroupBy::Comparability)
                        .build()
                        .await?
                        .run()
                        .await?;

                    let ctxs = SumCtxPairs::from_comparable(ctxs)?;
                    if let Some(ctxs) = ctxs {
                        for ctx in ctxs.into_inner() {
                            let (input, ctx) = ctx.into_inner();
                            generate(
                                args.optimization.channel_capacity,
                                force_overwrite,
                                verify,
                                input,
                                HashSet::from_iter(vec![ctx]),
                            )
                            .await?
                        }
                    }
                };

                for input in input {
                    let ctx = HashSet::from_iter(args.checksum.clone());
                    generate(
                        args.optimization.channel_capacity,
                        force_overwrite,
                        verify,
                        input,
                        ctx,
                    )
                    .await?;
                }
            }
        }
        Subcommands::Check {
            input,
            update,
            group_by,
        } => {
            let files = check(input, group_by).await?;

            for file in files {
                println!("The following groups of files are the same:");
                println!("{:#?}", file.names());

                if update {
                    file.write().await?;
                }
            }
        }
    };

    Ok(())
}

async fn check(input: Vec<String>, group_by: GroupBy) -> Result<Vec<SumsFile>> {
    CheckTaskBuilder::default()
        .with_input_files(input)
        .with_group_by(group_by)
        .build()
        .await?
        .run()
        .await
}

async fn generate(
    capacity: usize,
    force_overwrite: bool,
    verify: bool,
    input: String,
    ctx: HashSet<Ctx>,
) -> Result<()> {
    let file = File::open(&input).await?;
    let file_size = file_size(&file).await;
    let mut reader = ChannelReader::new(file, capacity);

    GenerateTaskBuilder::default()
        .with_overwrite(force_overwrite)
        .with_verify(verify)
        .with_input_file_name(input)
        .build()
        .await?
        .add_generate_tasks(ctx, &mut reader, file_size)?
        .add_reader_task(reader)?
        .run()
        .await?
        .write()
        .await
}
