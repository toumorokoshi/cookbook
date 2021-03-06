use std::{env::args, fs, path::PathBuf};

mod commands;
mod directory;
mod script;
#[cfg(test)]
mod tests;

pub fn main() {
    let args: Vec<String> = args().peekable().collect();
    match execute(args) {
        Ok(result) => print!("{}", result),
        Err(error_message) => print!("echo {}", error_message),
    };
}

pub enum CommandType {
    Execute,
    Completion,
}

enum TargetType {
    File,
    Directory,
}

pub fn execute(raw_args: Vec<String>) -> Result<String, String> {
    let mut arguments = raw_args.iter().peekable();
    // the first argument should be location of the tome binary.
    let tome_executable = match arguments.next() {
        Some(arg) => arg,
        None => return Err(String::from("0th argument should be the tome binary")),
    };
    let first_arg = match arguments.next() {
        Some(arg) => arg,
        None => return Err(String::from("at least one argument expected")),
    };
    // if the first command is init, then we should print the
    // the contents of init, since a user is trying to instantiate.
    if first_arg == "init" {
        return commands::init(tome_executable, arguments);
    }

    let mut target = PathBuf::from(first_arg);
    // next, we determine if we have a file or a directory,
    // recursing down arguments until we've exhausted arguments
    // that match a directory or file.
    let mut target_type = TargetType::Directory;
    let mut first_arg = true;
    let mut command_type = CommandType::Execute;
    // if no argument is passed, return help.
    if let None = arguments.peek() {
        match commands::help(target.to_str().unwrap_or_default(), arguments) {
            Ok(message) => return Ok(message),
            Err(io_error) => return Err(format!("{}", io_error)),
        }
    }
    loop {
        if let Some(arg) = arguments.peek() {
            // match against builtin commands
            if first_arg {
                match arg.as_ref() {
                    "--help" => {
                        arguments.next();
                        match commands::help(target.to_str().unwrap_or_default(), arguments) {
                            Ok(message) => return Ok(message),
                            Err(io_error) => return Err(format!("{}", io_error)),
                        }
                    }
                    "--complete" => {
                        arguments.next();
                        command_type = CommandType::Completion;
                        continue;
                    }
                    _ => {}
                }
            }
            first_arg = false;
            target.push(arg);
            if target.is_file() {
                target_type = TargetType::File;
                arguments.next();
                break;
            } else if target.is_dir() {
                target_type = TargetType::Directory;
                arguments.next();
            } else {
                // the current argument does not match
                // a directory or a file, so we've landed
                // on the strictest match.
                target.pop();
                break;
            }
        } else {
            break;
        }
    }
    let remaining_args = {
        let mut remaining_args = vec![];
        for arg in arguments {
            remaining_args.push(arg);
        }
        remaining_args
    };
    let output: String = match target_type {
        TargetType::Directory => match command_type {
            CommandType::Completion => {
                let mut result = vec![];
                let mut paths: Vec<_> = fs::read_dir(target.to_str().unwrap_or(""))
                    .unwrap()
                    .map(|r| r.unwrap())
                    .collect();
                paths.sort_by_key(|f| f.path());
                for path_buf in paths {
                    let path = path_buf.path();
                    if path.is_dir() && !directory::is_tome_script_directory(&path) {
                        continue;
                    }
                    if path.is_file()
                        && !script::is_tome_script(
                            path_buf.file_name().to_str().unwrap_or_default(),
                        )
                    {
                        continue;
                    }
                    result.push(path.file_name().unwrap().to_str().unwrap_or("").to_owned());
                }
                result.join(" ").to_owned()
            }
            CommandType::Execute => {
                return match remaining_args.len() {
                    0 => Err(format!(
                        "{} is a directory. tab-complete to choose subcommands",
                        target.to_str().unwrap_or("")
                    )),
                    _ => Err(format!(
                        "command {} not found in directory {}",
                        remaining_args[0],
                        target.to_str().unwrap_or("")
                    )),
                };
            }
        },
        TargetType::File => match commands::Script::load(&target.to_str().unwrap_or_default()) {
            Ok(script) => script.get_execution_body(command_type, &remaining_args)?,
            Err(error) => return Err(format!("IOError loading file: {:?}", error)),
        },
    };
    return Ok(output);
}
