#![feature(bindings_after_at)]
#![feature(box_patterns)]
#![feature(or_patterns)]

use std::{error::Error, fs::File, path::PathBuf, io::prelude::*};
use structopt::StructOpt;

#[macro_use]
extern crate pest_derive;

mod logic;
mod model;
mod script_engine;
mod parser;
mod script_compiler;
mod model_compiler;
mod vcd;

#[cfg(test)]
mod tests;

#[derive(StructOpt)]
#[structopt(name="simulator")]
struct Opt {
    /// Input model file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// Output VCD file
    #[structopt(parse(from_os_str))]
    output: PathBuf,

    /// The maximum number of time units to simulate for
    #[structopt(short="t", long="max-time", default_value="100000000000")]
    max_time: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();

    // Load model
    let mut input_model = "".into();
    File::open(opt.input)?.read_to_string(&mut input_model)?;
    let mut model = model::Model::compile(input_model)?;

    println!("Model loaded with:");
    println!("  - {} component definitions", model.component_definitions.len());
    println!("  - {} component instances", model.components.len());
    println!("  - {} connections", model.connections.len());
    println!("  - {} script interpreters", model.interpreters.len());
    println!("Simulating for up to {} time units", opt.max_time);

    // Prepare VCD generator
    let mut vcd = vcd::VcdGenerator::default();
    vcd.generate_header(&model);

    // Simulate
    model.construct();
    model.run(opt.max_time, |a, b| vcd.step(a, b));

    println!("Simulation complete at {}{} time units", model.time_elapsed, if model.time_elapsed > opt.max_time { "(!!!)" } else { "" });

    // Write VCD
    File::create(opt.output)?.write_all(vcd.contents().as_bytes())?;

    Ok(())
}
