// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>.
// This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt::{Debug};

use rand_xorshift::XorShiftRng;

use serde::{Serialize, Deserialize};

use std::rc::Rc;
use std::cell::RefCell;

use std::any::Any;

use crate::simulation::wrappers::{NormalWrapper, UniformWrapper, WeibullWrapper, DistributionWrapper};

#[typetag::serde(tag = "type")]
pub trait Asynchrony: Debug + objekt::Clone {
    fn get_async(&mut self, rng: Rc<RefCell<XorShiftRng>>, ts: i32) -> i32;

    fn calculate_async(&self, ts: i32, async_factor: f64) -> i32 {
        (ts as f64 * async_factor).round() as i32
    }

    fn as_any(&self) -> &dyn Any;
}

clone_trait_object!(Asynchrony);

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct NoAsynchrony;

#[typetag::serde]
impl Asynchrony for NoAsynchrony {
    fn get_async(&mut self, _rng: Rc<RefCell<XorShiftRng>>, _ts: i32) -> i32 {
        0
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl NoAsynchrony {
    pub fn new() -> Self {
        NoAsynchrony
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct UniformAsynchrony {
    uniform: UniformWrapper
}

#[typetag::serde]
impl Asynchrony for UniformAsynchrony {
    fn get_async(&mut self, rng: Rc<RefCell<XorShiftRng>>, ts: i32) -> i32 {
        let async_factor = self.uniform.sample(&mut *rng.borrow_mut());

        let asynchrony = self.calculate_async(ts, async_factor);

        asynchrony
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl UniformAsynchrony {
    pub fn new(low: f64, high: f64) -> Self {
        UniformAsynchrony { uniform: UniformWrapper::new_inclusive(low, high) }
    }
}

impl Default for UniformAsynchrony {
    fn default() -> Self {
        UniformAsynchrony { uniform: Default::default() }
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct NormalAsynchrony {
    normal: NormalWrapper
}

#[typetag::serde]
impl Asynchrony for NormalAsynchrony {
    fn get_async(&mut self, rng: Rc<RefCell<XorShiftRng>>, ts: i32) -> i32 {
        let async_factor = self.normal.sample(&mut *rng.borrow_mut());

        let asynchrony = self.calculate_async(ts, async_factor);

        asynchrony
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl NormalAsynchrony {
    pub fn new(mean: f64, std_dev: f64) -> Self {
        NormalAsynchrony { normal: NormalWrapper::new(mean, std_dev) }
    }
}

impl Default for NormalAsynchrony {
    fn default() -> Self {
        NormalAsynchrony { normal: Default::default() }
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct WeibullAsynchrony {
    weibull: WeibullWrapper
}

#[typetag::serde]
impl Asynchrony for WeibullAsynchrony {
    fn get_async(&mut self, rng: Rc<RefCell<XorShiftRng>>, ts: i32) -> i32 {
        let async_factor = self.weibull.sample(&mut *rng.borrow_mut());

        let asynchrony = self.calculate_async(ts, async_factor);

        asynchrony
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl WeibullAsynchrony {
    pub fn new(scale: f64, shape: f64) -> Self {
        WeibullAsynchrony { weibull: WeibullWrapper::new(scale, shape) }
    }
}

impl Default for WeibullAsynchrony {
    fn default() -> Self {
        WeibullAsynchrony { weibull: Default::default() }
    }
}

impl Default for Box<Asynchrony> {
    fn default() -> Self {
        Box::new(UniformAsynchrony::default())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    struct ConfAsync {
        asynchrony: Box<Asynchrony>
    }

    #[test]
    #[cfg(feature="checkpointing")]
    fn serialize_async() {
        let asynchrony = Box::new(UniformAsynchrony::new(0.0, 1.0)) as Box<Asynchrony>;

        let serialized = serde_yaml::to_string(&asynchrony).unwrap();

        println!("{}", serialized);


        let conf_async = ConfAsync { asynchrony };

        let serialized_conf = serde_yaml::to_string(&conf_async).unwrap();

        println!("{:?}", serialized_conf);
    }

    #[test]
    #[cfg(feature="checkpointing")]
    fn deserialize_async() {
        let s: &str = "asynchrony:\n  type: UniformAsynchrony\n  uniform:\n    low: 0.0\n    high: 1.0";
        println!("{}", s);

        let _deserialized: ConfAsync = serde_yaml::from_str(&s).unwrap();
    }

    #[test]
    #[cfg(feature="checkpointing")]
    fn serde_async() {
        let asynchrony = Box::new(UniformAsynchrony::new(0.0, 1.0)) as Box<Asynchrony>;
        let conf_async = ConfAsync { asynchrony };

        let serialized = serde_yaml::to_string(&conf_async).unwrap();

        let deserialized: ConfAsync = serde_yaml::from_str(&serialized).unwrap(); 

        println!("{:?}", deserialized);
    }
}
