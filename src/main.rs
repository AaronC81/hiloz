#![feature(bindings_after_at)]

mod logic;
mod model;
mod script_engine;
mod script_parser;
mod script_compiler;
mod model_compiler;

#[cfg(test)]
mod tests;

fn main() {
    println!("Hello, world!");
}
