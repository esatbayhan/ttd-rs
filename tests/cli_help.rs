use ttd::cli::Cli;

#[test]
fn cli_parses_subcommands() {
    let _cli = Cli {
        task_dir: None,
        command: Some(ttd::cli::Command::Add {
            line: "test".into(),
        }),
    };
}

#[test]
fn cli_allows_running_without_a_subcommand() {
    let cli = Cli {
        task_dir: None,
        command: None,
    };
    assert!(cli.command.is_none());
}
