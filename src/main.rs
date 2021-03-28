#![feature(bindings_after_at)]
#![feature(box_patterns)]

use std::{error::Error, fs::File};
use std::io::prelude::*;

mod logic;
mod model;
mod script_engine;
mod parser;
mod script_compiler;
mod model_compiler;
mod vcd;

#[cfg(test)]
mod tests;

fn main() -> Result<(), Box<dyn Error>> {
    let mut model = model::Model::compile("
        define component Clock {
            pin out;

            script {
                out <- H;
                sleep(1000);
                out <- L;
                sleep(1000);
            }
        }

        component clk = Clock();
    ".into())?;

    let mut vcd = vcd::VcdGenerator::default();
    vcd.generate_header(&model);

    model.run(100000, |a, b| vcd.step(a, b));

    File::create("out.vcd")?.write_all(vcd.contents().as_bytes())?;

    Ok(())
}
