#![feature(core)]
#![feature(collections)]

extern crate rand;

use std::any::Any;
use std::char;
use std::string;
use std::num::ToPrimitive;
use std::cmp::min;
use std::fmt::Debug;
use rand::random;
use rand::StdRng;
use rand::SeedableRng;
use rand::distributions::range::Range;
use rand::distributions::IndependentSample;

pub trait Generator<T> {
    fn grow(&self, rng: &mut StdRng, size: f64) -> T;
    fn shrink(&self, rng: &mut StdRng, &T) -> T;
}

pub type Seed = Vec<usize>; // seed for StdRng

#[derive(Debug)]
pub struct Config {
    seed: Seed,
    max_size: f64,
    max_tests: i64,
    max_shrinks: i64,
}

#[derive(Debug)]
pub enum Run<Input> {
    Success{
        config: Config,
    },
    Failure {
        config: Config,
        num_tests: i64,
        input: Input,
        failure: string::String,
        shrunk_input: Input,
        shrunk_failure: string::String,
    }
}

impl<Input: Debug> Run<Input> {
    pub fn unwrap(&self) -> () {
        match *self {
            Run::Success{..} => (),
            Run::Failure{..} => panic!(format!("Run failed: {:?}", self))
        }
    }
}

pub trait Property<Input> {
    fn test(&self, config: Config) -> Run<Input>;
}

pub struct ForAll<Input> {
    generator: Box<Generator<Input>>,
    function: fn(Input) -> ()
}

pub fn for_all<Input>(generator: Box<Generator<Input>>, function: fn(Input) -> ()) -> ForAll<Input> {
    ForAll{generator:generator, function:function}
}

fn print_panic(panic: Box<Any + Send>) -> string::String {
    panic.downcast_ref::<&str>().unwrap_or(&"<Unprintable panic>").to_string()
}

fn catch<Input: Send + 'static>(function: fn(Input) -> (), input: Input) -> Result<(), string::String> {
    let handle = std::thread::spawn(move || function(input));
    handle.join().map_err(print_panic)
}

impl<Input: Send + Clone + 'static> Property<Input> for ForAll<Input> {
    fn test(&self, config: Config) -> Run<Input> {
        let mut rng: StdRng = SeedableRng::from_seed(config.seed.as_slice());
        for test in (0..config.max_tests) {
            let size = config.max_size * (test.to_f64().unwrap() / config.max_tests.to_f64().unwrap());
            let input = self.generator.grow(&mut rng, size);
            let result = catch(self.function, input.clone());
            if result.is_err() {
                let failure = result.unwrap_err();
                let mut shrunk_input = input.clone();
                let mut shrunk_failure = failure.clone();
                for _ in (0..config.max_shrinks) {
                    let next_shrunk_input = self.generator.shrink(&mut rng, &shrunk_input);
                    let result = catch(self.function, next_shrunk_input.clone());
                    if result.is_err() {
                        shrunk_input = next_shrunk_input;
                        shrunk_failure = result.unwrap_err();
                    }
                }
                return Run::Failure {
                    config: config,
                    num_tests: test,
                    input: input,
                    failure: failure,
                    shrunk_input: shrunk_input,
                    shrunk_failure: shrunk_failure,
                }
            }
        }
        return Run::Success{
            config: config,
        }
    }
}

// TODO split gens out into a module

pub struct U32;

impl Generator<u32> for U32 {
    fn grow(&self, rng: &mut StdRng, size: f64) -> u32 {
        Range::new(0, size.to_u32().unwrap() + 1).ind_sample(rng)
    }
    fn shrink(&self, rng: &mut StdRng, value: &u32) -> u32 {
        Range::new(0, *value + 1).ind_sample(rng)
    }
}

pub struct Char;

impl Generator<char> for Char {
    fn grow(&self, rng: &mut StdRng, size: f64) -> char {
        let char_size = min(size.to_u32().unwrap(), char::MAX as u32);
        let char_code = Range::new(0, char_size + 1).ind_sample(rng);
        char::from_u32(char_code).unwrap() // cant fail because we used char::MAX
    }
    fn shrink(&self, rng: &mut StdRng, value: &char) -> char {
        let char_code = Range::new(0, *value as u32 + 1).ind_sample(rng);
        char::from_u32(char_code).unwrap() // cant fail because value <= char::MAX
    }
}

pub struct String;

impl Generator<string::String> for String {
    fn grow(&self, rng: &mut StdRng, size: f64) -> string::String {
        let length = Range::new(0, size.to_uint().unwrap() + 1).ind_sample(rng);
        let mut string = string::String::with_capacity(length);
        for _ in (0..length) {
            string.push(Char.grow(rng, size));
        }
        string
    }
    fn shrink(&self, rng: &mut StdRng, value: &string::String) -> string::String {
        let mut chars = value.chars().collect::<Vec<char>>();
        if chars.len() > 0 {
            let ix = Range::new(0, chars.len()).ind_sample(rng);
            let char = chars.remove(ix);
            if random() {
                chars.insert(ix, Char.shrink(rng, &char))
            }
        }
        let value = chars.drain().collect();
        value
    }
}

// TODO have a test that uses default config and panics, for #[test]

#[test]
fn test_panic() {
    fn oh_noes(_: i64) {
        panic!("oh noes");
    }
    assert_eq!(catch(oh_noes, 0), Result::Err("oh noes".to_string()));
}

#[test]
fn test_shrinking() {
    let config: Config = Config {
        seed: vec![2264676582817791, 2472426652827647, 1173672018575359, 2619002815774719, 3338075644100607, 7399177170452479, 2208063329140735, 8682999839195135, 620332180307967, 7778401773420543],
        max_tests: 1000,
        max_shrinks: 2000,
        max_size: 1000.0,
    };
    fn test(string: string::String) {
        assert!(!string.starts_with("o"));
    }
    let gen = String;
    let run = for_all(Box::new(gen), test).test(config);
    match run {
        Run::Failure{shrunk_input, ..} => assert_eq!(shrunk_input, "o"),
        _ => assert!(false, run),
    }
}