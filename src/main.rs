#![feature(bindings_after_at)]
#![feature(box_patterns)]

mod logic;
mod model;
mod script_engine;
mod parser;
mod script_compiler;
mod model_compiler;

#[cfg(test)]
mod tests;

fn main() {
    println!("Hello, world!");
}
