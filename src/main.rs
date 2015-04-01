#![allow(unstable)]
#![feature(plugin)]
#[plugin] #[no_link]
extern crate regex_macros;
extern crate regex;

mod cpu;

fn main() {
	println!("Probing");
	println!("load_average: {:?}", cpu::LoadAvgProbe::probe())
}
