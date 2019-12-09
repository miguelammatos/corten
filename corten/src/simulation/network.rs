// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>.
// This file may not be copied, modified, or distributed
// except according to those terms.

use std::fs::File;
use std::io::{BufRead, BufReader};

use std::path::Path;
use std::fmt::{Debug};

use rand::distributions::{Distribution, Uniform};
use rand_xorshift::XorShiftRng;

use std::fmt;
use serde::Serialize;
use serde::de::{self, Deserialize, Deserializer, Visitor, SeqAccess, MapAccess};

use std::rc::Rc;
use std::cell::RefCell;

use crate::simulation::wrappers::{UniformWrapper, LogNormalWrapper, DistributionWrapper};
use crate::simulation::Time;
use crate::simulation::ProcessId;

#[typetag::serde(tag = "type")]
pub trait Network: Debug + objekt::Clone {
    fn get_latency(&mut self, rng: Rc<RefCell<XorShiftRng>>, sender: ProcessId, target: ProcessId) -> Option<Time>;
}

clone_trait_object!(Network);

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct ConstantNetwork {
    latency: Time,
    jitter: Box<Jitter>,
    loss: f64,
    #[serde(skip, default = "uniform_default")]
    uniform: Uniform<f64>
}

fn uniform_default() -> Uniform<f64> {
    Uniform::new(0.0, 1.0)
}

impl ConstantNetwork {
    pub fn new(latency: Time, jitter: Box<Jitter>, loss: f64) -> Self {
        ConstantNetwork { latency, jitter, loss, uniform: Uniform::new(0.0, 1.0) }
    }
}

#[typetag::serde]
impl Network for ConstantNetwork {
    fn get_latency(&mut self, rng: Rc<RefCell<XorShiftRng>>, _sender: ProcessId, _target: ProcessId) -> Option<Time> {
        if self.uniform.sample(&mut *rng.borrow_mut()) < self.loss {
            None
        } else {
            Some(self.latency + self.jitter.get_jitter(rng.clone(), self.latency))
        }
    }
}

impl Default for ConstantNetwork {
    fn default() -> Self {
        ConstantNetwork::new(100, Box::new(NoJitter), 0.0)
    }
}

#[derive(Serialize)]
#[derive(Debug, Clone)]
pub struct MatrixNetwork {
    latency_matrix: Vec<Vec<Time>>,
    jitter: Box<Jitter>,
    loss: f64,
    #[serde(skip, default = "uniform_default")]
    uniform: Uniform<f64>
}

impl MatrixNetwork {
    /// assumes file with the following structure:
    /// sender node, target node, latency, each separated by '\t',
    /// where the identifiers of the nodes go from 0 to num_nodes-1
    pub fn new<P: AsRef<Path>>(filename: P, n: ProcessId, jitter: Box<Jitter>, loss: f64) -> Self {
        let mut latency_matrix: Vec<Vec<Time>> = (0..n).map(|x| vec![0; (x+1) as usize]).collect();

        let mut num_nodes_file = 0;
        let file = File::open(filename).unwrap();
        for line_result in BufReader::new(file).lines() {
            let line = line_result.unwrap();
            let content = line.split('\t').collect::<Vec<&str>>();

            let i = content[0].parse::<ProcessId>().unwrap();
            let j = content[1].parse::<ProcessId>().unwrap();
            let latency_float = content[2].parse::<f32>().unwrap().round();
            let latency = latency_float as Time;

            if i < n && j < n {
                MatrixNetwork::set_latency(&mut latency_matrix, i, j, latency);
            }

            if i > num_nodes_file {
                num_nodes_file = i;
            }
            if j > num_nodes_file {
                num_nodes_file = j;
            }
        }
        num_nodes_file += 1;

        //in case n > nodes in file
        for i in num_nodes_file..n {
            for j in 0..i {
                let latency = MatrixNetwork::get_latency(&mut latency_matrix, i % num_nodes_file, j);
                MatrixNetwork::set_latency(&mut latency_matrix, i, j, latency);
            }

            let latencies_sum: Time = latency_matrix[i as usize].iter().sum();
            let average_latency = latencies_sum as f32 / (i - 1) as f32;
            MatrixNetwork::set_latency(&mut latency_matrix, i, i % num_nodes_file, average_latency.round() as Time);
        }

        MatrixNetwork { latency_matrix, jitter, loss, uniform: Uniform::new(0.0, 1.0) }
    }
    fn new_from_matrix(latency_matrix: Vec<Vec<Time>>, jitter: Box<Jitter>, loss: f64) -> Self {
        MatrixNetwork { latency_matrix, jitter, loss, uniform: Uniform::new(0.0, 1.0) }
    }
    fn set_latency(latency_matrix: &mut Vec<Vec<Time>>, sender: ProcessId, target: ProcessId, latency: Time) {
        if sender > target {
            latency_matrix[sender as usize][target as usize] = latency;
        } else {
            latency_matrix[target as usize][sender as usize] = latency;
        }
    }
    fn get_latency(latency_matrix: &mut Vec<Vec<Time>>, sender: ProcessId, target: ProcessId) -> Time {
        if sender > target {
            latency_matrix[sender as usize][target as usize]
        } else {
            latency_matrix[target as usize][sender as usize]
        }
    }
}

#[typetag::serde]
impl Network for MatrixNetwork {
    fn get_latency(&mut self, rng: Rc<RefCell<XorShiftRng>>, sender: ProcessId, target: ProcessId) -> Option<Time> {
        if self.uniform.sample(&mut *rng.borrow_mut()) < self.loss {
            None
        } else {
            if sender > target {
                let latency = self.latency_matrix[sender as usize][target as usize];
                Some(latency + self.jitter.get_jitter(rng.clone(), latency))
            } else {
                let latency = self.latency_matrix[target as usize][sender as usize];
                Some(latency + self.jitter.get_jitter(rng.clone(), latency))
            }
        }
    }
}

impl<'de> Deserialize<'de> for MatrixNetwork {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[allow(non_camel_case_types)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field { Latency_File, N, Latency_Matrix, Jitter, Loss };

        struct MatrixNetworkVisitor;

        impl<'de> Visitor<'de> for MatrixNetworkVisitor {
            type Value = MatrixNetwork;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct MatrixNetwork")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<MatrixNetwork, V::Error>
                where
                    V: SeqAccess<'de>,
            {
                if let Some(3) = seq.size_hint() {
                    let latency_matrix = seq.next_element()?
                        .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                    let jitter = seq.next_element()?
                        .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                    let loss = seq.next_element()?
                        .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                    Ok(MatrixNetwork::new_from_matrix(latency_matrix, jitter, loss))
                } else {
                    let latency_file: String = seq.next_element()?
                        .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                    let n = seq.next_element()?
                        .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                    let jitter = seq.next_element()?
                        .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                    let loss = seq.next_element()?
                        .ok_or_else(|| de::Error::invalid_length(3, &self))?;
                    Ok(MatrixNetwork::new(latency_file, n, jitter, loss))
                }
            }

            fn visit_map<V>(self, mut map: V) -> Result<MatrixNetwork, V::Error>
                where
                    V: MapAccess<'de>,
            {
                let mut latency_file = None;
                let mut n = None;
                let mut latency_matrix = None;
                let mut jitter = None;
                let mut loss = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Latency_File => {
                            if latency_file.is_some() {
                                return Err(de::Error::duplicate_field("latency_file"));
                            }
                            latency_file = Some(map.next_value()?);
                        },
                        Field::N => {
                            if n.is_some() {
                                return Err(de::Error::duplicate_field("n"));
                            }
                            n = Some(map.next_value()?);
                        },
                        Field::Latency_Matrix => {
                            if latency_matrix.is_some() {
                                return Err(de::Error::duplicate_field("latency_matrix"));
                            }
                            latency_matrix = Some(map.next_value()?);
                        },
                        Field::Jitter => {
                            if jitter.is_some() {
                                return Err(de::Error::duplicate_field("jitter"));
                            }
                            jitter = Some(map.next_value()?);
                        },
                        Field::Loss => {
                            if loss.is_some() {
                                return Err(de::Error::duplicate_field("loss"));
                            }
                            loss = Some(map.next_value()?);
                        }
                    }
                }

                let jitter = jitter.ok_or_else(|| de::Error::missing_field("jitter"))?;
                let loss = loss.ok_or_else(|| de::Error::missing_field("loss"))?;

                if latency_file == None && latency_matrix != None {
                    let latency_matrix = latency_matrix.ok_or_else(|| de::Error::missing_field("latency_matrix"))?;

                    return Ok(MatrixNetwork::new_from_matrix(latency_matrix, jitter, loss));
                } else {
                    let latency_file: String = latency_file.ok_or_else(|| de::Error::missing_field("latency_file"))?;
                    let n = n.ok_or_else(|| de::Error::missing_field("n"))?;

                    return Ok(MatrixNetwork::new(latency_file, n, jitter, loss));
                }
            }
        }

        const FIELDS: &'static [&'static str] = &["mean", "std_dev"];
        deserializer.deserialize_struct("MatrixNetwork", FIELDS, MatrixNetworkVisitor)
    }
}


#[typetag::serde(tag = "type")]
pub trait Jitter: Debug + objekt::Clone {
    fn get_jitter(&mut self, rng: Rc<RefCell<XorShiftRng>>, latency: Time) -> Time;

    fn calculate_jitter(&self, latency: Time, jitter_factor: f64) -> Time {
        (latency as f64 * jitter_factor).round() as Time
    }
}

clone_trait_object!(Jitter);

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct NoJitter;

#[typetag::serde]
impl Jitter for NoJitter {
    fn get_jitter(&mut self, _rng: Rc<RefCell<XorShiftRng>>, _latency: Time) -> Time {
        0
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct UniformJitter {
    uniform: UniformWrapper
}

#[typetag::serde]
impl Jitter for UniformJitter {
    fn get_jitter(&mut self, rng: Rc<RefCell<XorShiftRng>>, latency: Time) -> Time {
        let jitter_factor = self.uniform.sample(&mut *rng.borrow_mut());
        let jitter = self.calculate_jitter(latency, jitter_factor);

        jitter
    }
}

impl UniformJitter {
    pub fn new(low: f64, high: f64) -> Self {
        UniformJitter { uniform: UniformWrapper::new_inclusive(low, high) }
    }
}

impl Default for UniformJitter {
    fn default() -> Self {
        UniformJitter::new(-0.5, 0.5)
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct LogNormalJitter {
    log_normal: LogNormalWrapper
}

#[typetag::serde]
impl Jitter for LogNormalJitter {
    fn get_jitter(&mut self, rng: Rc<RefCell<XorShiftRng>>, latency: Time) -> Time {
        let jitter_factor = self.log_normal.sample(&mut *rng.borrow_mut());
        let jitter = self.calculate_jitter(latency, jitter_factor);

        jitter
    }
}

impl LogNormalJitter {
    pub fn new(mean: f64, std_dev: f64) -> Self {
        LogNormalJitter { log_normal: LogNormalWrapper::new(mean, std_dev) }
    }
}

impl Default for LogNormalJitter {
    fn default() -> Self {
        LogNormalJitter::new(0.0, 0.5)
    }
}

impl Default for Box<Network> {
    fn default() -> Self {
        Box::new(ConstantNetwork::default())
    }
}


#[cfg(test)]
mod test {
    use crate::simulation::Time;
    use crate::simulation::network::NoJitter;
    use crate::simulation::network::MatrixNetwork;

    static FILENAME: &str = "data/pl_226nodes.latencies";

    #[test]
    fn test_matrix_exact_nodes() {
        let total_nodes = 226;
        let network = MatrixNetwork::new(FILENAME, total_nodes, Box::new(NoJitter), 0.0);
        assert_eq!(network.latency_matrix.len(), 226);
    }

    #[test]
    fn test_matrix_less_nodes() {
        let total_nodes = 10;
        let network = MatrixNetwork::new(FILENAME, total_nodes, Box::new(NoJitter), 0.0);
        assert_eq!(network.latency_matrix.len(), 10);
    }

    #[test]
    fn test_matrix_one_more_node() {
        let total_nodes = 227;
        let network = MatrixNetwork::new(FILENAME, total_nodes, Box::new(NoJitter), 0.0);
        assert_eq!(network.latency_matrix.len(), total_nodes as usize);
        for i in 1..(total_nodes-1) {
            let latency = network.latency_matrix[(total_nodes-1) as usize][i as usize];
            let compare_latency = network.latency_matrix[i as usize][0 as usize];
            assert_ne!(latency, 0);
            assert_eq!(latency, compare_latency);
        }
        let mut latencies_sum = 0;
        for i in 1..total_nodes {
            latencies_sum += network.latency_matrix[(total_nodes-1) as usize][i as usize];
        }
        let latency = latencies_sum as f32 / (total_nodes - 2) as f32;
        assert_eq!(network.latency_matrix[(total_nodes-1) as usize][0], latency.round() as Time);
        assert_eq!(network.latency_matrix[(total_nodes-1) as usize][(total_nodes-1) as usize], 0);
    }
}


#[cfg(test)]
mod test_checkpointing {
    use super::*;

    use serde_yaml;

    #[derive(Serialize, Deserialize, Debug)]
    struct ConfNetwork {
        network: Box<Network>
    }

    #[test]
    fn serialize_network() {
        let jitter = Box::new(LogNormalJitter::new(0.0, 0.1));
        let network = Box::new(ConstantNetwork::new(100, jitter, 0.05)) as Box<Network>;

        let serialized = serde_yaml::to_string(&network).unwrap();

        println!("{}", serialized);


        let conf_network = ConfNetwork { network };

        let serialized_conf = serde_yaml::to_string(&conf_network).unwrap();

        println!("{:?}", serialized_conf);
    }

    #[test]
    fn deserialize_constant_network() {
        let s: &str = "network:\n  type: ConstantNetwork\n  latency: 100\n  jitter:\n    type: LogNormalJitter\n    log_normal:\n        mean: 0.0\n        std_dev: 0.1\n  loss: 0.05";
        println!("{}", s);

        let _deserialized: ConfNetwork = serde_yaml::from_str(&s).unwrap();
    }

    #[test]
    fn deserialize_matrix_network() {
        let s: &str = "network:\n  type: MatrixNetwork\n  latency_file: \"data/pl_226nodes.latencies\"\n  n: 10\n  jitter:\n    type: LogNormalJitter\n    log_normal:\n        mean: 0.0\n        std_dev: 0.1\n  loss: 0.05";
        println!("{}", s);

        let _deserialized: ConfNetwork = serde_yaml::from_str(&s).unwrap();
    }

    #[test]
    fn serde_network() {
        let jitter = Box::new(LogNormalJitter::new(0.0, 0.1));
        let network = Box::new(ConstantNetwork::new(100, jitter, 0.05)) as Box<Network>;
        let conf_network = ConfNetwork { network };

        let serialized = serde_yaml::to_string(&conf_network).unwrap();

        let deserialized: ConfNetwork = serde_yaml::from_str(&serialized).unwrap(); 

        println!("{:?}", deserialized);
    }
}
