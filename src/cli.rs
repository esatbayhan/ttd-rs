use std::path::PathBuf;

/// Manual parser to avoid heavyweight CLI frameworks.
/// Uses only `std::env::args()` — no dependencies.

#[derive(Debug)]
pub struct Cli {
    pub task_dir: Option<PathBuf>,
    pub command: Option<Command>,
}

#[derive(Debug)]
pub enum Command {
    Add { line: String },
    List,
    Done { id: String },
    Search { query: String },
}

impl Cli {
    pub fn parse() -> Result<Self, String> {
        let mut args = std::env::args().skip(1).peekable();
        let mut task_dir = None;
        let mut command = None;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--task-dir" => {
                    task_dir = Some(PathBuf::from(
                        args.next()
                            .ok_or_else(|| "--task-dir requires a value".to_string())?,
                    ));
                }
                "add" => {
                    let rest: Vec<String> = (&mut args).collect();
                    if rest.is_empty() {
                        return Err("ttd add: missing task text".to_string());
                    }
                    command = Some(Command::Add {
                        line: rest.join(" "),
                    });
                }
                "list" => command = Some(Command::List),
                "done" => {
                    let id = args
                        .next()
                        .ok_or_else(|| "ttd done: missing task identifier".to_string())?;
                    command = Some(Command::Done { id });
                }
                "search" => {
                    let query = args
                        .next()
                        .ok_or_else(|| "ttd search: missing search query".to_string())?;
                    command = Some(Command::Search { query });
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                "--version" | "-V" => {
                    println!("ttd {}", env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }
                other => return Err(format!("unknown option or command: {other}")),
            }
        }

        Ok(Cli { task_dir, command })
    }
}

fn print_help() {
    print!(
        r#"ttd — A plain-text task manager for the todo.txt.d format

Usage:
  ttd [--task-dir <path>] [command]

Commands:
  add <text>           Add a new task, e.g. `ttd add due:2026-04-02 learning for exam`
  list                 List all open tasks
  done <id>            Mark a task as done
  search <query>       Search tasks by keyword

Options:
  --task-dir <path>    Directory containing the task files
  --help, -h           Print this help message
  --version, -V        Print version information

When run without a command, ttd launches the interactive TUI.
"#
    );
}
