//! The crate responsable for executing the core work of `Zork++`,
// generate command lines and execute them in a shell of the current
// operating system against the designed compilers in the configuration
// file.
mod commands;

use color_eyre::{eyre::Context, Result};
use std::{collections::HashMap, path::Path};

use crate::{
    cli::CliArgs,
    config_file::{compiler::CppCompiler, modules::ModuleInterface, ZorkConfigFile},
    utils::{self, constants::DEFAULT_OUTPUT_DIR, reader::find_config_file},
};

use self::commands::execute_command;

/// The entry point of the compilation process
///
/// Whenever this process gets triggered, the files declared within the
/// configuration file will be build.
///
/// TODO Decision path for building the executable command line,
/// the tests executable command line, a static lib, a dylib...
pub fn build_project(base_path: &Path, _cli_args: &CliArgs) -> Result<()> {
    let config_file: String =
        find_config_file(base_path).with_context(|| "Failed to read configuration file")?;
    let config: ZorkConfigFile = toml::from_str(config_file.as_str())
        .with_context(|| "Could not parse configuration file")?;

    // Create the directory for dump the generated files
    create_output_directory(base_path, &config)?;

    // 1st - Build the modules
    let _modules_commands = build_modules(&config);

    Ok(())
}

/// Triggers the build process for compile the declared modules in the project
///
/// This function acts like a operation result processor, by running instances
/// and parsing the obtained result, handling the flux according to the
/// compiler responses>
fn build_modules(config: &ZorkConfigFile) -> Result<HashMap<String, Vec<String>>> {
    let compiler = &config.compiler.cpp_compiler;
    let mut commands: HashMap<String, Vec<String>> = HashMap::new();

    // TODO Dev todo's!
    // Change the string types for strong types (ie, unit structs with strong typing)
    // Also, can we check first is modules and interfaces .is_some() and then lauch this process?
    if let Some(modules) = &config.modules {
        if let Some(interfaces) = &modules.interfaces {
            // TODO append to a collection to make them able to be dump into a text file
            let miu_commands = prebuild_module_interfaces(config, interfaces);

            // Store the commands for later dump it to a file (probably if check if needed?)
            commands.insert(
                // Change also for a strong type
                String::from("MIU"), // this will be a strong type, so don't worry about raw str
                miu_commands
                    .iter()
                    .map(|command| command.join(" "))
                    .collect::<Vec<_>>(),
            );

            // Could this potentially be delayed until everything is up?
            for arguments in miu_commands {
                execute_command(compiler, arguments)?
            }
        }
    }

    Ok(commands)
}

/// Parses the configuration in order to build the BMIs declared for the project,
/// by precompiling the module interface units
fn prebuild_module_interfaces(
    config: &ZorkConfigFile,
    interfaces: &Vec<ModuleInterface>,
) -> Vec<Vec<String>> {
    let mut commands: Vec<Vec<String>> = Vec::with_capacity(interfaces.len());

    interfaces.iter().for_each(|module_interface| {
        commands.push(
            module_interfaces::get_module_ifc_args(config, module_interface),
        )
    });

    log::info!(
        "Module interface commands: {:?}",
        commands
            .iter()
            .map(|command| { command.join(" ") })
            .collect::<Vec<_>>()
    );

    commands
}

/// Creates the directory for output the elements generated
/// during the build process. Also, it will generate the
/// ['output_build_dir'/zork], which is a subfolder
/// where Zork dumps the things that needs to work correctly
/// under different conditions.
///
/// Under /zork, some new folders are created:
/// - a /intrinsics folder in created as well,
/// where different specific details of Zork++ are stored
/// related with the C++ compilers
///
/// - a /cache folder, where lives the metadata cached by Zork++
/// in order to track different aspects of the program (last time
/// modified files, last process build time...)
///  
/// TODO Generate the caché process, like last time project build,
/// and only rebuild files that is metadata contains a newer last
/// time modified date that the last Zork++ process
pub fn create_output_directory(base_path: &Path, config: &ZorkConfigFile) -> Result<()> {
    let out_dir = config
        .build
        .as_ref()
        .and_then(|build| build.output_dir)
        .unwrap_or(DEFAULT_OUTPUT_DIR);

    let compiler = &config.compiler.cpp_compiler;

    // Recursively create a directory and all of its parent components if they are missing
    let modules_path = Path::new(base_path)
        .join(out_dir)
        .join(compiler.to_string())
        .join("modules");
    let zork_path = base_path.join(out_dir).join("zork");
    let zork_cache_path = zork_path.join("cache");
    let zork_intrinsics_path = zork_path.join("intrinsics");

    utils::fs::create_directory(&modules_path.join("interfaces"))?;
    utils::fs::create_directory(&modules_path.join("implementations"))?;
    utils::fs::create_directory(&zork_cache_path)?;
    utils::fs::create_directory(&zork_intrinsics_path)?;

    if compiler.eq(&CppCompiler::CLANG) {
        // TODO This possible would be provisional
        utils::fs::create_file(
            &zork_intrinsics_path,
            "std.h",
            utils::template::resources::STD_HEADER.as_bytes(),
        )?;
        utils::fs::create_file(
            &zork_intrinsics_path,
            "zork.modulemap",
            utils::template::resources::ZORK_MODULEMAP.as_bytes(),
        )?;
    }

    Ok(())
}

mod module_interfaces {
    use crate::config_file::{ZorkConfigFile, modules::ModuleInterface, compiler::CppCompiler};

    use super::helpers;

    /// Generates the expected arguments for precompile the BMIs depending on self
    pub fn get_module_ifc_args(
        config: &ZorkConfigFile,
        interface: &ModuleInterface,
    ) -> Vec<String> {
        let compiler = &config.compiler.cpp_compiler;
        let base_path = config.modules.as_ref().unwrap().base_ifcs_dir;
        let out_dir = config.build.as_ref().map_or("", |build_attribute| {
            build_attribute.output_dir.unwrap_or_default()
        });

        let mut arguments = Vec::with_capacity(8);
        arguments.push(config.compiler.cpp_standard.as_cmd_arg(compiler));

        match *compiler {
            // Refactor this one?
            CppCompiler::CLANG => {
                if let Some(std_lib) = &config.compiler.std_lib {
                    arguments.push(format!("-stdlib={}", std_lib.as_str()))
                }

                arguments.push("-fimplicit-modules".to_string());
                arguments.push("-x".to_string());
                arguments.push("c++-module".to_string());
                arguments.push("--precompile".to_string());

                if std::env::consts::OS.eq("windows") {
                    arguments.push(
                        // This is a Zork++ feature to allow the users to write `import std;`
                        // under -std=c++20 with clang linking against GCC under Windows with
                        // some MinGW installation or similar.
                        // Should this be handled in another way?
                        format!("-fmodule-map-file={out_dir}/zork/intrinsics/zork.modulemap"),
                    )
                } else {
                    arguments.push("-fimplicit-module-maps".to_string())
                }

                // The resultant BMI as a .pcm file
                arguments.push("-o".to_string());
                // The output file
                arguments.push(helpers::generate_prebuild_miu(compiler, out_dir, interface));
                arguments.push(helpers::miu_input_file(interface, base_path))
            },
            CppCompiler::MSVC => {
                arguments.push("-c".to_string());
                arguments.push("-ifcOutput".to_string());
                // The output .ifc file
                arguments.push(helpers::generate_prebuild_miu(compiler, out_dir, interface));
                // The output .obj file
                arguments.push(format!("/Fo{out_dir}/{compiler}/modules/interfaces\\"));
                // The input file
                arguments.push("-interface".to_string());
                arguments.push("-TP".to_string());
                arguments.push(helpers::miu_input_file(interface, base_path))
            },
            CppCompiler::GCC => {
                arguments.push("-fmodules-ts".to_string());
                arguments.push("-c".to_string());
                
                // The input file
                arguments.push(helpers::miu_input_file(interface, base_path));
                
                // The output file
                arguments.push("-o".to_string());
                arguments.push(helpers::generate_prebuild_miu(compiler, out_dir, interface));
            },
        }

        arguments
    }
}

/// Helpers for reduce the cyclomatic complexity introduced by the
/// kind of workflow that should be done with this parse, format and
/// generate
mod helpers {
    use super::*;

    /// Formats the string that represents the input module interface file
    /// that will be passed to the compiler
    pub(crate) fn miu_input_file(interface: &ModuleInterface, base_path: Option<&str>) -> String {
        base_path.map_or_else(
            || interface.filename.to_string(),
            |bp| format!("{bp}/{}", interface.filename),
        )
    }

    pub(crate) fn generate_prebuild_miu(
        compiler: &CppCompiler,
        out_dir: &str,
        interface: &ModuleInterface,
    ) -> String {
        let miu_ext = match compiler {
            CppCompiler::CLANG => "pcm",
            CppCompiler::MSVC => "ifc",
            CppCompiler::GCC => "o",
        };

        if let Some(module_name) = interface.module_name {
            format!(
                "{out_dir}/{compiler}/modules/interfaces/{module_name}.{miu_ext}"
            )
        } else {
            format!(
                "{out_dir}/{compiler}/modules/interfaces/{}.{miu_ext}",
                interface.filename.split('.').collect::<Vec<_>>()[0]
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use color_eyre::Result;
    use tempfile::tempdir;

    use crate::utils::template::resources::CONFIG_FILE;

    use super::*;

    #[test]
    fn test_creation_directories() -> Result<()> {
        let temp = tempdir()?;

        let zcf: ZorkConfigFile = toml::from_str(CONFIG_FILE)?;

        // This should create and out/ directory in the ./zork++ folder at the root of this project
        create_output_directory(temp.path(), &zcf)?;

        assert!(temp.path().join("out").exists());
        assert!(temp.path().join("out/zork").exists());
        assert!(temp.path().join("out/zork/cache").exists());
        assert!(temp.path().join("out/zork/intrinsics").exists());

        Ok(())
    }
}
