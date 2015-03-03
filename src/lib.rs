extern crate rand;

use std::any;
use std::char;
use std::string;
use std::ops::Neg;
use std::num::FromPrimitive;
use std::num::ToPrimitive;
use std::cmp::min;
use rand::random;
use rand::StdRng;
use rand::SeedableRng;
use rand::distributions::range::Range;
use rand::distributions::range::SampleRange;
use rand::distributions::IndependentSample;

trait Generator<T> {
    fn grow(&self, rng: &mut StdRng, size: f64) -> T;
    fn shrink(&self, rng: &mut StdRng, &T) -> T;
}

type Seed = Vec<usize>; // seed for StdRng
type Panic = Box<any::Any + Send + 'static>; // error reaped from panicking threads

#[derive(Debug)]
struct Config {
    seed: Seed,
    max_size: f64,
    max_tests: i64,
    max_shrinks: i64,
}

#[derive(Debug)]
enum Run<Input> {
    Success{
        config: Config,
    },
    Failure {
        config: Config,
        num_tests: i64,
        input: Input,
        failure: Panic,
        shrunk_input: Input,
        shrunk_failure: Panic,
    }
}

trait Property<Input> {
    fn test(&self, config: Config) -> Run<Input>;
}

struct ForAll<Input> {
    generator: Box<Generator<Input>>,
    function: Box<Fn(Input) -> () + Sync + 'static>
}

fn forall<Input>(generator: Box<Generator<Input>>, function: Box<Fn(Input) -> () + Sync + 'static>) {
    ForAll{generator: generator, function:function};
}

impl<Input: Send> Property<Input> for ForAll<Input> {
    fn test(&self, config: Config) -> Run<Input> {
        let function = self.function;
        let mut rng: StdRng = SeedableRng::from_seed(config.seed.as_slice());
        for test in (0..config.max_tests) {
            let size = config.max_size * (test.to_f64().unwrap() / config.max_tests.to_f64().unwrap());
            let input = self.generator.grow(&mut rng, size);
            let result = std::thread::spawn(|| function(input)).join();
            if result.is_err() {
                let failure = result.unwrap_err();
                let mut shrunk_input = input;
                let mut shrunk_failure = failure;
                for shrink in (0..config.max_shrinks) {
                    let next_shrunk_input = self.generator.shrink(&mut rng, &shrunk_input);
                    let result = std::thread::spawn(|| function(next_shrunk_input)).join();
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

struct U32;

impl Generator<u32> for U32 {
    fn grow(&self, rng: &mut StdRng, size: f64) -> u32 {
        Range::new(0, size.to_i64().unwrap()).ind_sample(rng);
    }
    fn shrink(&self, rng: &mut StdRng, value: &u32) -> u32 {
        Range::new(0, *value).ind_sample(rng);
    }
}

struct Char;

impl Generator<char> for Char {
    fn grow(&self, rng: &mut StdRng, size: f64) -> char {
        let char_size = min(size.to_u32().unwrap(), char::MAX as u32);
        let char_code = Range::new(0, char_size).ind_sample(rng);
        char::from_u32(char_code).unwrap() // cant fail because we used char::MAX
    }
    fn shrink(&self, rng: &mut StdRng, value: &char) -> char {
        let char_code = Range::new(0, *value as u32).ind_sample(rng);
        char::from_u32(char_code).unwrap() // cant fail because value <= char::MAX
    }
}

struct String;

impl Generator<string::String> for string::String {
    fn grow(&self, rng: &mut StdRng, size: f64) -> string::String {
        let length = Range::new(0, size.to_uint().unwrap()).ind_sample(rng);
        let string = string::String::with_capacity(length);
        for _ in (0..length) {
            string.push(Char.grow(rng, size));
        }
        string
    }
    fn shrink(&self, rng: &mut StdRng, value: &string::String) -> string::String {
        let mut value = value.clone();
        let ix = Range::new(0, value.len()).ind_sample(rng);
        let char = value.remove(ix);
        if random() {
            value.insert(ix, Char.shrink(rng, &char))
        }
        value
    }
}

// TODO have a test that uses default config and panics, for #[test]
