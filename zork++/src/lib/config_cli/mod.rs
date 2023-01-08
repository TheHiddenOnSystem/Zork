use clap::{Parser, Subcommand, ValueEnum};

/// [`CliArgs`] This is parser arguments to command line argument
///
/// #Test
/// ```rust
/// use clap::Parser;
/// use zork::config_cli::{CliArgs, Command, CppCompiler};
///
/// let parser = CliArgs::parse_from(["", "-vv"]);
/// assert_eq!(2, parser.verbose);
///
/// let parser = CliArgs::parse_from(["", "tests"]);
/// assert_eq!(parser.command, Some(Command::Tests));
///
// Create Template Project
/// let parser = CliArgs::parse_from(["", "-n", "--git", "--compiler", "clang"]);
/// assert_eq!(parser.new_template, true);
/// assert_eq!(parser.git, true);
/// assert_eq!(parser.compiler, Some(CppCompiler::CLANG));
///
///
/// ```
#[derive(Parser, Debug)]
#[command(name = "Zork++")]
#[command(author = "Zero Day Code")]
#[command(version = "0.5.0")]
#[command(
    about = "Zork++ is a build system for modern C++ projects",
    long_about = "Zork++ is a project of Zero Day Code. Find us: https://github.com/zerodaycode/Zork"
)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(short, long, action = clap::ArgAction::Count, help="Zork++ maximum allowed verbosity level is: '-vv'")]
    pub verbose: u8,

    #[arg(short, long, group = "base_option", help = "Create Project template")]
    pub new_template: bool,
    #[arg(
        long,
        help = "Initializes a new local git repo, can you use with template project"
    )]
    pub git: bool,
    #[arg(
        long,
        help = "Indicates what compiler wants the user to use with the autogenerated project mode, can you use with template project"
    )]
    pub compiler: Option<CppCompiler>,
}

/// [`Command`] -  The core enum commands
#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Command {
    /// Executes the tests under the specified directory in the config file
    Tests,
}

/// [`CppCompiler`] The C++ compilers available within Zork++ as a command line argument for the `new` argument
/// TODO Possible future interesting on support the Intel's C++ compiler?
#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum CppCompiler {
    CLANG,
    MSVC,
    GCC,
}

impl CppCompiler {
    pub fn get_default_extesion(&self) -> &str {
        match *self {
            CppCompiler::CLANG => "cppm",
            CppCompiler::MSVC => "ixx",
            CppCompiler::GCC => todo!("GCC is still not supported yet by Zork++"),
        }
    }
}
