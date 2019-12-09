// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>.
// This file may not be copied, modified, or distributed
// except according to those terms.

extern crate corten;

extern crate rand;
extern crate rand_xorshift;

use corten::simulation::SimulationKernel;
use corten::simulation::ApplicationBase;

use corten::simulation::utils;

use corten::echo_lib::*;

use std::rc::Rc;
use std::cell::RefCell;


fn main() {
    let conf_filename = "config/conf-main-end.yaml";

    let app_conf: Rc<AppConf> = Rc::new(utils::yaml_from_file_to_object(&conf_filename));

    let mut apps = Vec::new();
    for i in 0..app_conf.n {
        apps.push(Rc::new(RefCell::new(Box::new(Application::new(i, 0, 0, 0, app_conf.clone())) as Box<dyn ApplicationBase>)));
    }
    let kernel = SimulationKernel::init(&apps, conf_filename);

    stats(&kernel.get_applications());
}
