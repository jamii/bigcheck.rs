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

// TODO is there any way to avoid cloning input each time?
//      we can't consume the input because it may take multiple shrinks to find a failing input

impl<A: Arbitrary + Clone, B: Arbitrary + Clone> Arbitrary for (A,B) {
    fn grow(rng: &mut StdRng, size: f64) -> (A,B) {
        (Arbitrary::grow(rng, size), Arbitrary::grow(rng, size))
    }
    fn shrink(rng: &mut StdRng, value: &(A,B)) -> (A,B) {
        let (a, b) = value.clone();
        match Range::new(0, 2).ind_sample(rng) {
            0 => (Arbitrary::shrink(rng, &a), b),
            1 => (a, Arbitrary::shrink(rng, &b)),
            _ => unreachable!(),
        }
    }
}

impl<A: Arbitrary + Clone, B: Arbitrary + Clone, C: Arbitrary + Clone> Arbitrary for (A,B,C) {
    fn grow(rng: &mut StdRng, size: f64) -> (A,B,C) {
        (Arbitrary::grow(rng, size), Arbitrary::grow(rng, size), Arbitrary::grow(rng, size))
    }
    fn shrink(rng: &mut StdRng, value: &(A,B,C)) -> (A,B,C) {
        let (a, b, c) = value.clone();
        match Range::new(0, 3).ind_sample(rng) {
            0 => (Arbitrary::shrink(rng, &a), b, c),
            1 => (a, Arbitrary::shrink(rng, &b), c),
            2 => (a, b, Arbitrary::shrink(rng, &c)),
            _ => unreachable!(),
        }
    }
}

impl<A: Arbitrary + Clone, B: Arbitrary + Clone, C: Arbitrary + Clone, D: Arbitrary + Clone> Arbitrary for (A,B,C,D) {
    fn grow(rng: &mut StdRng, size: f64) -> (A,B,C,D) {
        (Arbitrary::grow(rng, size), Arbitrary::grow(rng, size), Arbitrary::grow(rng, size), Arbitrary::grow(rng, size))
    }
    fn shrink(rng: &mut StdRng, value: &(A,B,C,D)) -> (A,B,C,D) {
        let (a, b, c, d) = value.clone();
        match Range::new(0, 4).ind_sample(rng) {
            0 => (Arbitrary::shrink(rng, &a), b, c, d),
            1 => (a, Arbitrary::shrink(rng, &b), c, d),
            2 => (a, b, Arbitrary::shrink(rng, &c), d),
            3 => (a, b, c, Arbitrary::shrink(rng, &d)),
            _ => unreachable!(),
        }
    }
}

impl<A: Arbitrary + Clone, B: Arbitrary + Clone, C: Arbitrary + Clone, D: Arbitrary + Clone, E: Arbitrary + Clone> Arbitrary for (A,B,C,D,E) {
    fn grow(rng: &mut StdRng, size: f64) -> (A,B,C,D,E) {
        (Arbitrary::grow(rng, size), Arbitrary::grow(rng, size), Arbitrary::grow(rng, size), Arbitrary::grow(rng, size), Arbitrary::grow(rng, size))
    }
    fn shrink(rng: &mut StdRng, value: &(A,B,C,D,E)) -> (A,B,C,D,E) {
        let (a, b, c, d, e) = value.clone();
        match Range::new(0, 5).ind_sample(rng) {
            0 => (Arbitrary::shrink(rng, &a), b, c, d, e),
            1 => (a, Arbitrary::shrink(rng, &b), c, d, e),
            2 => (a, b, Arbitrary::shrink(rng, &c), d, e),
            3 => (a, b, c, Arbitrary::shrink(rng, &d), e),
            4 => (a, b, c, d, Arbitrary::shrink(rng, &e)),
            _ => unreachable!(),
        }
    }
}

// TODO more tuples :(

impl<A: Arbitrary + Clone> Arbitrary for Vec<A> {
    fn grow(rng: &mut StdRng, size: f64) -> Vec<A> {
        let length = Range::new(0, size.to_uint().unwrap() + 1).ind_sample(rng);
        let mut vec = Vec::with_capacity(length);
        for _ in (0..length) {
            vec.push(Arbitrary::grow(rng, size));
        }
        vec
    }
    fn shrink(rng: &mut StdRng, value: &Vec<A>) -> Vec<A> {
        let mut vec = value.clone();
        if vec.len() > 0 {
            let ix = Range::new(0, vec.len()).ind_sample(rng);
            let elem = vec.remove(ix);
            if random() {
                vec.insert(ix, Arbitrary::shrink(rng, &elem))
            }
        }
        vec
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