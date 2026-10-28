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

    #[command(about = "Monitor .sayn files for changes and automatically regenerate")]
    Watch(WatchArgs),

    #[command(about = "Repair generated application errors")]
    Fix(FixArgs),

    #[command(about = "Create a restorable checkpoint of the generated application")]
    Snapshot(SnapshotArgs),

    #[command(about = "Restore a previous generated application checkpoint")]
    Rollback(RollbackArgs),

    #[command(about = "Export a clean, deployable copy of generated source")]
    Export(ExportArgs),

    #[command(about = "Remove temporary build files and caches safely")]
    Clean(CleanArgs),

    #[command(about = "Inspect Sayn capability permissions")]
    Permissions(PermissionsArgs),

    #[command(about = "Show authenticated user and active provider details")]
    Whoami(WhoamiArgs),

    #[command(about = "Clear active authentication and provider credentials")]
    Logout(LogoutArgs),

    #[command(about = "Manage AI model providers")]
    Provider(ProviderArgs),
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

#[derive(Args, Debug, PartialEq)]
pub struct WatchArgs {
    #[arg(long, help = "Require approval before regenerating on change")]
    pub manual: bool,

    #[arg(long, help = "Automatically regenerate on change within limits")]
    pub auto: bool,

    #[arg(long, help = "Maximum budget in dollars for the watch session")]
    pub budget: Option<f64>,
}

#[derive(Args, Debug, PartialEq)]
pub struct FixArgs {
    #[arg(long, help = "Simulate repair without modifying files")]
    pub dry_run: bool,

    #[arg(long, help = "Focus specifically on dependency resolution")]
    pub dependency: bool,

    #[arg(long, help = "Focus specifically on TypeScript compilation errors")]
    pub types: bool,

    #[arg(long, help = "Focus on production build failure repair")]
    pub build: bool,
}

#[derive(Args, Debug, PartialEq)]
pub struct SnapshotArgs {
    #[arg(help = "Optional snapshot ID (auto-generated if omitted)")]
    pub id: Option<String>,
}

#[derive(Args, Debug, PartialEq)]
pub struct RollbackArgs {
    #[arg(help = "Snapshot ID to restore")]
    pub id: String,
}

#[derive(Args, Debug, PartialEq)]
pub struct ExportArgs {
    #[arg(short, long, help = "Target export directory (defaults to ./dist)")]
    pub output: Option<PathBuf>,
}

#[derive(Args, Debug, PartialEq)]
pub struct CleanArgs {
    #[arg(long, help = "Clean all disposable caches and logs")]
    pub all: bool,
}

#[derive(Args, Debug, PartialEq)]
pub struct PermissionsArgs {
    #[arg(help = "Action: list, reset, revoke (defaults to list)")]
    pub action: Option<String>,
}

#[derive(Args, Debug, PartialEq)]
pub struct WhoamiArgs {}

#[derive(Args, Debug, PartialEq)]
pub struct LogoutArgs {
    #[arg(long, help = "Specific provider to log out of (sayn, nous, custom)")]
    pub provider: Option<String>,

    #[arg(long, help = "Clear credentials for all providers")]
    pub all_providers: bool,
}

#[derive(Args, Debug, PartialEq)]
pub struct ProviderArgs {
    #[command(subcommand)]
    pub action: Option<ProviderSubcommand>,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum ProviderSubcommand {
    List,
    Add {
        #[arg(help = "Provider name (e.g., openai, anthropic, gemini)")]
        name: Option<String>,
    },
    Use {
        #[arg(help = "Provider name to set as active")]
        name: String,
    },
    Test {
        #[arg(help = "Provider name to test (defaults to active provider)")]
        name: Option<String>,
    },
    Remove {
        #[arg(help = "Provider name to remove")]
        name: String,
    },
}
