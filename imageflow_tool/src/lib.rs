#![feature(slice_concat_ext)]


extern crate clap;
extern crate imageflow_types as s;
extern crate imageflow_core as fc;
extern crate serde_json;
extern crate chrono;
extern crate serde;
use std::io::Write;

use std::path::{Path,PathBuf};
mod cmd_build;
pub mod self_test;
mod capture;


use clap::{App, Arg, SubCommand};


pub fn main_with_exit_code() -> i32 {
    let version = s::version::one_line_version();
    let app = App::new("imageflow_tool").version(version.as_ref())
        .arg(  Arg::with_name("capture-to").long("capture-to").takes_value(true)
            .help("Run whatever you're doing in a sub-process, capturing output, input, and version detail")
        )
        .subcommand(
            SubCommand::with_name("diagnose")
                .about("Diagnostic utilities")
                .arg(
                    Arg::with_name("show-compilation-info").long("show-compilation-info")
                        .help("Show all the information stored in this executable about the environment in which it was compiled.")
                ).arg(
                Arg::with_name("self-test").long("self-test")
                    .help("Creates a 'self_tests' directory and runs self-tests"))
                .arg(
                    Arg::with_name("wait").long("wait")
                        .help("Process stays in memory until you press the enter key.")
                )
                .arg(
                    Arg::with_name("call-panic").long("call-panic")
                        .help("Triggers a Rust panic (so you can observe failure/backtrace behavior)")
                )
        )
        .subcommand(
            SubCommand::with_name("examples")
                .about("Generate usage examples")
                .arg(
                    Arg::with_name("generate").long("generate")
                        .help("Create an 'examples' directory")
                )
        )

        // --json [path]
        // --response [response_json_path]
        // --demo [name]
        // --in 0 a.png b.png
        // --out a.png

        //Eventually:
        // --local-only (prevent remote URL requests)
        // --no-io-ids (Disables interpretation of numbers in --in and --out as io_id assignment).
        // --no-clobber
        // --debug (verbose, graph export, frame export?)
        // --debug-package




        // file.json --in a.png a.png --out s.png
        // file.json --in 0 a.png 1 b.png --out 3 base64


        .subcommand(SubCommand::with_name("v0.1/build")
            .about("Runs the given operation file")
            .arg(
                Arg::with_name("in").long("in").min_values(1)
                    .multiple(true)
                    .help("Replace/add inputs for the operation file")
            )
            .arg(Arg::with_name("out").long("out").multiple(true).min_values(1)
                .help("Replace/add outputs for the operation file"))
            //.arg(Arg::with_name("demo").long("demo").takes_value(true).possible_values(&["example:200x200_png"]))
            .arg(Arg::with_name("json").long("json").takes_value(true).required(true))
            .arg(Arg::with_name("response").long("response").takes_value(true))
            .arg(Arg::with_name("bundle-to").long("bundle-to").takes_value(true).help("Copies the recipe and all dependencies into the given folder, simplifying it."))
            .arg(Arg::with_name("debug-package").long("debug-package").takes_value(true).help("Creates a debug package in the given folder so others can reproduce the behavior you are seeing"))

        );
    let matches = app.get_matches();
    if matches.is_present("capture-to"){
        let mut filtered_args = std::env::args().collect::<Vec<String>>();
        //let err = &mut std::io::stderr();
        //writeln!(err,"{:?}", filtered_args).unwrap();
        for ix in 0..filtered_args.len() {
            if filtered_args[ix] == "--capture-to"{
                //Remove this and the next arg
                filtered_args.remove(ix);
                if ix < filtered_args.len() - 1{
                    filtered_args.remove(ix);
                }
                break;
            }
        }
        filtered_args.remove(0); //Remove the tool executable itself
        //writeln!(err,"{:?}", filtered_args).unwrap();

        let cap = capture::CaptureTo::create_default(matches.value_of("capture-to").unwrap(), filtered_args);
        cap.run();
        return cap.exit_code();
    }
    //        get_matches_from_safe_borrow(std::env::args()).unwrap_or_else(|e| {
    //        // Otherwise, write to stderr and exit
    //        app.maybe_wait_for_exit(e);
    //    });
    //


    if let Some(ref matches) = matches.subcommand_matches("v0.1/build") {
        let m: &&clap::ArgMatches = matches;


        let source = if m.is_present("demo") {
            cmd_build::JobSource::NamedDemo(m.value_of("demo").unwrap().to_owned())
        } else {
            cmd_build::JobSource::JsonFile(m.value_of("json").unwrap().to_owned())
        };

        let builder =
            cmd_build::CmdBuild::parse(source, m.values_of_lossy("in"), m.values_of_lossy("out"))
                .build_maybe();
        if let Some(dir_str) = m.value_of("debug-package").and_then(|v| Some(v.to_owned())){
            builder.write_errors_maybe().unwrap();
            let dir = Path::new(&dir_str);
            builder.bundle_to(&dir);
            let curdir = std::env::current_dir().unwrap();
            std::env::set_current_dir(&dir).unwrap();
            let cap = capture::CaptureTo::create_default("recipe", vec!["--json".to_owned(), "recipe.json".to_owned()]);
            cap.run();
            //Restore current directory
            std::env::set_current_dir(&curdir).unwrap();
            let archive_name = PathBuf::from(format!("{}.zip", &dir_str));
            capture::zip_directory_nonrecursive(&dir,&archive_name.as_path()).unwrap();
            return cap.exit_code();
        }else {
            if let Some(dir) = m.value_of("bundle-to").and_then(|v| Some(v.to_owned())) {
                builder.write_errors_maybe().unwrap();
                let dir = Path::new(&dir);
                return builder.bundle_to(&dir);
            } else {
                builder.write_response_maybe(m.value_of("response"))
                    .expect("IO error writing JSON output file. Does the directory exist?");
                builder.write_errors_maybe().expect("Writing to stderr failed!");
                return builder.get_exit_code().unwrap();
            }
        }
    }

    if let Some(ref matches) = matches.subcommand_matches("diagnose") {
        let m: &&clap::ArgMatches = matches;

        if m.is_present("show-compilation-info") {
            println!("{}\n{}\n",
                     s::version::one_line_version(),
                     s::version::all_build_info_pairs());
            return 0;
        }

        if m.is_present("self-test") {
            return self_test::run(None);
        }
        if m.is_present("wait") {
            let mut input_buf = String::new();
            let input = std::io::stdin().read_line(&mut input_buf).ok().expect("Failed to read from stdin. Are you using --wait in a non-interactive shell?");
            println!("{}", input);
            return 0;
        }
        if m.is_present("call-panic") {
            panic!("Panicking on command");
        }
    }
    if let Some(ref matches) = matches.subcommand_matches("examples") {
        let m: &&clap::ArgMatches = matches;

        if m.is_present("generate") {
            self_test::export_examples(None);
            return 0;
        }
    }

    64
}
