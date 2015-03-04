#![feature(core)]
#![feature(collections)]

extern crate rand;

use std::any::Any;
use std::char;
use std::num::ToPrimitive;
use std::cmp::min;
use std::fmt::Debug;
use rand::random;
use rand::StdRng;
use rand::SeedableRng;
use rand::distributions::range::Range;
use rand::distributions::IndependentSample;

pub type Seed = Vec<usize>; // seed for StdRng

#[derive(Debug)]
pub struct Config {
    seed: Seed, // TODO Option<Seed> ?
    max_size: f64,
    max_tests: i64,
    max_shrinks: i64,
}

#[derive(Debug)]
pub enum Run<Input> {
    Success,
    Failure {
        num_tests: i64,
        input: Input,
        failure: String,
        shrunk_input: Input,
        shrunk_failure: String,
    }
}

impl<Input: Debug> Run<Input> {
    pub fn unwrap(self) -> () {
        match self {
            Run::Success => (),
            Run::Failure{..} => panic!(format!("Run failed: {:?}", self))
        }
    }
}

pub trait Arbitrary {
    fn grow(rng: &mut StdRng, size: f64) -> Self;
    fn shrink(rng: &mut StdRng, &Self) -> Self;
}

fn print_panic(panic: Box<Any + Send>) -> String {
    panic.downcast_ref::<&str>().unwrap_or(&"<Unprintable panic>").to_string()
}

fn catch<Input: Send + 'static>(function: fn(Input) -> (), input: Input) -> Result<(), String> {
    let handle = std::thread::spawn(move || function(input));
    handle.join().map_err(print_panic)
}

pub fn run<Input: Arbitrary + Clone + Send + 'static>(f: fn(Input) -> (), config: &Config) -> Run<Input> {
    let mut rng: StdRng = SeedableRng::from_seed(config.seed.as_slice());
    for test in (0..config.max_tests) {
        let size = config.max_size * (test.to_f64().unwrap() / config.max_tests.to_f64().unwrap());
        let input: Input = Arbitrary::grow(&mut rng, size);
        let result = catch(f, input.clone());
        if result.is_err() {
            let failure = result.unwrap_err();
            let mut shrunk_input = input.clone();
            let mut shrunk_failure = failure.clone();
            for _ in (0..config.max_shrinks) {
                let next_shrunk_input = Arbitrary::shrink(&mut rng, &shrunk_input);
                let result = catch(f, next_shrunk_input.clone());
                if result.is_err() {
                    shrunk_input = next_shrunk_input;
                    shrunk_failure = result.unwrap_err();
                }
            }
            return Run::Failure {
                num_tests: test,
                input: input,
                failure: failure,
                shrunk_input: shrunk_input,
                shrunk_failure: shrunk_failure,
            }
        }
    }
    return Run::Success
}

pub fn check<Input: Arbitrary + Debug + Clone + Send + 'static>(f: fn(Input) -> (), config: &Config) {
    run(f, config).unwrap();
}

impl Arbitrary for u32 {
    fn grow(rng: &mut StdRng, size: f64) -> u32 {
        Range::new(0, size.to_u32().unwrap() + 1).ind_sample(rng)
    }
    fn shrink(rng: &mut StdRng, value: &u32) -> u32 {
        Range::new(0, *value + 1).ind_sample(rng)
    }
}

impl Arbitrary for char {
    fn grow(rng: &mut StdRng, size: f64) -> char {
        let char_size = min(size.to_u32().unwrap(), char::MAX as u32);
        let char_code = Range::new(0, char_size + 1).ind_sample(rng);
        char::from_u32(char_code).unwrap() // cant fail because we used char::MAX
    }
    fn shrink(rng: &mut StdRng, value: &char) -> char {
        let char_code = Range::new(0, *value as u32 + 1).ind_sample(rng);
        char::from_u32(char_code).unwrap() // cant fail because value <= char::MAX
    }
}

impl Arbitrary for String {
    fn grow(rng: &mut StdRng, size: f64) -> String {
        let length = Range::new(0, size.to_uint().unwrap() + 1).ind_sample(rng);
        let mut string = String::with_capacity(length);
        for _ in (0..length) {
            string.push(Arbitrary::grow(rng, size));
        }
        string
    }
    fn shrink(rng: &mut StdRng, value: &String) -> String {
        let mut chars = value.chars().collect::<Vec<char>>();
        if chars.len() > 0 {
            let ix = Range::new(0, chars.len()).ind_sample(rng);
            let char = chars.remove(ix);
            if random() {
                chars.insert(ix, Arbitrary::shrink(rng, &char))
            }
        }
        let value = chars.drain().collect();
        value
    }
}

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
    fn test(string: String) {
        assert!(!string.starts_with("o"));
    }
    match run(test, &config) {
        Run::Failure{shrunk_input, ..} => assert_eq!(shrunk_input, "o"),
        _ => assert!(false),
    }
}