mod args;
mod dispatch;

pub use dispatch::run;

#[cfg(test)]
mod help_contract_tests {
    use super::args::Cli;
    use clap::CommandFactory;

    fn assert_subcommands_have_about(command: &clap::Command) {
        for subcommand in command.get_subcommands() {
            assert!(
                subcommand.get_about().is_some(),
                "missing about for command {}",
                subcommand.get_name()
            );
            assert_subcommands_have_about(subcommand);
        }
    }

    #[test]
    fn every_cli_command_has_help_text() {
        let command = Cli::command();
        assert_subcommands_have_about(&command);
    }
}
