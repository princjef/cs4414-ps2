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
            
            let line = stdin.read_line().unwrap();
            let cmd_line = line.trim().to_owned();
            let program = cmd_line.splitn(' ', 1).nth(0).expect("no program");
            
            match program {
                ""          =>  { continue; }
                "exit"      =>  { return; }
                "cd"        =>  { self.run_cd(cmd_line); }
                "history"   =>  { self.run_history(cmd_line); }
                _           =>  { self.run_cmdline(cmd_line); }
            }

            self.cmd_history.push(program.to_owned());
        }
    }
    
    fn run_cmdline(&mut self, cmd_line: &str) {
        let mut argv: ~[~str] = self.get_args(cmd_line);
    
        if argv.len() > 0 {
            let program: ~str = argv.remove(0);
            self.run_cmd(program, argv);
        }
    }
    
    fn run_cmd(&mut self, program: &str, argv: &[~str]) {
        if self.cmd_exists(program) {
            run::process_status(program, argv);
        } else {
            println!("{:s}: command not found", program);
        }
    }

    fn cmd_exists(&mut self, cmd_path: &str) -> bool {
        let ret = run::process_output("which", [cmd_path.to_owned()]);
        return ret.expect("exit code error.").status.success();
    }

    fn run_cd(&mut self, cmd_line: &str) {
        let argv: ~[~str] = self.get_args(cmd_line);
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
    }

    fn run_history(&mut self, cmd_line: &str) {
        let argv: ~[~str] = self.get_args(cmd_line);
        if (argv.len() > 1) {
            println!("Error: history does not take options (sadly).");
        }
        else {
            let mut i = 1;
            for entry in self.cmd_history.iter() {
                println!("{:d} \t{:s}", i, entry.to_owned());
                i = i + 1;
            }
        }

    }

    fn get_args(&mut self, cmd_line: &str) -> ~[~str] {
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
        Some(cmd_line) => Shell::new("").run_cmdline(cmd_line),
        None           => Shell::new("gash > ").run()
    }
}
