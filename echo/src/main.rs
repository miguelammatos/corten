// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>.
// This file may not be copied, modified, or distributed
// except according to those terms.

extern crate serde;
#[macro_use] extern crate serde_derive;

use corten::simulation::{Process, SimulationKernel, ApplicationBase, Operation, ProcessId, Time, utils};

use std::any::Any;
use std::rc::Rc;
use std::cell::RefCell;

//holds the application configuration
//each struct has the derive below to automate debuging/printing and serializing the sim.
#[derive(Debug, Serialize, Deserialize)]
pub struct AppConf {
    pub n: ProcessId, //number of processes
    fanout: i8, //number of echos to send
    cycles: u16, //number of echo cycles to run
    period: Time //time between cycles
}

//hols the application configuration, read from the yaml file
impl AppConf {
    pub fn new(n: ProcessId, fanout: i8, cycles: u16, period: Time) -> AppConf {
        AppConf { n, fanout, cycles, period }
    }
}

//struct that holds the application state
#[derive(Debug, Serialize, Deserialize)]
pub struct EchoApplication {
    id: ProcessId,
    cycle: u16, //number of cycles executed
    nb_echos_sent: i32,
    nb_echos_received: i32,
    conf: Rc<AppConf>,
}

//constructor for the Echo Application
impl EchoApplication {
    pub fn new(id: ProcessId, cycle: u16, nb_echos_sent: i32, nb_echos_received: i32, conf: Rc<AppConf>) -> Self {
        EchoApplication { id, cycle, nb_echos_sent, nb_echos_received, conf }
    }
}

//ApplicationBase needs to be implemented for every application
//init is called to initialize the application
//recover is called when the application's process recovers from a failure
//derive serialization
#[typetag::serde]
impl ApplicationBase for EchoApplication {
    fn init(&mut self, process: Rc<RefCell<Process>>) {
        //we initialize the application by schedulling a new Cycle
        process.borrow().periodic(Box::new(Cycle {}), self.conf.period, self.conf.cycles);
    }
    fn leave(&mut self, _process: Rc<RefCell<Process>>) {}
    fn recover(&mut self, process: Rc<RefCell<Process>>) {
        if self.cycle < self.conf.cycles {
            let remaining_cycles = self.conf.cycles - self.cycle;
            process.borrow().periodic(Box::new(Cycle {}), self.conf.period, remaining_cycles);
        }
    }
    fn on_load(&mut self, _process: Rc<RefCell<Process>>, _apps: &Vec<Rc<RefCell<Box<dyn ApplicationBase>>>>) {}

    //boilerplate
    fn as_any(&self) -> &dyn Any {
        self
    }
    //boilerplate
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

//Cycle event, the EchoApplication will execute this periodically
//this can also have parameters, see Echo and EchoReply below
#[derive(Debug, Serialize, Deserialize)]
struct Cycle {}

//implement the logic for the Cycle event inside the invoke method
#[typetag::serde]
impl Operation for Cycle {
    fn invoke(&self, app_b: Rc<RefCell<Box<dyn ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        //boilerplate, access the EchoApplication state (struct)
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut EchoApplication = app_borrow.as_any_mut().downcast_mut::<EchoApplication>().unwrap();

        //if we still have cycle to go, run again
        if app.cycle < app.conf.cycles {
            //let time = process.borrow().get_time();
            //println!("Time {} Process {} Cycle", time, app.id);

            //select fanout targets to send Echo to
            for _ in 0..app.conf.fanout {
                //get_random return a random process in the system
                let target = (process.borrow().get_random() * app.conf.n as f64) as ProcessId;


                //in every Cycle, send an Echo message to other nodes
                process.borrow().send(Box::new( //this line is boilerplate,
                    //send Echo message with several parameters (encoded in Echo struct)
                    Echo { sender: app.id, target, msg: app.id as i32 * 100, }), target);

                app.nb_echos_sent += 1;
            }
            app.cycle += 1;
        }
    }
}

//Echo event/message
#[derive(Debug, Serialize, Deserialize)]
struct Echo {
    sender: ProcessId,
    target: ProcessId,
    msg: i32,
}

//implement handling of Echo message in the invoke
#[typetag::serde]
impl Operation for Echo {
    fn invoke(&self, app_b: Rc<RefCell<Box<dyn ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut EchoApplication = app_borrow.as_any_mut().downcast_mut::<EchoApplication>().unwrap();

        //println!("Time {} Process {} received echo from {}", process.borrow().get_time(), app.id, self.sender);

        app.nb_echos_received += 1;
        //reply to the sender with an EchoReply
        process.borrow().send(Box::new( //boilerplate
            EchoReply { sender: app.id, target: self.sender, nb_echoes: app.nb_echos_received }), self.sender);
    }
}

//EchoReply event/message
#[derive(Debug, Serialize, Deserialize)]
struct EchoReply {
    sender: ProcessId,
    target: ProcessId,
    nb_echoes:i32,
}

#[typetag::serde]
impl Operation for EchoReply {
    fn invoke(&self, _app_b: Rc<RefCell<Box<dyn ApplicationBase>>>, _process: Rc<RefCell<Process>>) {
        //nothing to do
        //let mut app_borrow = app_b.borrow_mut();
        //let app: &mut EchoApplication = app_borrow.as_any_mut().downcast_mut::<EchoApplication>().unwrap();
        //println!("Time {} Process {} received echo_reply from {} which got {} echoes", process.borrow().get_time(), app.id, self.sender, self.nb_echoes);
    }
}


//simulation finished, compute stats
pub fn stats(apps: &Vec<Rc<RefCell<Box<dyn ApplicationBase>>>>)  {
    println!("Gathering stats...");
    let mut echos_sent = Vec::with_capacity(apps.len());
    let mut echos_received = Vec::with_capacity(apps.len());
    let mut max_echos_received = 0;
    let mut min_echos_received =  i32::max_value();
    let mut total_echos_received = 0;

    //traverse all the application instances and gather stats
    for app in apps {
        let app_borrow = app.borrow();
        let a = match app_borrow.as_any().downcast_ref::<EchoApplication>() {
            Some(b) => b,
            None => panic!("not an Application"),
        };
        echos_sent.push(a.nb_echos_sent);
        echos_received.push(a.nb_echos_received);
        if max_echos_received < a.nb_echos_received {
            max_echos_received = a.nb_echos_received;
        }

        if min_echos_received > a.nb_echos_received {
            min_echos_received = a.nb_echos_received;
        }

        total_echos_received = total_echos_received + a.nb_echos_received;
    }
    println!("Echos sent: {:?}", echos_sent);
    println!("Echos received: {:?}", echos_received);
    println!("Echos received: min {} max {} total {}", min_echos_received, max_echos_received, total_echos_received);
    println!("DONE");

    //(echos_sent, echos_received, max_echos_received)
}


//Reads the configuration file and starts the simulation
fn main() {
    //simulation configuration
    let conf_filename = "config/conf-echo.yaml";
    //initialize app configuration
    let app_conf: Rc<AppConf> = Rc::new(utils::yaml_from_file_to_object(&conf_filename));

    //initialize all nodes
    let mut apps = Vec::new();
    for i in 0..app_conf.n {
        apps.push(Rc::new(RefCell::new(Box::new(
            EchoApplication::new(i, 0, 0, 0,
                                 app_conf.clone())) as Box<dyn ApplicationBase>)));
    }
    //run the simulation
    let kernel = SimulationKernel::init(&apps, conf_filename);

    //simulation finished, compute stats
    stats(&kernel.get_applications());
}