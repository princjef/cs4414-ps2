//
// gash.rs
//
// Starting code for PS2
// Running on Rust 0.9
//
// University of Virginia - cs4414 Spring 2014
// Weilin Xu, David Evans
// Version 0.4
//

extern mod extra;

use std::{io, run, os, str};
use std::io::buffered::BufferedReader;
use std::path::posix::Path;
use std::io::fs::File;
use std::io::stdin;
use std::io::signal::{Listener, Interrupt};
use extra::getopts;

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

        // let (port_interrupt, chan_interrupt) : (Port<~str>, Chan<~str>) = Chan::new();
        let (port_PID, chan_PID) : (Port<i32>, Chan<i32>) = Chan::new();
        let (port_temp, chan_temp) : (Port<i32>, Chan<i32>) = Chan::new();
            // -99 is "exit" signal
            // -1 is "continue" signal
        chan_temp.send(-1);
        spawn(proc() {
            let mut listener = Listener::new();
            listener.register(Interrupt);

            let mut last_pid : i32 = 1;
            loop {
                match port_PID.try_recv() {
                    Some(-99) => {
                        listener.unregister(Interrupt);
                        chan_temp.send(-99);    // Exit code.
                        return;
                    }
                    Some(-1) => {
                        port_PID.recv();     // Burn nop
                        chan_temp.send(-1);    // To complete process
                    }
                    Some(pid) => {
                        println!("got one")
                        let mut my_pid : i32;
                        unsafe { my_pid = std::libc::getpid(); }
                        if (pid != my_pid) {
                            last_pid = pid;
                            println!("Changed PID to {:d}", last_pid);
                        }
                            port_PID.recv();    // Burn nop
                            chan_temp.send(-1);   // To complete process
                    }
                    None    => {}
                }
                match listener.port.try_recv() {
                    Some(Interrupt) => {
                        // chan_temp.send(port_PID.recv());
                        println!("Killing {:d}", last_pid);
                        unsafe { 
                            let result : i32 = std::libc::funcs::posix88::signal::kill(last_pid, 0);
                        } 
                    }
                    _ => {}
                }
            }
        });
        
        loop {
            let status : i32 = port_temp.recv();   // Wait for op to finish.
            if (status == -99) { return; }
            print(self.cmd_prompt); // Prints "gash >"
            io::stdio::flush();

            let mut line : ~str = stdin.read_line().unwrap().to_owned();
            let mut cmd_line: ~str = line.trim().to_owned();
            let mut background: bool = false;
            let mut i : uint = 0;

            // This block handles if there are no spaces around pipe.
            let lineClone = line.clone();
            for character in lineClone.chars() {
                if character == '<' {
                    line = line.slice(0, i).to_owned() + " < " + line.slice(i+1, line.char_len()).to_owned();
                    i = i + 2;
                } else if character == '>' {
                    line = line.slice(0, i).to_owned() + " > " + line.slice(i+1, line.char_len()).to_owned();
                    i = i + 2;
                } else if character == '|' {
                    line = line.slice(0, i).to_owned() + " | " + line.slice(i+1, line.char_len()).to_owned();
                    i = i + 2;
                }
                i = i + 1;
            }

            if cmd_line.char_len() > 0 {
                background = cmd_line.char_at(cmd_line.char_len() - 1) == '&';
                if background {
                    cmd_line = cmd_line.slice(0, cmd_line.char_len() - 1).trim().to_owned();
                }
            }

            let params = cmd_line.clone().to_owned();
            let program = cmd_line.splitn(' ', 1).nth(0).expect("no program");

            match program {
                ""          =>  { chan_PID.send(-1);    // nop
                                  chan_PID.send(-1);   // nop
                                  continue; }
                "exit"      =>  { chan_PID.send(-99); }  // Exit
                "cd"        =>  { /*chan_temp.send(-1);*/
                                  Shell::run_check_mode(background, Shell::run_cd(params), &chan_PID); }
                "history"   =>  { /*chan_temp.send(-1);*/
                                  Shell::run_check_mode(background, Shell::run_history(params, self.cmd_history), &chan_PID); }
                _           =>  { /*chan_temp.send(-1);*/
                                  Shell::run_check_mode(background, Shell::run_cmdline(params), &chan_PID); }
            }

            self.cmd_history.push(program.to_owned());
        }
    }

    fn run_check_mode(background: bool, f: proc(), chan_PID : &Chan<i32>) {
        let (port_temp, chan_temp) : (Port<i32>, Chan<i32>) = Chan::new();
        if background {
            spawn(proc() { f(); });
            chan_PID.send(-1);
            chan_PID.send(-1);
        } else {
            spawn(proc() {
                unsafe { chan_temp.send(std::libc::getpid()); 
                    println!("PID is {:d}", std::libc::getpid());
                }
                f();
                chan_temp.send(-1);
            });
            chan_PID.send(port_temp.recv());
            chan_PID.send(port_temp.recv());
        }
    }
    
    fn run_cmdline_single(cmd_line: &str) {
        (Shell::run_cmdline(cmd_line))();
    }

    fn run_cmdline(cmd_line: &str) -> proc() {
        let params = cmd_line.to_owned();
        return proc() {
            if params.trim() != "" {
                let output = Shell::handle_pipes(params, false);
                // write output to file (if necessary and exists)
                match output {
                    Some(outString) => {
                        match Shell::get_output_file(params) {
                            Some(fileName) => {
                                let mut f = File::create(&Path::new(fileName));
                                f.write(outString.into_bytes());
                            }
                            None => { /* Shouldn't happen... */ }
                        }
                    }
                    None => { /*Do nothing*/ }
                }
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
                if out_pipe {
                    true
                } else {
                    false
                }
            }
        };
        
        if pipeSplit.len() == 2 {
            return Shell::run_cmd(program, argv, Shell::handle_pipes(pipeSplit[1], true), hasOutRedirect);
        } else {
            return Shell::run_cmd(program, argv, Shell::get_input_file_contents(cmd_line), hasOutRedirect);
        }
    }
    
    fn run_cmd(program: &str, argv: &[~str], inputStr: Option<~str>, hasOutRedirect: bool) -> Option<~str> {
        if Shell::cmd_exists(program) {
            let mut options = run::ProcessOptions::new();
            options.in_fd = match inputStr {
                Some(_) => { None }
                None => { Some(0) }
            };

            options.out_fd = if hasOutRedirect {
                None
            } else {
                Some(1)
            };

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
                let processOutput = process.finish_with_output();
                return Some(str::from_utf8(processOutput.output).to_owned());
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
        let mut activeRedirect = false;
        return cmd_line.split(' ').filter_map(|x| if x == ">" || x == "<" {
            activeRedirect = true;
            None
        } else if x == "" {
            None
        } else if activeRedirect {
            activeRedirect = false;
            None
        } else {
            Some(x.to_owned())
        }).to_owned_vec();
    }

    fn get_args(cmd_line: &str) -> ~[~str] {
        return cmd_line.split(' ').filter_map(|x| if x != "" { Some(x.to_owned()) } else { None }).to_owned_vec();
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
