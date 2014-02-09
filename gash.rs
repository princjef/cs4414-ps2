//
// gash.rs
//
// Completed code for PS2
// Running on Rust 0.9
//
// University of Virginia - cs4414 Spring 2014
// Matt Pearson-Beck, Jeff Principe
//

extern mod extra;

use std::{io, run, os, str};
use std::io::buffered::BufferedReader;
use std::path::posix::Path;
use std::io::fs::File;
use std::io::stdin;
use std::io::signal::{Listener, Interrupt};
use extra::getopts;

static exit_code : i32 = -999;
static nop_code : i32 = -1;

struct Shell {
    cmd_prompt : ~str,
    cmd_history : ~[~str],
}

impl Shell {

    fn new(prompt_str: &str) -> Shell {
        Shell {
            cmd_prompt: prompt_str.to_owned(),
            cmd_history: ~[],
        }
    }
    
    fn run(&mut self) {        
        let mut stdin = BufferedReader::new(stdin());

        let (port_func, chan_func) : (Port<i32>, Chan<i32>) = Chan::new();
        let (port_status, chan_status) : (Port<i32>, Chan<i32>) = Chan::new();
        chan_status.send(nop_code);
        spawn(proc() {
            let mut listener = Listener::new();
            listener.register(Interrupt);
            loop {
                match port_func.try_recv() {
                    Some(status) if status == exit_code => {
                        listener.unregister(Interrupt);
                        unsafe { std::libc::exit(1 as std::libc::c_int); }
                        // return; // hangs if child processes running, so we used exit.
                    }
                    Some(response) => { chan_status.send(response); }   // To complete process
                    None    => {}
                }
            }
        });
        
        loop {
            port_status.recv();     // Wait for last op to finish.
            print(self.cmd_prompt); // Prints "gash >"
            io::stdio::flush();

            let line : ~str = stdin.read_line().unwrap().to_owned();
            let mut cmd_line: ~str = line.trim().to_owned();
            let mut background: bool = false;

            if cmd_line.char_len() > 0 {
                background = cmd_line.char_at(cmd_line.char_len() - 1) == '&';
                if background {
                    cmd_line = cmd_line.slice(0, cmd_line.char_len() - 1).trim().to_owned();
                }
            }

            let params = cmd_line.clone().to_owned();
            let program = cmd_line.splitn(' ', 1).nth(0).expect("no program");

            match program {
                ""          =>  { chan_func.send(nop_code);
                                  continue; }
                "exit"      =>  { chan_func.send(exit_code); }  // Exit
                "cd"        =>  { Shell::run_check_mode(background, Shell::run_cd(params), &chan_func); }
                "history"   =>  { Shell::run_check_mode(background, Shell::run_history(params, self.cmd_history), &chan_func); }
                _           =>  { Shell::run_check_mode(background, Shell::run_cmdline(params), &chan_func); }
            }

            self.cmd_history.push(program.to_owned());
        }
    }

    fn run_check_mode(background: bool, f: proc(), chan_func : &Chan<i32>) {
        let (port_temp, chan_temp) : (Port<i32>, Chan<i32>) = Chan::new();
        if background {
            spawn(proc() { 
                f(); 
            });
            chan_func.send(nop_code);  // Return immediately.
        } else {
            spawn(proc() {
                f();
                chan_temp.send(nop_code); // Wait until f returns.
            });
            chan_func.send(port_temp.recv());   // Pass on the message.
        }
    }
    
    fn run_cmdline_single(cmd_line: &str) {
        (Shell::run_cmdline(cmd_line))();
    }

    fn run_cmdline(cmd_line: &str) -> proc() {
        let params = cmd_line.to_owned();
        return proc() {
            if params.trim() != "" {
                Shell::handle_pipes(params, false);
            }
        };
    }

    fn handle_pipes(cmd_line: &str, out_pipe: bool) -> Option<~str> {
        let pipeSplit = Shell::split_on_last_pipe(cmd_line);
        let mut argv = Shell::get_args_no_redirects(pipeSplit[0]);
        let program = argv.remove(0);
        let hasOutRedirect = match Shell::get_output_file(cmd_line) {
            Some(_) => { true }
            None => {
                if out_pipe { true } 
                else { false }
            }
        };
        
        if pipeSplit.len() == 2 {
            return Shell::run_cmd(program, argv, Shell::handle_pipes(pipeSplit[1], true), hasOutRedirect, None);
        } else {
            return Shell::run_cmd(program, argv, Shell::get_input_file_contents(cmd_line), hasOutRedirect, Shell::get_output_file(cmd_line));
        }
    }
    
    fn run_cmd(program: &str, argv: &[~str], inputStr: Option<~str>, hasOutRedirect: bool, outFilename: Option<~str>) -> Option<~str> {
        if Shell::cmd_exists(program) {
            let mut options = run::ProcessOptions::new();
            options.in_fd = match inputStr {
                Some(_) => { None }
                None => { Some(0) } // 0 is stdin.
            };

            unsafe {
                options.out_fd = if hasOutRedirect {
                    match outFilename {
                        Some(ref filename) => {
                            Some(std::libc::funcs::posix88::fcntl::open(filename.to_c_str().unwrap(), std::libc::consts::os::posix88::O_RDWR | std::libc::consts::os::posix88::O_TRUNC | std::libc::consts::os::posix88::O_CREAT, std::libc::consts::os::posix88::S_IRUSR | std::libc::consts::os::posix88::S_IWUSR))
                        }
                        None => {None}
                    }
                } else {
                    Some(1) // 1 is stdout.
                };                
            }

            let mut process = run::Process::new(program, argv, options).unwrap();

            match inputStr {
                Some(string) => {
                    let buf = string.into_bytes();
                    process.input().write(buf);
                }
                None => {}
            }

            process.close_input();

            if hasOutRedirect {
                match outFilename {
                    Some(_) => {
                        process.finish();
                        return None;
                    }
                    None => {
                        let processOutput = process.finish_with_output();
                        return Some(str::from_utf8(processOutput.output).to_owned());
                    }
                }
            } else {
                process.finish();
                return None;
            };
        } else {
            println!("{:s}: command not found", program);
            return None;
        }
    }

    fn cmd_exists(cmd_path: &str) -> bool {
        let ret = run::process_output("which", [cmd_path.to_owned()]);
        return ret.expect("exit code error.").status.success();
    }

    fn run_cd(cmd_line: &str) -> proc() {
        let params = cmd_line.to_owned();
        return proc() {
            let argv: ~[~str] = Shell::get_args(params);
            let pathOpt: Option<Path> = match argv.len() {
                1   =>  { os::homedir() }
                0   =>  { os::homedir() }
                _   =>  { Some(Path::new(argv[1])) }
            };

            match pathOpt {
                Some(path)   =>  {
                    if path.is_dir() {
                        os::change_dir(&path);
                    } else {
                        println!("Error: {:s} is not a directory", path.as_str().unwrap());
                    }
                }
                None        =>  {
                    println!("Error: Invalid path");
                }
            };
        };
    }

    fn run_history(cmd_line: &str, cmd_history: &[~str]) -> proc() {
        let history = cmd_history.to_owned();
        let params = cmd_line.to_owned();
        return proc() {
            let argv: ~[~str] = Shell::get_args(params);
            if (argv.len() > 1) {
                println!("Error: history does not take options (sadly).");
            }
            else {
                let mut i = 1;
                for entry in history.iter() {
                    println!("{:d} \t{:s}", i, entry.to_owned());
                    i = i + 1;
                }
            }            
        };
    }

    fn split_on_last_pipe(cmd_line: &str) -> ~[~str] {
        return cmd_line.rsplitn('|', 1).filter_map(|x| Some(x.trim().to_owned())).to_owned_vec();
    }

    fn get_input_file_contents(cmd_line: &str) -> Option<~str> {
        return match cmd_line.splitn('<', 1).nth(1) {
            Some(inputFileCandidate) => {
                match inputFileCandidate.trim().splitn(' ', 1).nth(0) {
                    Some(inputFile) => {
                        match File::open(&Path::new(inputFile)) {
                            Some(mut file) => {
                                Some(file.read_to_str())
                            }
                            None => {
                                println!("File {:s} does not exist", inputFile);
                                None
                            }
                        }
                    }
                    None => { None }
                }
            }
            None => { None }
        };
    }

    fn get_output_file(cmd_line: &str) -> Option<~str> {
        return match cmd_line.splitn('>', 1).nth(1) {
            Some(inputFileCandidate) => {
                match inputFileCandidate.trim().splitn(' ', 1).nth(0) {
                    Some(inputFile) => { Some(inputFile.to_owned()) }
                    None => { None }
                }
            }
            None => { None }
        };
    }

    fn get_args_no_redirects(cmd_line: &str) -> ~[~str] {
        let mut activeRedirect: bool = false;
        let mut args: ~[~str] = ~[];
        let mut input: ~[~str] = Shell::get_args(cmd_line);
        for arg in input.mut_iter() {
            if *arg == ~">" || *arg == ~"<" {
                activeRedirect = true;
            } else if activeRedirect {
                activeRedirect = false;
            } else {
                let newArg = arg.to_owned();
                args.push(newArg);
            }
        }

        let result = args;
        return result;
    }

    fn get_args(cmd_line: &str) -> ~[~str] {
        let mut current: ~str = ~"";
        let mut escape: bool = false;
        let mut quoted: bool = false;
        let mut params: ~[~str] = ~[];

        for symbol in cmd_line.chars() {
            if escape {
                if symbol == 't' || symbol == 'n' || symbol == 'r' {
                    current = current + "\\";
                }
                current = current + str::from_char(symbol);
                escape = false;
            } else if symbol == ' ' || symbol == '\t' {
                if quoted {
                    current = current + str::from_char(symbol);
                } else if current.char_len() != 0 {
                    params.push(current.to_owned());
                    current = ~"";
                }
            } else if symbol == '\\' {
                escape = true;
            } else if symbol == '"' {
                quoted = !quoted;
            } else {
                current = current + str::from_char(symbol);
            }
        }

        if current.char_len() > 0 {
            params.push(current.to_owned());
        }

        let result = params;
        return result;
    }
}

fn get_cmdline_from_args() -> Option<~str> {
    /* Begin processing program arguments and initiate the parameters. */
    let args = os::args();
    
    let opts = ~[
        getopts::optopt("c")
    ];
    
    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => { m }
        Err(f) => { fail!(f.to_err_msg()) }
    };
    
    if matches.opt_present("c") {
        let cmd_str = match matches.opt_str("c") {
                                                Some(cmd_str) => {cmd_str.to_owned()}, 
                                                None => {~""}
                                              };
        return Some(cmd_str);
    } else {
        return None;
    }
}

fn main() {
    let opt_cmd_line = get_cmdline_from_args();
    
    match opt_cmd_line {
        Some(cmd_line) => Shell::run_cmdline_single(cmd_line),
        None           => Shell::new("gash > ").run()
    }
}
