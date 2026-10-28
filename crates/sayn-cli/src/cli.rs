use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "sayn",
    author = "Sayn Contributors",
    version,
    about = "An AI-native, instruction-first programming language and compiler toolchain",
    long_about = "Write what your software should do in .sayn files. Run 'sayn craft' to interpret instructions, generate or update a real application, validate the result, and run it locally."
)]
pub struct Cli {
    #[arg(
        short,
        long,
        global = true,
        help = "Path to the project root directory"
    )]
    pub project: Option<PathBuf>,

    #[arg(short, long, global = true, help = "Enable verbose diagnostic output")]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    #[command(about = "Scaffold a new Sayn project")]
    Create(CreateArgs),

    #[command(about = "Initialize Sayn in an existing project")]
    Init(InitArgs),

    #[command(about = "Validate .sayn instructions and configuration offline")]
    Check(CheckArgs),

    #[command(about = "Preview what Sayn intends to generate")]
    Plan(PlanArgs),

    #[command(about = "Interpret instructions, generate code, validate, and start dev mode")]
    Craft(CraftArgs),

    #[command(about = "Build and validate deployable production output")]
    Build(BuildArgs),

    #[command(about = "Format .sayn instruction files deterministically")]
    Format(FormatArgs),

    #[command(about = "Show project status, instruction files, and generated output state")]
    Status(StatusArgs),

    #[command(about = "Compare generated changes with instruction expectations")]
    Diff(DiffArgs),

    #[command(about = "Diagnose the local development environment")]
    Doctor(DoctorArgs),

    #[command(about = "Manage Sayn account authentication and BYOK credentials")]
    Auth(AuthArgs),

    #[command(about = "Inspect and select available AI model providers")]
    Model(ModelArgs),

    #[command(about = "View or modify Sayn configuration")]
    Config(ConfigArgs),
}

#[derive(Args, Debug, PartialEq)]
pub struct CreateArgs {
    #[arg(long, help = "Scaffold a web project (Next.js)")]
    pub web: bool,

    #[arg(long, help = "Scaffold an API service (reserved)")]
    pub api: bool,

    #[arg(long, help = "Scaffold a game prototype (reserved)")]
    pub game: bool,

    #[arg(help = "Project directory name (or '.' for current directory)")]
    pub name: Option<String>,
}

#[derive(Args, Debug, PartialEq)]
pub struct InitArgs {
    #[arg(help = "Directory to initialize (defaults to current directory)")]
    pub directory: Option<PathBuf>,
}

#[derive(Args, Debug, PartialEq)]
pub struct CheckArgs {
    #[arg(long, help = "Treat warnings as errors")]
    pub strict: bool,
}

#[derive(Args, Debug, PartialEq)]
pub struct PlanArgs {
    #[arg(long, help = "Output plan in JSON format")]
    pub json: bool,

    #[arg(
        long,
        help = "Perform offline static analysis without AI model requests"
    )]
    pub offline: bool,
}

#[derive(Args, Debug, PartialEq)]
pub struct CraftArgs {
    #[arg(help = "Optional craft action, such as 'resume'")]
    pub action: Option<String>,

    #[arg(long, help = "Generate and validate without starting the dev server")]
    pub no_run: bool,

    #[arg(long, help = "Preview generation changes without modifying files")]
    pub dry_run: bool,

    #[arg(long, help = "Override selected AI model")]
    pub model: Option<String>,
}

#[derive(Args, Debug, PartialEq)]
pub struct BuildArgs {
    #[arg(long, help = "Produce an optimized release build")]
    pub release: bool,

    #[arg(long, help = "Target output directory")]
    pub out_dir: Option<PathBuf>,
}

#[derive(Args, Debug, PartialEq)]
pub struct FormatArgs {
    #[arg(help = "Specific .sayn file or directory to format (defaults to project)")]
    pub path: Option<PathBuf>,

    #[arg(long, help = "Check if files are formatted without modifying them")]
    pub check: bool,

    #[arg(long, help = "Display proposed formatting diffs")]
    pub diff: bool,
}

#[derive(Args, Debug, PartialEq)]
pub struct StatusArgs {}

#[derive(Args, Debug, PartialEq)]
pub struct DiffArgs {}

#[derive(Args, Debug, PartialEq)]
pub struct DoctorArgs {}

#[derive(Args, Debug, PartialEq)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub action: Option<AuthSubcommand>,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum AuthSubcommand {
    Login,
    Logout,
    Status,
    Token,
}

#[derive(Args, Debug, PartialEq)]
pub struct ModelArgs {
    #[command(subcommand)]
    pub action: Option<ModelSubcommand>,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum ModelSubcommand {
    List,
    Select { name: String },
}

#[derive(Args, Debug, PartialEq)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: Option<ConfigSubcommand>,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum ConfigSubcommand {
    Get { key: String },
    Set { key: String, value: String },
    List,
}
