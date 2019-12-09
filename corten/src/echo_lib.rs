// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>.
// This file may not be copied, modified, or distributed
// except according to those terms.

use crate::simulation::Process;
use crate::simulation::ApplicationBase;
use crate::simulation::Operation;
use crate::simulation::ProcessId;
use crate::simulation::Time;

use std::any::Any;

use std::rc::Rc;
use std::cell::RefCell;

//holds the application configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct AppConf {
    pub n: ProcessId, //number of processes
    fanout: i8, //number of echos to send
    cycles: u16, //number of echo cycles to run
    period: Time //time between cycles
}

impl AppConf {
    pub fn new(n: ProcessId, fanout: i8, cycles: u16, period: Time) -> AppConf {
        AppConf { n, fanout, cycles, period }
    }
}

#[cfg_attr(feature = "checkpointing", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct Application {
    id: ProcessId,
    cycle : u16,
    nb_echos_sent: i32,
    nb_echos_received: i32,
    conf: Rc<AppConf>,
    pub executed: Vec<(Time, ProcessId, ProcessId)> //ts, sender, target
}

impl Application {
    pub fn new(id: ProcessId, cycle: u16, nb_echos_sent: i32, nb_echos_received: i32, conf: Rc<AppConf>) -> Self {
        Application { id, cycle, nb_echos_sent, nb_echos_received, conf, executed: Vec::new() }
    }
}

#[cfg_attr(feature = "checkpointing", typetag::serde)]
impl ApplicationBase for Application {
    fn init(&mut self, process: Rc<RefCell<Process>>) {
        //let time = process.borrow().get_time();
        //println!("Time {} Process {} Init", time, self.id);

        process.borrow().periodic(Box::new(Cycle {}), self.conf.period, self.conf.cycles);
    }
    fn leave(&mut self, _process: Rc<RefCell<Process>>) {}
    fn recover(&mut self, process: Rc<RefCell<Process>>) {
        if self.cycle < self.conf.cycles {
            let remaining_cycles = self.conf.cycles - self.cycle;
            process.borrow().periodic(Box::new(Cycle {}), 200, remaining_cycles);
        }
    }
    fn on_load(&mut self, _process: Rc<RefCell<Process>>, _apps: &Vec<Rc<RefCell<Box<dyn ApplicationBase>>>>) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg_attr(feature = "checkpointing", derive(Serialize, Deserialize))]
#[derive(Debug)]
struct Cycle {}

#[cfg_attr(feature = "checkpointing", typetag::serde)]
impl Operation for Cycle {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut Application = app_borrow.as_any_mut().downcast_mut::<Application>().unwrap();

        if app.cycle < app.conf.cycles {
            //let time = process.borrow().get_time();
            //println!("Time {} Process {} Cycle", time, app.id);

            //select fanout targets to send Echo to
            for _ in 0..app.conf.fanout {
                let target = (process.borrow().get_random() * app.conf.n as f64) as ProcessId;

                process.borrow().send(Box::new(Echo { sender: app.id, target, msg: app.id as i32 * 100, }), target);

                app.nb_echos_sent += 1;
            }
            app.cycle += 1;
        }
    }
}

#[cfg_attr(feature = "checkpointing", derive(Serialize, Deserialize))]
#[derive(Debug)]
struct Echo {
    sender: ProcessId,
    target: ProcessId,
    msg: i32,
}

#[cfg_attr(feature = "checkpointing", typetag::serde)]
impl Operation for Echo {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut Application = app_borrow.as_any_mut().downcast_mut::<Application>().unwrap();

        //println!("Time {} Process {} received echo from {}", process.borrow().get_time(), app.id, self.sender);

        app.executed.push((process.borrow().get_time(), self.sender, self.target));

        app.nb_echos_received += 1;
        process.borrow().send(Box::new(EchoReply { sender: app.id, target: self.sender, nb_echoes: app.nb_echos_received }), self.sender);
    }
}

#[cfg_attr(feature = "checkpointing", derive(Serialize, Deserialize))]
#[derive(Debug)]
struct EchoReply {
    sender: ProcessId,
    target: ProcessId,
    nb_echoes:i32,
}

#[cfg_attr(feature = "checkpointing", typetag::serde)]
impl Operation for EchoReply {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut Application = app_borrow.as_any_mut().downcast_mut::<Application>().unwrap();

        app.executed.push((process.borrow().get_time(), self.sender, self.target));

        //println!("Time {} Process {} received echo_reply from {} which got {} echoes", process.borrow().get_time(), app.id, self.sender, self.nb_echoes);
    }
}

pub fn stats(apps: &Vec<Rc<RefCell<Box<ApplicationBase>>>>) -> (Vec<i32>, Vec<i32>, i32) {
    println!("Gathering stats...");
    let mut echos_sent = Vec::with_capacity(apps.len());
    let mut echos_received = Vec::with_capacity(apps.len());
    let mut max_echos_received = 0;
    for app in apps {
        let app_borrow = app.borrow();
        let a = match app_borrow.as_any().downcast_ref::<Application>() {
            Some(b) => b,
            None => panic!("not an Application"),
        };
        echos_sent.push(a.nb_echos_sent);
        echos_received.push(a.nb_echos_received);
        if max_echos_received < a.nb_echos_received {
            max_echos_received = a.nb_echos_received;
        }
    }
    println!("Echos sent: {:?}", echos_sent);
    println!("Echos received: {:?}", echos_received);
    println!("Max echos received: {}", max_echos_received);
    println!("DONE");

    (echos_sent, echos_received, max_echos_received)
}
