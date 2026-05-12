//! Wicket CLI.
//!
//! `wicket check`  reads a full Intent JSON (raw mode).
//! `wicket edit`   verb-shaped preflight for filesystem mutation.
//! `wicket run`    verb-shaped preflight for command execution.
//! `wicket commit` verb-shaped preflight for git commits.
//!
//! Exit codes follow SPEC.md §8.2.

use clap::{Parser, Subcommand, ValueEnum};
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;
use wicket::cook::{auto_detect_policy_paths, CommitPacket, EditPacket, RunPacket};
use wicket::grant::{self, Grant, GrantValidation};
use wicket::{
    Intent, OperationClass, Outcome, PrecedenceResolution, StandingClass, SurfaceVerdict,
};

#[derive(Parser)]
#[command(
    name = "wicket",
    about = "Admissibility preflight for agentic operations",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read a full Intent JSON from --input or stdin.
    Check {
        #[arg(long, short = 'i')]
        input: Option<PathBuf>,
        #[arg(long)]
        strict_exit: bool,
        #[arg(long)]
        canonical: bool,
        #[arg(long)]
        brief: bool,
    },
    /// Preflight a filesystem edit. Reads target file (if it exists) and
    /// auto-detects local policy docs.
    Edit {
        /// Path to file being edited.
        target: PathBuf,
        /// Concrete sentence describing the rule the actor cites.
        #[arg(long)]
        because: String,
        #[arg(long, value_enum, default_value_t = StandingArg::Recommend)]
        standing: StandingArg,
        /// Path to a standing grant (JSON). When supplied and validated, the
        /// grant's standing class supersedes --standing and the grant is
        /// attached as `policy_ref` evidence.
        #[arg(long)]
        standing_grant: Option<PathBuf>,
        /// Caller's precedence resolution. Default `active` (no superseding rule
        /// known). Use `unresolved` for genuine uncertainty.
        #[arg(long, value_enum, default_value_t = PrecedenceArg::Active)]
        precedence: PrecedenceArg,
        /// Token identifying a fresh human confirmation. Required for bind ops.
        #[arg(long)]
        human_confirm: Option<String>,
        /// Additional policy doc paths beyond auto-detected ones.
        #[arg(long = "policy")]
        policy_paths: Vec<PathBuf>,
        /// Override actor identifier.
        #[arg(long, default_value = "claude-code")]
        actor: String,
        #[arg(long)]
        strict_exit: bool,
        #[arg(long)]
        brief: bool,
        /// Print the cooked Intent JSON instead of running `check`.
        #[arg(long)]
        show_intent: bool,
    },
    /// Preflight a command-execution intent. The command is described, not run.
    Run {
        /// Command line being proposed (for the verdict; not executed by Wicket).
        command: String,
        #[arg(long)]
        because: String,
        #[arg(long, value_enum, default_value_t = ClassArg::Execute)]
        class: ClassArg,
        #[arg(long, value_enum, default_value_t = StandingArg::Recommend)]
        standing: StandingArg,
        #[arg(long)]
        standing_grant: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = PrecedenceArg::Active)]
        precedence: PrecedenceArg,
        #[arg(long)]
        human_confirm: Option<String>,
        #[arg(long = "policy")]
        policy_paths: Vec<PathBuf>,
        #[arg(long, default_value = "claude-code")]
        actor: String,
        #[arg(long)]
        strict_exit: bool,
        #[arg(long)]
        brief: bool,
        #[arg(long)]
        show_intent: bool,
    },
    /// Preflight a git commit on the current branch.
    Commit {
        #[arg(long)]
        because: String,
        /// Treat the commit as bind-class (irreversible). Default is execute.
        #[arg(long)]
        irreversible: bool,
        #[arg(long, value_enum, default_value_t = StandingArg::Recommend)]
        standing: StandingArg,
        #[arg(long)]
        standing_grant: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = PrecedenceArg::Active)]
        precedence: PrecedenceArg,
        #[arg(long)]
        human_confirm: Option<String>,
        #[arg(long = "policy")]
        policy_paths: Vec<PathBuf>,
        #[arg(long, default_value = "claude-code")]
        actor: String,
        #[arg(long)]
        strict_exit: bool,
        #[arg(long)]
        brief: bool,
        #[arg(long)]
        show_intent: bool,
    },
}

#[derive(Copy, Clone, ValueEnum)]
enum PrecedenceArg {
    Active,
    Superseded,
    Ambiguous,
    Unresolved,
}

impl From<PrecedenceArg> for PrecedenceResolution {
    fn from(p: PrecedenceArg) -> Self {
        match p {
            PrecedenceArg::Active => PrecedenceResolution::Active,
            PrecedenceArg::Superseded => PrecedenceResolution::Superseded,
            PrecedenceArg::Ambiguous => PrecedenceResolution::Ambiguous,
            PrecedenceArg::Unresolved => PrecedenceResolution::Unresolved,
        }
    }
}

#[derive(Copy, Clone, ValueEnum)]
enum StandingArg {
    Observe,
    Interpret,
    Recommend,
    Authorize,
    Execute,
}

impl From<StandingArg> for StandingClass {
    fn from(s: StandingArg) -> Self {
        match s {
            StandingArg::Observe => StandingClass::Observe,
            StandingArg::Interpret => StandingClass::Interpret,
            StandingArg::Recommend => StandingClass::Recommend,
            StandingArg::Authorize => StandingClass::Authorize,
            StandingArg::Execute => StandingClass::Execute,
        }
    }
}

#[derive(Copy, Clone, ValueEnum)]
enum ClassArg {
    Observe,
    Interpret,
    Recommend,
    Authorize,
    Execute,
    Bind,
}

impl From<ClassArg> for OperationClass {
    fn from(c: ClassArg) -> Self {
        match c {
            ClassArg::Observe => OperationClass::Observe,
            ClassArg::Interpret => OperationClass::Interpret,
            ClassArg::Recommend => OperationClass::Recommend,
            ClassArg::Authorize => OperationClass::Authorize,
            ClassArg::Execute => OperationClass::Execute,
            ClassArg::Bind => OperationClass::Bind,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check { input, strict_exit, canonical, brief } => {
            run_check(input, strict_exit, canonical, brief)
        }
        Command::Edit {
            target, because, standing, standing_grant, precedence, human_confirm,
            mut policy_paths, actor, strict_exit, brief, show_intent,
        } => {
            let cwd = std::env::current_dir().unwrap_or_default();
            for p in auto_detect_policy_paths(&cwd) {
                if !policy_paths.contains(&p) {
                    policy_paths.push(p);
                }
            }
            let (effective_standing, grant_evidence) = match resolve_grant(
                standing_grant.as_deref(),
                &actor,
                "edit",
                &target,
                standing.into(),
            ) {
                Ok(v) => v,
                Err(code) => return code,
            };
            let intent = wicket::cook::cook_edit(EditPacket {
                actor,
                standing: effective_standing,
                because,
                target,
                cwd,
                policy_paths,
                human_confirmation: human_confirm,
                precedence: precedence.into(),
                grant_evidence,
            });
            if show_intent {
                emit_intent(&intent);
                ExitCode::SUCCESS
            } else {
                run_intent(&intent, strict_exit, false, brief, "edit")
            }
        }
        Command::Run {
            command, because, class, standing, standing_grant, precedence, human_confirm,
            mut policy_paths, actor, strict_exit, brief, show_intent,
        } => {
            let cwd = std::env::current_dir().unwrap_or_default();
            for p in auto_detect_policy_paths(&cwd) {
                if !policy_paths.contains(&p) {
                    policy_paths.push(p);
                }
            }
            let (effective_standing, grant_evidence) = match resolve_grant(
                standing_grant.as_deref(),
                &actor,
                "run",
                &cwd,
                standing.into(),
            ) {
                Ok(v) => v,
                Err(code) => return code,
            };
            let intent = wicket::cook::cook_run(RunPacket {
                actor,
                standing: effective_standing,
                because,
                command,
                operation_class: class.into(),
                cwd,
                policy_paths,
                human_confirmation: human_confirm,
                precedence: precedence.into(),
                grant_evidence,
            });
            if show_intent {
                emit_intent(&intent);
                ExitCode::SUCCESS
            } else {
                run_intent(&intent, strict_exit, false, brief, "run")
            }
        }
        Command::Commit {
            because, irreversible, standing, standing_grant, precedence, human_confirm,
            mut policy_paths, actor, strict_exit, brief, show_intent,
        } => {
            let cwd = std::env::current_dir().unwrap_or_default();
            for p in auto_detect_policy_paths(&cwd) {
                if !policy_paths.contains(&p) {
                    policy_paths.push(p);
                }
            }
            let class = if irreversible {
                OperationClass::Bind
            } else {
                OperationClass::Execute
            };
            let (effective_standing, grant_evidence) = match resolve_grant(
                standing_grant.as_deref(),
                &actor,
                "commit",
                &cwd,
                standing.into(),
            ) {
                Ok(v) => v,
                Err(code) => return code,
            };
            let intent = wicket::cook::cook_commit(CommitPacket {
                actor,
                standing: effective_standing,
                because,
                operation_class: class,
                cwd,
                policy_paths,
                human_confirmation: human_confirm,
                precedence: precedence.into(),
                grant_evidence,
            });
            if show_intent {
                emit_intent(&intent);
                ExitCode::SUCCESS
            } else {
                run_intent(&intent, strict_exit, false, brief, "commit")
            }
        }
    }
}

fn run_check(input: Option<PathBuf>, strict_exit: bool, canonical: bool, brief: bool) -> ExitCode {
    let raw = match read_input(input.as_deref()) {
        Ok(s) => s,
        Err(e) => return emit_input_error(&format!("read failed: {e}")),
    };

    let intent: Intent = match serde_json::from_str(&raw) {
        Ok(i) => i,
        Err(e) => return emit_input_error(&format!("schema invalid: {e}")),
    };

    run_intent(&intent, strict_exit, canonical, brief, "check")
}

fn run_intent(
    intent: &Intent,
    strict_exit: bool,
    canonical: bool,
    brief: bool,
    verb: &str,
) -> ExitCode {
    let outcome = wicket::check(intent);

    if brief {
        print_brief(&outcome, verb, &intent.target);
    } else {
        print_full(&outcome, canonical);
    }

    exit_for(outcome.surface_verdict, strict_exit)
}

/// Validate a `--standing-grant` if supplied. Returns the effective standing
/// (elevated when the grant applies) and an optional `policy_ref` Evidence
/// pointing at the grant. On failure, prints the validation reason to stderr
/// and returns an exit code (65 = data error).
fn resolve_grant(
    grant_path: Option<&std::path::Path>,
    actor: &str,
    op: &str,
    target: &std::path::Path,
    fallback_standing: StandingClass,
) -> Result<(StandingClass, Option<wicket::Evidence>), ExitCode> {
    let Some(path) = grant_path else {
        return Ok((fallback_standing, None));
    };

    let grant = match Grant::load(path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("wicket: failed to load standing grant: {e}");
            return Err(ExitCode::from(65));
        }
    };
    let now = chrono::Utc::now();
    match grant::validate(&grant, actor, op, target, now) {
        GrantValidation::Valid => {
            let ev = grant::grant_evidence(&grant, path, actor, now);
            Ok((grant.standing, Some(ev)))
        }
        other => {
            eprintln!("wicket: standing grant rejected: {other}");
            Err(ExitCode::from(65))
        }
    }
}

fn emit_intent(intent: &Intent) {
    if let Ok(s) = serde_json::to_string_pretty(intent) {
        println!("{s}");
    }
}

fn read_input(path: Option<&std::path::Path>) -> std::io::Result<String> {
    match path {
        Some(p) => std::fs::read_to_string(p),
        None => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s)?;
            Ok(s)
        }
    }
}

fn print_full(outcome: &Outcome, canonical: bool) {
    let serialized = if canonical {
        serde_jcs::to_string(outcome).ok()
    } else {
        serde_json::to_string_pretty(outcome).ok()
    };
    if let Some(s) = serialized {
        println!("{s}");
    }
}

fn print_brief(outcome: &Outcome, verb: &str, target: &str) {
    let verdict = match outcome.surface_verdict {
        SurfaceVerdict::Authorized => "AUTHORIZED",
        SurfaceVerdict::AdvisoryOnly => "ADVISORY_ONLY",
        SurfaceVerdict::Denied => "DENIED",
        SurfaceVerdict::Gap => "GAP",
        SurfaceVerdict::Unaccounted => "UNACCOUNTED",
    };

    // Top reason codes: prefer the first non-OK / non-CALLER_ASSERTED_UNVERIFIED
    // codes from each dimension, falling back to the dimension OK if nothing else.
    let mut top: Vec<&str> = Vec::new();
    for c in &outcome.reason_codes {
        if c.ends_with("_CALLER_ASSERTED_UNVERIFIED") || c.ends_with("_LADDER_V1_FLAT") {
            continue;
        }
        if !top.contains(&c.as_str()) {
            top.push(c);
        }
        if top.len() >= 4 {
            break;
        }
    }
    if top.is_empty() {
        top.push("(no salient codes)");
    }

    println!("{verdict}  {verb} {target}");
    println!("  reasons: {}", top.join(", "));
    if !outcome.allowed.is_empty() {
        println!("  allowed: {}", outcome.allowed.join(", "));
    }
    if !outcome.forbidden.is_empty() {
        println!("  forbidden: {}", outcome.forbidden.join(", "));
    }
    println!("  receipt: {}", outcome.receipt.receipt_id);
}

fn emit_input_error(detail: &str) -> ExitCode {
    let body = serde_json::json!({
        "class": "error",
        "surface_verdict": "unaccounted",
        "reason_codes": ["INPUT_SCHEMA_INVALID"],
        "error_detail": detail,
    });
    if let Ok(s) = serde_json::to_string_pretty(&body) {
        println!("{s}");
    }
    ExitCode::from(64)
}

fn exit_for(v: SurfaceVerdict, strict: bool) -> ExitCode {
    match (v, strict) {
        (SurfaceVerdict::Unaccounted, _) => ExitCode::from(2),
        (SurfaceVerdict::Denied, true) | (SurfaceVerdict::Gap, true) => ExitCode::from(2),
        _ => ExitCode::SUCCESS,
    }
}
