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

use std::{io, run, os};
use std::io::buffered::BufferedReader;
// use std::io::stdio::StdReader;
// use std::io::Reader;
// use std::io::Writer;
use std::io::stdin;
use extra::getopts;

struct Shell {
    cmd_prompt: ~str,
    cmd_history: ~[~str],
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
        
        loop {
            print(self.cmd_prompt);
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
                ""          =>  { continue; }
                "exit"      =>  { return; }
                "cd"        =>  { Shell::run_check_mode(background, Shell::run_cd(params)); }
                "history"   =>  { Shell::run_check_mode(background, Shell::run_history(params, self.cmd_history)); }
                _           =>  { Shell::run_check_mode(background, Shell::run_cmdline(params)); }
            }

            self.cmd_history.push(program.to_owned());
        }
    }

    fn run_check_mode(background: bool, f: proc()) {
        if background {
            spawn(proc() { f(); });
        } else {
            f();
        }
    }
    
    fn run_cmdline_single(cmd_line: &str) {
        (Shell::run_cmdline(cmd_line))();
    }

    fn run_cmdline(cmd_line: &str) -> proc() {
        let params = cmd_line.to_owned();
        return proc() {
            let mut argv: ~[~str] = Shell::get_args(params);
        
            if argv.len() > 0 {
                let program: ~str = argv.remove(0);
                Shell::run_cmd(program, argv/*, None, false*/);
            }
        };
    }
    
    fn run_cmd(program: &str, argv: &[~str]/*, inputStream: Option<~Reader>, hasOutRedirect: bool*/) /*-> Option<~[u8]>*/ {
        if Shell::cmd_exists(program) {
            run::process_status(program, argv);
            
            // let mut options = run::ProcessOptions::new();
            // options.in_fd = match inputStream {
            //     Some(reader) => { None }
            //     None => { Some(0) }
            // };

            // if hasOutRedirect {
            //     options.out_fd = None;
            // } else {
            //     options.out_fd = Some(1);
            // }

            // let mut process = run::Process::new(program, argv, options).unwrap();
            // match options.in_fd {
            //     Some(_) => {  }
            //     None => {
            //         let buf = inputStream.read_to_str().into_bytes();
            //         process.input().write(buf);
            //     }
            // }

            // if hasOutRedirect {
            //     let mut processOutput = process.finish_with_output();
            //     return Some(processOutput.output);

            // } else {
            //     process.finish();
            //     return None;
            // }
        } else {
            println!("{:s}: command not found", program);
            // return None;
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
