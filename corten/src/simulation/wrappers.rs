// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>.
// This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;

use rand::prelude::*;
use rand::distributions::{Normal, Uniform, Weibull, LogNormal, Distribution};

use serde::{Serialize};
use serde::de::{self, Deserialize, Deserializer, Visitor, SeqAccess, MapAccess};

pub trait DistributionWrapper {
    fn sample<R: Rng>(&self, rng: &mut R) -> f64;
}

#[derive(Debug, Clone, Serialize)]
pub struct NormalWrapper {
    #[serde(skip_serializing)]
    normal: Normal,
    mean: f64,
    std_dev: f64
}

impl NormalWrapper {
    pub fn new(mean: f64, std_dev: f64) -> Self {
        NormalWrapper { mean, std_dev, normal: Normal::new(mean, std_dev) }
    }
}

impl DistributionWrapper for NormalWrapper {
    fn sample<R: Rng>(&self, rng: &mut R) -> f64 {
        self.normal.sample(rng)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UniformWrapper {
    #[serde(skip_serializing)]
    uniform: Uniform<f64>,
    low: f64,
    high: f64
}

impl UniformWrapper {
    pub fn new_inclusive(low: f64, high: f64) -> Self {
        UniformWrapper { low, high, uniform: Uniform::new_inclusive(low, high) }
    }
    pub fn new(low: f64, high: f64) -> Self {
        UniformWrapper { low, high, uniform: Uniform::new(low, high) }
    }
}

impl DistributionWrapper for UniformWrapper {
    fn sample<R: Rng>(&self, rng: &mut R) -> f64 {
        self.uniform.sample(rng)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WeibullWrapper {
    #[serde(skip_serializing)]
    weibull: Weibull,
    scale: f64,
    shape: f64
}

impl WeibullWrapper {
    pub fn new(scale: f64, shape: f64) -> Self {
        WeibullWrapper { scale, shape, weibull: Weibull::new(scale, shape) }
    }
}

impl DistributionWrapper for WeibullWrapper {
    fn sample<R: Rng>(&self, rng: &mut R) -> f64 {
        self.weibull.sample(rng)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LogNormalWrapper {
    #[serde(skip_serializing)]
    log_normal: LogNormal,
    mean: f64,
    std_dev: f64
}

impl LogNormalWrapper {
    pub fn new(mean: f64, std_dev: f64) -> Self {
        LogNormalWrapper { mean, std_dev, log_normal: LogNormal::new(mean, std_dev) }
    }
}

impl DistributionWrapper for LogNormalWrapper {
    fn sample<R: Rng>(&self, rng: &mut R) -> f64 {
        self.log_normal.sample(rng)
    }
}


impl Default for UniformWrapper {
    fn default() -> Self {
        UniformWrapper::new_inclusive(-0.1, 0.1)
    }
}

impl Default for NormalWrapper {
    fn default() -> Self {
        NormalWrapper::new(0.0, 0.1)
    }
}

impl Default for WeibullWrapper {
    fn default() -> Self {
        WeibullWrapper::new(1.0, 1.5)
    }
}

impl Default for LogNormalWrapper {
    fn default() -> Self {
        LogNormalWrapper::new(0.0, 0.5)
    }
}

impl<'de> Deserialize<'de> for NormalWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[allow(non_camel_case_types)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field { Mean, Std_Dev };

        struct NormalWrapperVisitor;

        impl<'de> Visitor<'de> for NormalWrapperVisitor {
            type Value = NormalWrapper;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct NormalWrapper")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<NormalWrapper, V::Error>
                where
                    V: SeqAccess<'de>,
            {
                let mean = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let std_dev = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(NormalWrapper::new(mean, std_dev))
            }

            fn visit_map<V>(self, mut map: V) -> Result<NormalWrapper, V::Error>
                where
                    V: MapAccess<'de>,
            {
                let mut mean = None;
                let mut std_dev = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Mean => {
                            if mean.is_some() {
                                return Err(de::Error::duplicate_field("mean"));
                            }
                            mean = Some(map.next_value()?);
                        }
                        Field::Std_Dev => {
                            if std_dev.is_some() {
                                return Err(de::Error::duplicate_field("std_dev"));
                            }
                            std_dev = Some(map.next_value()?);
                        }
                    }
                }
                let mean = mean.ok_or_else(|| de::Error::missing_field("mean"))?;
                let std_dev = std_dev.ok_or_else(|| de::Error::missing_field("std_dev"))?;
                Ok(NormalWrapper::new(mean, std_dev))
            }
        }

        const FIELDS: &'static [&'static str] = &["mean", "std_dev"];
        deserializer.deserialize_struct("NormalWrapper", FIELDS, NormalWrapperVisitor)
    }
}

impl<'de> Deserialize<'de> for UniformWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field { Low, High };

        struct UniformWrapperVisitor;

        impl<'de> Visitor<'de> for UniformWrapperVisitor {
            type Value = UniformWrapper;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct UniformWrapper")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<UniformWrapper, V::Error>
                where
                    V: SeqAccess<'de>,
            {
                let low = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let high = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(UniformWrapper::new_inclusive(low, high))
            }

            fn visit_map<V>(self, mut map: V) -> Result<UniformWrapper, V::Error>
                where
                    V: MapAccess<'de>,
            {
                let mut low = None;
                let mut high = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Low => {
                            if low.is_some() {
                                return Err(de::Error::duplicate_field("low"));
                            }
                            low = Some(map.next_value()?);
                        }
                        Field::High => {
                            if high.is_some() {
                                return Err(de::Error::duplicate_field("high"));
                            }
                            high = Some(map.next_value()?);
                        }
                    }
                }
                let low = low.ok_or_else(|| de::Error::missing_field("low"))?;
                let high = high.ok_or_else(|| de::Error::missing_field("high"))?;
                Ok(UniformWrapper::new_inclusive(low, high))
            }
        }

        const FIELDS: &'static [&'static str] = &["low", "high"];
        deserializer.deserialize_struct("UniformWrapper", FIELDS, UniformWrapperVisitor)
    }
}

impl<'de> Deserialize<'de> for WeibullWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field { Scale, Shape };

        struct WeibullWrapperVisitor;

        impl<'de> Visitor<'de> for WeibullWrapperVisitor {
            type Value = WeibullWrapper;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct WeibullWrapper")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<WeibullWrapper, V::Error>
                where
                    V: SeqAccess<'de>,
            {
                let scale = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let shape = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(WeibullWrapper::new(scale, shape))
            }

            fn visit_map<V>(self, mut map: V) -> Result<WeibullWrapper, V::Error>
                where
                    V: MapAccess<'de>,
            {
                let mut scale = None;
                let mut shape = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Scale => {
                            if scale.is_some() {
                                return Err(de::Error::duplicate_field("scale"));
                            }
                            scale = Some(map.next_value()?);
                        }
                        Field::Shape => {
                            if shape.is_some() {
                                return Err(de::Error::duplicate_field("shape"));
                            }
                            shape = Some(map.next_value()?);
                        }
                    }
                }
                let scale = scale.ok_or_else(|| de::Error::missing_field("scale"))?;
                let shape = shape.ok_or_else(|| de::Error::missing_field("shape"))?;
                Ok(WeibullWrapper::new(scale, shape))
            }
        }

        const FIELDS: &'static [&'static str] = &["scale", "shape"];
        deserializer.deserialize_struct("WeibullWrapper", FIELDS, WeibullWrapperVisitor)
    }
}

impl<'de> Deserialize<'de> for LogNormalWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[allow(non_camel_case_types)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field { Mean, Std_Dev };

        struct LogNormalWrapperVisitor;

        impl<'de> Visitor<'de> for LogNormalWrapperVisitor {
            type Value = LogNormalWrapper;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct LogNormalWrapper")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<LogNormalWrapper, V::Error>
                where
                    V: SeqAccess<'de>,
            {
                let mean = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let std_dev = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(LogNormalWrapper::new(mean, std_dev))
            }

            fn visit_map<V>(self, mut map: V) -> Result<LogNormalWrapper, V::Error>
                where
                    V: MapAccess<'de>,
            {
                let mut mean = None;
                let mut std_dev = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Mean => {
                            if mean.is_some() {
                                return Err(de::Error::duplicate_field("mean"));
                            }
                            mean = Some(map.next_value()?);
                        }
                        Field::Std_Dev => {
                            if std_dev.is_some() {
                                return Err(de::Error::duplicate_field("std_dev"));
                            }
                            std_dev = Some(map.next_value()?);
                        }
                    }
                }
                let mean = mean.ok_or_else(|| de::Error::missing_field("mean"))?;
                let std_dev = std_dev.ok_or_else(|| de::Error::missing_field("std_dev"))?;
                Ok(LogNormalWrapper::new(mean, std_dev))
            }
        }

        const FIELDS: &'static [&'static str] = &["mean", "std_dev"];
        deserializer.deserialize_struct("LogNormalWrapper", FIELDS, LogNormalWrapperVisitor)
    }
}

#[cfg(test)]
mod test {
    use rand::prelude::*;

    use crate::simulation::wrappers::{NormalWrapper, UniformWrapper, WeibullWrapper, LogNormalWrapper, DistributionWrapper};
    
    use serde_yaml;

    #[test]
    fn test_normal_wrapper() {
        let distribution: NormalWrapper = Default::default();
        let mut rng = StdRng::seed_from_u64(0);
        let random_num = distribution.sample(&mut rng);

        let serialized = serde_yaml::to_string(&distribution).unwrap();

        let deserialized: NormalWrapper = serde_yaml::from_str(&serialized).unwrap();

        let mut rng2 = StdRng::seed_from_u64(0);
        let random_num2 = deserialized.sample(&mut rng2);
        assert_eq!(random_num, random_num2);
    }

    #[test]
    fn test_uniform_wrapper() {
        let distribution: UniformWrapper = Default::default();
        let mut rng = StdRng::seed_from_u64(0);
        let random_num = distribution.sample(&mut rng);

        let serialized = serde_yaml::to_string(&distribution).unwrap();

        let deserialized: UniformWrapper = serde_yaml::from_str(&serialized).unwrap();

        let mut rng2 = StdRng::seed_from_u64(0);
        let random_num2 = deserialized.sample(&mut rng2);
        assert_eq!(random_num, random_num2);
    }

    #[test]
    fn test_weibull_wrapper() {
        let distribution: WeibullWrapper = Default::default();
        let mut rng = StdRng::seed_from_u64(0);
        let random_num = distribution.sample(&mut rng);

        let serialized = serde_yaml::to_string(&distribution).unwrap();

        let deserialized: WeibullWrapper = serde_yaml::from_str(&serialized).unwrap();

        let mut rng2 = StdRng::seed_from_u64(0);
        let random_num2 = deserialized.sample(&mut rng2);
        assert_eq!(random_num, random_num2);
    }

    #[test]
    fn test_log_normal_wrapper() {
        let distribution: LogNormalWrapper = Default::default();
        let mut rng = StdRng::seed_from_u64(0);
        let random_num = distribution.sample(&mut rng);

        let serialized = serde_yaml::to_string(&distribution).unwrap();

        let deserialized: LogNormalWrapper = serde_yaml::from_str(&serialized).unwrap();

        let mut rng2 = StdRng::seed_from_u64(0);
        let random_num2 = deserialized.sample(&mut rng2);
        assert_eq!(random_num, random_num2);
    }
}
