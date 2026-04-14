use std::{env, ffi::OsString, path::PathBuf, sync::Arc};

use color_eyre::{
    Result,
    eyre::{bail, eyre},
};
use slynx::SlynxContext;

#[derive(Debug, Default)]
struct CliArgs {
    target: Option<PathBuf>,
    hir: bool,
    ir: bool,
}

impl CliArgs {
    fn parse() -> Result<Self> {
        let mut parsed = Self::default();
        let mut args = env::args_os();
        let program_name = args.next().unwrap_or_else(|| OsString::from("slynx"));

        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--hir" => parsed.hir = true,
                "--ir" => parsed.ir = true,
                "--target" | "-t" => {
                    let value = args.next().ok_or_else(|| {
                        eyre!("expected a file path after {}", arg.to_string_lossy())
                    })?;
                    parsed.set_target(PathBuf::from(value))?;
                }
                "--help" | "-h" => {
                    Self::print_help(&program_name);
                    std::process::exit(0);
                }
                flag if flag.starts_with('-') => {
                    bail!("unrecognized flag: {flag}");
                }
                _ => {
                    // Accept a positional file path so the CLI stays easy to use in addition
                    // to supporting the explicit --target form requested by the issue.
                    parsed.set_target(PathBuf::from(arg))?;
                }
            }
        }

        if parsed.target.is_none() {
            Self::print_help(&program_name);
            bail!("missing target file. Use --target <path> or pass the path positionally");
        }

        Ok(parsed)
    }

    fn set_target(&mut self, target: PathBuf) -> Result<()> {
        if self.target.is_some() {
            bail!("target file was provided more than once");
        }
        self.target = Some(target);
        Ok(())
    }

    fn print_help(program_name: &OsString) {
        eprintln!(
            "Usage: {} [--target <path> | <path>] [--hir] [--ir]\n\n\
             Options:\n  \
             -t, --target <path>  Input .slynx file to compile\n  \
             --hir                Write the textual HIR dump next to the input as .hir\n  \
             --ir                 Write the textual IR dump next to the input as .ir\n  \
             -h, --help           Show this help message",
            program_name.to_string_lossy()
        );
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = CliArgs::parse()?;
    let entry_point = Arc::new(cli.target.expect("target should be set during parsing"));

    // Build the compiler stages once so optional dump flags do not rerun the pipeline.
    let context = SlynxContext::new(entry_point)?;
    let stages = context.build_stages()?;

    if cli.hir {
        stages.write_hir()?;
    }

    if cli.ir {
        stages.write_ir()?;
    }

    // Keep the CLI aligned with the existing library behavior by always writing the .sir output.
    let output = stages.into_output();
    output.write()?;

    Ok(())
}
