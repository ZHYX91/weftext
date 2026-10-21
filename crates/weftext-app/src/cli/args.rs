use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[command(name = "weftext-cli", version, disable_help_subcommand = false)]
pub struct Args {
    #[arg(long, global = true, conflicts_with = "endpoint")]
    pub data_dir: Option<PathBuf>,
    #[arg(long, global = true, conflicts_with = "data_dir")]
    pub endpoint: Option<String>,
    #[arg(long, global = true, requires = "endpoint")]
    pub credential_env: Option<String>,
    #[arg(long, global = true, requires = "endpoint")]
    pub allow_loopback_http: bool,
    #[arg(long, global = true, default_value = "zh-CN", value_parser = ["zh-CN", "en-US"])]
    pub locale: String,
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Json)]
    pub format: OutputFormat,
    #[arg(long, global = true)]
    pub output: Option<PathBuf>,
    #[arg(long, global = true)]
    pub allow_partial: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Json,
    Human,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Serve(crate::launch::ServeArgs),
    OwnerStatus,
    OwnerStop {
        #[arg(long)]
        owner_identity: PathBuf,
    },
    Capabilities,
    Library {
        #[command(subcommand)]
        command: LibraryCommand,
    },
    Note {
        #[command(subcommand)]
        command: NoteCommand,
    },
    Query {
        #[command(subcommand)]
        command: QueryCommand,
    },
    Operation {
        #[command(subcommand)]
        command: OperationCommand,
    },
    History {
        #[command(subcommand)]
        command: HistoryCommand,
    },
    Export {
        #[command(subcommand)]
        command: ExportCommand,
    },
    Import {
        #[command(subcommand)]
        command: ImportCommand,
    },
    Sync {
        #[command(subcommand)]
        command: SyncCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum LibraryCommand {
    List,
    Inspect {
        #[arg(long)]
        path: PathBuf,
    },
    Create {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        options: Option<PathBuf>,
        #[command(flatten)]
        record: Record,
    },
}

#[derive(Debug, Clone, ClapArgs)]
#[group(required = true, multiple = false)]
pub struct Document {
    #[arg(long)]
    pub path: Option<PathBuf>,
    #[arg(long)]
    pub resource: Option<String>,
}

#[derive(Debug, Clone, ClapArgs)]
pub struct Record {
    #[arg(long)]
    pub request_file: PathBuf,
}

#[derive(Debug, Clone, Subcommand)]
pub enum NoteCommand {
    Read {
        #[command(flatten)]
        document: Document,
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long)]
        source_output: Option<PathBuf>,
    },
    Create {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        parent: PathBuf,
        #[arg(long)]
        name: String,
        #[command(flatten)]
        record: Record,
    },
    Write {
        #[arg(long)]
        basis: PathBuf,
        #[arg(long)]
        source: PathBuf,
        #[command(flatten)]
        record: Record,
    },
    SaveAs {
        #[arg(long)]
        basis: PathBuf,
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        path: PathBuf,
        #[command(flatten)]
        record: Record,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum QueryCommand {
    Catalog {
        #[arg(long)]
        root: Option<PathBuf>,
    },
    Run {
        #[arg(long)]
        input: PathBuf,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum OperationCommand {
    Inspect {
        #[arg(long)]
        request: PathBuf,
    },
    Settle {
        #[arg(long)]
        request: PathBuf,
        #[command(flatten)]
        record: Record,
    },
    Prepare {
        #[arg(long)]
        input: PathBuf,
        #[command(flatten)]
        record: Record,
    },
    Commit {
        #[arg(long)]
        input: PathBuf,
        #[command(flatten)]
        record: Record,
    },
    Recoveries,
    Recovery {
        #[arg(long)]
        operation_id: String,
    },
    Resolve {
        #[arg(long)]
        operation_id: String,
        #[arg(long)]
        action: String,
        #[command(flatten)]
        record: Record,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum HistoryCommand {
    List {
        #[arg(long)]
        basis: PathBuf,
    },
    Read {
        #[arg(long)]
        basis: PathBuf,
        #[arg(long)]
        version: String,
    },
    Compare {
        #[arg(long)]
        basis: PathBuf,
        #[arg(long)]
        version: String,
    },
    Restore {
        #[arg(long)]
        input: PathBuf,
        #[command(flatten)]
        record: Record,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ExportCommand {
    Prepare {
        #[arg(long, required_unless_present = "input")]
        basis: Option<PathBuf>,
        #[arg(long,conflicts_with_all=["source","kind"])]
        input: Option<PathBuf>,
        #[arg(long, requires = "basis")]
        source: Option<PathBuf>,
        #[arg(long, required_unless_present = "input", requires = "basis")]
        kind: Option<String>,
        #[arg(long, default_value = "none")]
        review_mode: String,
        #[arg(long)]
        include_resolved: bool,
        #[arg(long, default_value = "source")]
        style_policy: String,
        #[command(flatten)]
        record: Record,
    },
    Status {
        #[arg(long)]
        request: PathBuf,
    },
    Confirm {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        digest: String,
        #[arg(long)]
        accept_losses: bool,
        #[command(flatten)]
        record: Record,
    },
    Download {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        destination: PathBuf,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ImportCommand {
    Inspect {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        kind: String,
        #[command(flatten)]
        destination: ImportDestination,
    },
    Prepare {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        kind: String,
        #[command(flatten)]
        destination: ImportDestination,
        #[command(flatten)]
        record: Record,
    },
    Status {
        #[arg(long)]
        request: PathBuf,
    },
    Commit {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        plan_id: String,
        #[arg(long)]
        accept_losses: bool,
        #[command(flatten)]
        record: Record,
    },
    Settle {
        #[arg(long)]
        request: PathBuf,
        #[command(flatten)]
        record: Record,
    },
}

#[derive(Debug, Clone, ClapArgs)]
pub struct ImportDestination {
    #[arg(long, conflicts_with_all=["resource","slot"])]
    pub basis: Option<PathBuf>,
    #[arg(long, requires = "basis")]
    pub parent: Option<PathBuf>,
    #[arg(long, requires = "basis")]
    pub name: Option<String>,
    #[arg(long, conflicts_with = "slot")]
    pub resource: Option<String>,
    #[arg(long, conflicts_with = "resource")]
    pub slot: Option<String>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum SyncCommand {
    Check {
        #[command(flatten)]
        connection: SyncConnection,
    },
    Push {
        #[command(flatten)]
        connection: SyncConnection,
        #[arg(long)]
        root: PathBuf,
        #[arg(long,default_value_t=900,value_parser=clap::value_parser!(u64).range(0..=900))]
        wait_seconds: u64,
        #[command(flatten)]
        record: Record,
    },
    Pull {
        #[command(flatten)]
        connection: SyncConnection,
        #[arg(long)]
        objects: Option<PathBuf>,
        #[arg(long,default_value_t=900,value_parser=clap::value_parser!(u64).range(0..=900))]
        wait_seconds: u64,
        #[command(flatten)]
        record: Record,
    },
    Job {
        #[arg(long)]
        request: PathBuf,
    },
    Stop {
        #[arg(long)]
        request: PathBuf,
        #[command(flatten)]
        record: Record,
    },
    Binding {
        #[arg(long)]
        root: PathBuf,
    },
    Enable {
        #[arg(long)]
        root: PathBuf,
        #[command(flatten)]
        record: Record,
    },
    Status {
        #[arg(long)]
        input: PathBuf,
    },
    Publish {
        #[arg(long)]
        root: PathBuf,
        #[command(flatten)]
        record: Record,
    },
    Receive {
        #[arg(long)]
        input: PathBuf,
        #[command(flatten)]
        record: Record,
    },
    Export {
        #[arg(long)]
        input: PathBuf,
        #[command(flatten)]
        record: Record,
    },
    History {
        #[arg(long)]
        input: PathBuf,
    },
    Operation {
        #[arg(long)]
        operation_id: String,
    },
    Cancel {
        #[arg(long)]
        input: PathBuf,
        #[command(flatten)]
        record: Record,
    },
    AdoptionCut {
        #[arg(long)]
        input: PathBuf,
    },
    RevalidationCut {
        #[arg(long)]
        input: PathBuf,
    },
    RevalidationReview {
        #[arg(long)]
        input: PathBuf,
    },
}
#[derive(Debug, Clone, ClapArgs)]
pub struct SyncConnection {
    #[arg(long)]
    pub server: String,
    #[arg(long)]
    pub server_credential_env: String,
    #[arg(long)]
    pub allow_server_loopback_http: bool,
    #[arg(long)]
    pub identity: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn effects_require_a_recoverable_known_input_file() {
        assert!(
            Args::try_parse_from([
                "weftext-cli",
                "note",
                "write",
                "--basis",
                "b.json",
                "--source",
                "-"
            ])
            .is_err()
        );
        let args = Args::try_parse_from([
            "weftext-cli",
            "note",
            "write",
            "--basis",
            "b.json",
            "--source",
            "-",
            "--request-file",
            "r.weftext-request.json",
        ])
        .unwrap();
        assert!(matches!(
            args.command,
            Command::Note {
                command: NoteCommand::Write { .. }
            }
        ));
    }
    #[test]
    fn local_and_remote_are_explicit_and_secret_is_not_an_argument() {
        assert!(
            Args::try_parse_from([
                "weftext-cli",
                "--endpoint",
                "https://notes.test",
                "--data-dir",
                "state",
                "capabilities"
            ])
            .is_err()
        );
        assert!(
            Args::try_parse_from(["weftext-cli", "--token", "secret", "capabilities"]).is_err()
        );
        assert!(
            Args::try_parse_from([
                "weftext-cli",
                "note",
                "read",
                "--path",
                "a.typ",
                "--resource",
                "a"
            ])
            .is_err()
        );
    }
}
