// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>.
// This file may not be copied, modified, or distributed
// except according to those terms.

extern crate corten;

extern crate serde;
#[macro_use] extern crate serde_derive;

extern crate rand;
extern crate rand_xorshift;

extern crate num_bigint;
extern crate num_traits;

extern crate argparse;

use corten::simulation::Process;
use corten::simulation::ProcessId;
use corten::simulation::SimulationKernel;
use corten::simulation::ApplicationBase;
use corten::simulation::Operation;
use corten::simulation::utils;
use corten::simulation::Time;
use corten::simulation::Conf;

use std::any::Any;

use std::rc::Rc;
use std::cell::RefCell;

use rand::prelude::*;
use rand_xorshift::XorShiftRng;

use std::collections::HashSet;

use num_bigint::BigUint;
use num_bigint::RandBigInt;
use num_traits::pow;

use argparse::{ArgumentParser, Store};

type ChordId = BigUint;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct Ids {
    chord_id: ChordId,
    process_id: ProcessId
}

impl Ids {
    fn new(chord_id: ChordId, process_id: ProcessId) -> Self {
        Ids { chord_id, process_id }
    }
    fn get_chord_id(&self) -> &ChordId {
        &self.chord_id
    }
    fn get_process_id(&self) -> ProcessId {
        self.process_id
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ChordApp {
    id: ChordId,
    finger: Vec<Option<Ids>>,
    successor_list: Vec<Option<Ids>>,
    predecessor: Option<Ids>,
    next: u8,
    period: Time,
    period_check_predecessor: Time,
    count: u16,
    m: u8,
    successor_list_size: u8,
    stats: Rc<RefCell<LookupStats>>
}

#[derive(Debug, Serialize, Deserialize)]
struct LookupStats {
    fails: u32,
    lookups: u32,
    latencies: Vec<Time>
}

impl LookupStats {
    fn new() -> Self {
        LookupStats { fails: 0, lookups: 0, latencies: Vec::new() }
    }
}

impl ChordApp {
    pub fn new(id: ChordId, m: u8, successor_list_size: u8, period: Time, period_check_predecessor: Time, count: u16, stats: Rc<RefCell<LookupStats>>) -> Self {
        ChordApp { id, finger: vec![None; m as usize], successor_list: vec![None; successor_list_size as usize], predecessor: None, m, successor_list_size, next: 0, period, period_check_predecessor, count, stats }
    }

    pub fn new_from_conf(id: ChordId, conf: &ChordConf, stats: Rc<RefCell<LookupStats>>) -> Self {
        ChordApp::new(id, conf.m, conf.successor_list_size, conf.period, conf.period_check_predecessor, conf.count, stats)
    }

    fn schedule_periodic_calls(&self, process: Rc<RefCell<Process>>) {
        process.borrow().periodic(Box::new(Maintainer), self.period, self.count);
        process.borrow().periodic(Box::new(CheckPredecessor), self.period_check_predecessor, self.count);
    }

    fn set_successor(&mut self, successor: Option<Ids>, process: Rc<RefCell<Process>>) {
        self.successor_list[0 as usize] = successor.clone();
        self.finger[0 as usize] = successor.clone();

        if successor != None && self.successor_list_size != 1 {
            let sender = Ids::new(self.id.clone(), process.borrow().get_id());
            process.borrow().send(Box::new(SuccessorList { sender }), successor.unwrap().get_process_id());
        }
    }

    fn successor(&self) -> Option<Ids> {
        self.successor_list[0 as usize].clone()
    }

    fn get_full_list(&self) -> Vec<Ids> {
        let mut succ_list_no_options: Vec<Ids> = self.successor_list.iter().cloned()
            .filter(|x| *x != None)
            .map(|x| x.unwrap())
            .collect();
        let finger_no_options: Vec<Ids> = self.finger.iter().cloned()
            .filter(|x| *x != None)
            .map(|x| x.unwrap())
            .collect();
        succ_list_no_options.extend(finger_no_options);
        succ_list_no_options.sort_by(|a1, a2| {
            let dist1 = self.distance(&self.id, a1.get_chord_id());
            let dist2 = self.distance(&self.id, a2.get_chord_id());
            dist1.cmp(&dist2)
        });
        succ_list_no_options
    }

    /*fn gen_chord_id(m: u8, rng: &mut XorShiftRng) -> ChordId {
        let two: ChordId = From::from(2_u32);
        rng.gen_range(0, pow(two, m.into()))
    }*/

    fn gen_chord_id(m: u8, rng: &mut XorShiftRng) -> ChordId {
        rng.gen_biguint(m.into())
    }

    fn distance(&self, a: &ChordId, b: &ChordId) -> ChordId {
        if b >= a {
            b - a
        } else {
            let two: ChordId = From::from(2_u32);
            b + pow(two, self.m.into()) - a
        }
    }
}

fn between(id: &ChordId, a: &ChordId, b: &ChordId) -> bool {
    if a == b {
        return id != a
    } else if a < b {
        return id > a && id < b;
    } else { // b < a
        return id > a || id < b;
    }
}

fn between_right_inclusive(id: &ChordId, a: &ChordId, b: &ChordId) -> bool {
    return between(id, a, b) || (a != b && id == b);
}

#[typetag::serde]
impl ApplicationBase for ChordApp {
    fn init(&mut self, process: Rc<RefCell<Process>>) {
        let processes_up = process.borrow().get_global_view();

        if processes_up.len() == 0 {
            process.borrow().call(Box::new(Create), 0);
        } else {
            let my_process_id = process.borrow().get_id();
            let mut node_in_ring = my_process_id;
            let mut i = 0;
            while node_in_ring == my_process_id {
                let p = processes_up[i as usize].clone();
                node_in_ring = p.borrow().get_id();
                i += 0;
            }
            process.borrow().call(Box::new(Join { node_in_ring }), 0);
        }
    }

    fn leave(&mut self, process: Rc<RefCell<Process>>) {
        let successor = self.successor();
        let predecessor = self.predecessor.clone();
        if successor != None {
            process.borrow().send(Box::new(LeavingToSuccessor { leaving: self.id.clone(), predecessor: predecessor.clone() }), successor.clone().unwrap().get_process_id());
        }
        if predecessor != None {
            process.borrow().send(Box::new(LeavingToPredecessor { leaving: self.id.clone(), successor }), predecessor.unwrap().get_process_id());
        }
    }

    fn recover(&mut self, process: Rc<RefCell<Process>>) {
        // reset predecessor, successor_list, fingers and next
        self.predecessor = None;
        for item in &mut self.successor_list {
            *item = None;
        }
        for item in &mut self.finger {
            *item = None;
        }
        self.next = 0;

        self.init(process);
    }

    fn on_load(&mut self, process: Rc<RefCell<Process>>, apps: &Vec<Rc<RefCell<Box<dyn ApplicationBase>>>>) {
        let try_b = apps[0 as usize].try_borrow();
        if try_b.is_ok() {
            let app_borrow = try_b.unwrap();
            let app: &ChordApp = app_borrow.as_any().downcast_ref::<ChordApp>().unwrap();
            self.stats = app.stats.clone();
        }

        let lookups_to_make_per_app: u16 = 10;
        process.borrow().periodic(Box::new(RandomLookup), 1, lookups_to_make_per_app);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}


#[derive(Debug, Serialize, Deserialize)]
struct ChordConf {
    m: u8,
    #[serde(default = "default_successor_list_size")]
    successor_list_size: u8,
    n: ProcessId,
    #[serde(default = "default_period")]
    period: Time,
    #[serde(default = "default_period")]
    period_check_predecessor: Time,
    #[serde(default = "default_count")]
    count: u16
}

fn default_successor_list_size() -> u8 { 1 }
fn default_period() -> i32 { 200 }
fn default_count() -> u16 { 100 }


#[derive(Debug, Serialize, Deserialize)]
struct Create;

#[typetag::serde]
impl Operation for Create {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        //println!("Time {} Chord_id {} - Create", process.borrow().get_time(), app.id);

        app.predecessor = None;
        let successor = Some(Ids::new(app.id.clone(), process.borrow().get_id()));
        app.set_successor(successor, process.clone());

        app.schedule_periodic_calls(process);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Join {
    node_in_ring: ProcessId
}

#[typetag::serde]
impl Operation for Join {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        //println!("Time {} Chord_id {} - Join node_in_ring {}", process.borrow().get_time(), app.id, self.node_in_ring);

        app.predecessor = None;
        let sender = Ids::new(app.id.clone(), process.borrow().get_id());
        process.borrow().send(Box::new(FindSuccessor { id: app.id.clone(), response: FindSuccessorResponse::Join2, sender }), self.node_in_ring);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Join2 {
    successor: Option<Ids>
}

#[typetag::serde]
impl Operation for Join2 {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        //println!("Time {} Chord_id {} - Join2 succ {:?}", process.borrow().get_time(), app.id, self.successor);

        app.set_successor(self.successor.clone(), process.clone());

        app.schedule_periodic_calls(process);
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
enum FindSuccessorResponse {
    Join2,
    FixFingers2(u8),
    Lookup(Time)
}

#[derive(Debug, Serialize, Deserialize)]
struct Stabilize;

#[typetag::serde]
impl Operation for Stabilize {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        //println!("Time {} Chord_id {} - Stabilize1", process.borrow().get_time(), app.id);

        if app.successor() != None {
            let sender = Ids::new(app.id.clone(), process.borrow().get_id());
            process.borrow().send(Box::new(Predecessor { sender }), app.successor().unwrap().get_process_id());
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Predecessor {
    sender: Ids
}

#[typetag::serde]
impl Operation for Predecessor {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        //println!("Time {} Chord_id {} - Predecessor {:?} asked by {}", process.borrow().get_time(), app.id, app.predecessor, self.sender);

        process.borrow().send(Box::new(Stabilize2 { successor_predecessor: app.predecessor.clone() }), self.sender.get_process_id());
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Stabilize2 {
    successor_predecessor: Option<Ids>
}

#[typetag::serde]
impl Operation for Stabilize2 {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        //println!("Time {} Chord_id {} - Stabilize2, successor_predecessor {:?} where successor {:?}", process.borrow().get_time(), app.id, self.successor_predecessor, app.successor());

        if app.successor() != None {
            if self.successor_predecessor != None && between(self.successor_predecessor.as_ref().unwrap().get_chord_id(), &app.id, app.successor().unwrap().get_chord_id()) {
                app.set_successor(self.successor_predecessor.clone(), process.clone());
            } else {
                // to update successor list (because the successor list of the successor might have changed)
                app.set_successor(app.successor(), process.clone());
            }
            let target = app.successor().unwrap().get_process_id();
            process.borrow().send(Box::new(Notify { n_prime: Ids::new(app.id.clone(), process.borrow().get_id()) }), target);
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Notify {
    n_prime: Ids
}

#[typetag::serde]
impl Operation for Notify {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, _process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        //println!("Time {} Chord_id {} - Notify, possible predecessor {}, predecessor before {:?}", process.borrow().get_time(), app.id, self.n_prime, app.predecessor);

        if app.predecessor == None || between(&self.n_prime.get_chord_id(), app.predecessor.as_ref().unwrap().get_chord_id(), &app.id) {
            app.predecessor = Some(self.n_prime.clone());
        }

        //println!("Time {} Chord_id {} - Notify, predecessor after {:?}", process.borrow().get_time(), app.id, app.predecessor);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct FixFingers;

#[typetag::serde]
impl Operation for FixFingers {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        app.next += 1;
        if app.next >= app.m {
            app.next = 0;
        }

        let two: ChordId = From::from(2_u32);
        let id = (app.id.clone() + pow(two.clone(), app.next.into())) % pow(two, app.m.into());

        //println!("Time {} Chord_id {} - FixFingers, next {}, find_suc of {}", process.borrow().get_time(), app.id, app.next, id);

        let sender = Ids::new(app.id.clone(), process.borrow().get_id());
        process.borrow().call(Box::new(FindSuccessor { id, response: FindSuccessorResponse::FixFingers2(app.next), sender }), 0);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct FixFingers2 {
    next: u8,
    successor: Option<Ids>
}

#[typetag::serde]
impl Operation for FixFingers2 {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, _process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        //println!("Time {} Chord_id {} - FixFingers2, finger[{}]={:?}", _process.borrow().get_time(), app.id, self.next, self.successor);

        app.finger[self.next as usize] = self.successor.clone();
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CheckPredecessor;

#[typetag::serde]
impl Operation for CheckPredecessor {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        if app.predecessor != None {
            let process_id = app.predecessor.as_ref().unwrap().get_process_id();
            if !process.borrow().is_process_up(process_id) {
                app.predecessor = None;
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct FindSuccessor {
    id: ChordId,
    response: FindSuccessorResponse,
    sender: Ids
}

#[typetag::serde]
impl Operation for FindSuccessor {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        //println!("Time {} Chord_id {} whose successor is {:?} - FindSuccessor of {}; response {:?}, sender {:?}", process.borrow().get_time(), app.id, app.successor(), self.id, self.response, self.sender);

        if let FindSuccessorResponse::Lookup(_) = self.response {
            if self.id == app.id {
                let result = Some(Ids::new(app.id.clone(), process.borrow().get_id()));
                find_successor_reply_aux(&app.id, process.clone(), &self.id, result, self.response, &self.sender);
                return;
            }
        }

        if app.successor() != None && between_right_inclusive(&self.id, &app.id, app.successor().unwrap().get_chord_id()) {
            //println!("FindSuccessor case 1: successor {:?}", &app.successor());
            find_successor_reply_aux(&app.id,process.clone(), &self.id, app.successor(), self.response, &self.sender);
        } else {
            //println!("FindSuccessor case 2");
            process.borrow().call(Box::new(ClosestPrecedingNode { id: self.id.clone(), response: self.response, sender: self.sender.clone() }), 0);
        }
    }
}

fn find_successor_reply_aux(_app_id: &ChordId, process: Rc<RefCell<Process>>, id: &ChordId, successor: Option<Ids>, response: FindSuccessorResponse, sender: &Ids) {
    //println!("Time {} Chord_id {} - find_successor_reply_aux {:?}", process.borrow().get_time(), app_id, &response);

    match response {
        FindSuccessorResponse::Join2 => {
            let target = sender.get_process_id();
            process.borrow().send(Box::new(Join2 { successor }), target);
        },
        FindSuccessorResponse::FixFingers2(next) => {
            let target = sender.get_process_id();
            process.borrow().send(Box::new(FixFingers2 { next, successor }), target);
        },
        FindSuccessorResponse::Lookup(request_time) => {
            let target = sender.get_process_id();
            process.borrow().send(Box::new(LookupResponse { id: id.clone(), response: successor, request_time }), target);
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct FindSuccessor2 {
    id: ChordId,
    n_prime: Ids,
    response: FindSuccessorResponse,
    sender: Ids
}

#[typetag::serde]
impl Operation for FindSuccessor2 {
    fn invoke(&self, _app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        //let mut app_borrow = _app_b.borrow_mut();
        //let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        //println!("Time {} Chord_id {} - FindSuccessor2, call FindSuccessor of {}", process.borrow().get_time(), app.id, self.id);

        process.borrow().send(Box::new(FindSuccessor { id: self.id.clone(), response: self.response, sender: self.sender.clone() }), self.n_prime.get_process_id());
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ClosestPrecedingNode {
    id: ChordId,
    response: FindSuccessorResponse,
    sender: Ids
}

#[typetag::serde]
impl Operation for ClosestPrecedingNode {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        //println!("{:?}", app.finger);

        let mut n_prime = Ids::new(app.id.clone(), process.borrow().get_id());
        for id_item in app.get_full_list().iter().rev() {
            if between(&id_item.get_chord_id(), &app.id, &self.id) {
                n_prime = id_item.clone();
                break;
            }
        }

        if n_prime.get_chord_id() == &app.id {
            if let FindSuccessorResponse::Lookup(_) = self.response {
                return;
            }

            find_successor_reply_aux(&app.id,process.clone(), &self.id, Some(n_prime), self.response, &self.sender);
            return;
        }

        //println!("Time {}, ClosestPrecedingNode, Chord_id {}, returns {}", process.borrow().get_time(), app.id, n_prime);
        process.borrow().call(Box::new(FindSuccessor2 { id: self.id.clone(), n_prime, response: self.response, sender: self.sender.clone() }), 0);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SuccessorList {
    sender: Ids
}

#[typetag::serde]
impl Operation for SuccessorList {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        //println!("Time {}, SuccessorList, Chord_id {}", process.borrow().get_time(), app.id);

        let range = 0..(app.successor_list.len() - 1);
        let partial_successor_list = app.successor_list[range].iter().cloned().collect();
        process.borrow().send(Box::new(SuccessorListResponse { partial_successor_list }), self.sender.get_process_id());
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SuccessorListResponse {
    partial_successor_list: Vec<Option<Ids>>
}

#[typetag::serde]
impl Operation for SuccessorListResponse {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, _process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        //println!("Time {}, SuccessorListResponse, Chord_id {}", _process.borrow().get_time(), app.id);

        app.successor_list[1..].clone_from_slice(&self.partial_successor_list);
    }
}


#[derive(Debug, Serialize, Deserialize)]
struct Lookup {
    id: ChordId
}

#[typetag::serde]
impl Operation for Lookup {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        //println!("Time {} Chord_id {} - Lookup", process.borrow().get_time(), app.id);

        let sender = Ids::new(app.id.clone(), process.borrow().get_id());
        let current_ts = process.borrow().get_time();
        process.borrow().call(Box::new(FindSuccessor { id: self.id.clone(), response: FindSuccessorResponse::Lookup(current_ts), sender }), 0);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct RandomLookup;

#[typetag::serde]
impl Operation for RandomLookup {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        //println!("Time {} Chord_id {} - RandomLookup", process.borrow().get_time(), app.id);

        let id = ChordApp::gen_chord_id(app.m, &mut *process.borrow().get_rng().borrow_mut());
        app.stats.borrow_mut().lookups += 1;

        let sender = Ids::new(app.id.clone(), process.borrow().get_id());
        let current_ts = process.borrow().get_time();
        process.borrow().call(Box::new(FindSuccessor { id, response: FindSuccessorResponse::Lookup(current_ts), sender }), 0);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct LookupResponse {
    id: ChordId,
    response: Option<Ids>,
    request_time: Time
}

#[typetag::serde]
impl Operation for LookupResponse {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        //println!("Time {} Chord_id {} - LookupResponse", process.borrow().get_time(), app.id);

        if self.response != None {
            let latency = process.borrow().get_time() - self.request_time;
            app.stats.borrow_mut().latencies.push(latency);
        } else {
            app.stats.borrow_mut().fails += 1;
        }
    }
}


#[derive(Debug, Serialize, Deserialize)]
struct LeavingToPredecessor {
    leaving: ChordId,
    successor: Option<Ids>
}

#[typetag::serde]
impl Operation for LeavingToPredecessor {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        println!("Time {} Chord_id {} - LeavingToPredecessor, leaving {}, successor {:?}, old_successor {:?}", process.borrow().get_time(), app.id, self.leaving, self.successor, app.successor());

        let current_successor = app.successor();
        if self.successor != None {
            if current_successor == None
                || current_successor.as_ref().unwrap().get_chord_id() == &self.leaving
                || between_right_inclusive(self.successor.as_ref().unwrap().get_chord_id(), &app.id, current_successor.as_ref().unwrap().get_chord_id()) {
                app.set_successor(self.successor.clone(), process.clone());
            }
        } else {
            if app.successor_list_size == 1 {
                app.set_successor(None, process.clone());
            } else {
                for i in 1..app.successor_list.len() {
                    let succ_element = app.successor_list[i].clone();
                    if succ_element != None {
                        app.set_successor(succ_element, process.clone());
                        break;
                    }
                }
            }
        }

        println!("Chord_id {} new_successor {:?}", app.id, app.successor());
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct LeavingToSuccessor {
    leaving: ChordId,
    predecessor: Option<Ids>
}

#[typetag::serde]
impl Operation for LeavingToSuccessor {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();

        println!("Time {} Chord_id {} - LeavingToSuccessor, leaving {}, predecessor {:?}, old_predecessor {:?}", process.borrow().get_time(), app.id, self.leaving, self.predecessor, app.predecessor);

        if self.predecessor != None {
            if app.predecessor == None
                || app.predecessor.as_ref().unwrap().get_chord_id() == &self.leaving
                || (self.predecessor != None && between(self.predecessor.as_ref().unwrap().get_chord_id(), app.predecessor.as_ref().unwrap().get_chord_id(), &app.id)) {
                app.predecessor = self.predecessor.clone();
            }
        } else if app.predecessor != None && app.predecessor.as_ref().unwrap().get_chord_id() == &self.leaving {
            app.predecessor = None;
        }

        println!("Chord_id {} new_predecessor {:?}", app.id, app.predecessor);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Maintainer;

#[typetag::serde]
impl Operation for Maintainer {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        //{
        //    let mut app_borrow = app_b.borrow_mut();
        //    let app: &mut ChordApp = app_borrow.as_any_mut().downcast_mut::<ChordApp>().unwrap();
//
        //    //println!("Time {} Chord_id {} - Maintainer", process.borrow().get_time(), app.id);
        //}

        Stabilize.invoke(app_b.clone(), process.clone());
        FixFingers.invoke(app_b.clone(), process.clone());
    }
}


fn _save_app_info(apps: &Vec<Rc<RefCell<Box<dyn ApplicationBase>>>>, processes: &Vec<Rc<RefCell<Process>>>, conf: &Conf) {
    let mut res = "".to_string();
    for (i, app) in apps.iter().enumerate() {
        let app_borrow = app.borrow();
        let app: &ChordApp = app_borrow.as_any().downcast_ref::<ChordApp>().unwrap();

        res += &format!("\n{:?}, process_id: {}\npredecessor: {:?}\nsuccessor_list: {:?}\nfingers: {:?}\n", app.id, processes[i as usize].borrow().get_id(), app.predecessor, app.successor_list, app.finger);
    }

    utils::save_to_file(format!("ring-n{}.out", conf.n), res, false);
}

fn stats(apps: &Vec<Rc<RefCell<Box<dyn ApplicationBase>>>>, conf: &Conf) {
    println!("\nStats\n");
    let app_borrow = apps[0 as usize].borrow();
    let app: &ChordApp = app_borrow.as_any().downcast_ref::<ChordApp>().unwrap();

    println!("Lookups: {} Fails: {} Success: {}", app.stats.borrow().lookups, app.stats.borrow().fails, app.stats.borrow().latencies.len());

    if let Some(_) = conf.load {
        let filename = format!("latencies-load-n{}.dat", conf.n);
        let latencies = utils::vec_to_one_item_per_line(&app.stats.borrow().latencies);
        utils::save_to_file(filename, latencies, false);
    }
}


fn main() {
    let mut conf_filename: String = "".to_string(); 

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Chord simulation. ");
        ap.print_usage("cargo run -- --conf <config_filename>", &mut ::std::io::stderr()).unwrap();
        ap.refer(&mut conf_filename)
            .required()
            .add_option(&["--conf"], Store,
                        "Configuration filename");
        ap.parse_args_or_exit();
    }

    let chord_conf: ChordConf = utils::yaml_from_file_to_object(&conf_filename);

    let two: ChordId = From::from(2_u32);
    let num_processes: ChordId = From::from(chord_conf.n);
    let max_processes = pow(two, chord_conf.m.into());
    assert!(num_processes <= max_processes, "Number of processes (n) should be less than or equal to 2^m\nThe numbers provided were: n={n} and m={m} so {n} <= {max_proc} is not satisfied", n = chord_conf.n, m = chord_conf.m, max_proc = max_processes);

    let conf: Conf = utils::yaml_from_file_to_object(&conf_filename);

    let mut apps = Vec::new();
    if let None = &conf.load {
        let stats = Rc::new(RefCell::new(LookupStats::new()));

        let mut rng = XorShiftRng::seed_from_u64(0);
        let mut used_ids = HashSet::new();
        for _ in 0..chord_conf.n {
            let mut chord_id = ChordApp::gen_chord_id(chord_conf.m, &mut rng);
            while used_ids.contains(&chord_id) {
                chord_id = ChordApp::gen_chord_id(chord_conf.m, &mut rng);
            }
            used_ids.insert(chord_id.clone());
            let app: Box<ChordApp> = Box::new(ChordApp::new_from_conf(chord_id, &chord_conf, stats.clone()));
            apps.push(Rc::new(RefCell::new(app as Box<dyn ApplicationBase>)));
        }
    }

    let kernel = SimulationKernel::init(&apps, conf_filename);

    stats(&kernel.get_applications(), &conf);

    //_save_app_info(&kernel.get_applications(), &kernel.get_processes(), &conf);
}
