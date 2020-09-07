// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>.
// This file may not be copied, modified, or distributed
// except according to those terms.

extern crate binary_heap_plus;
#[cfg(feature = "checkpointing_on_ctrlc")] extern crate ctrlc;

pub mod network;
use self::network::Network;

pub mod asynchrony;
use self::asynchrony::Asynchrony;
use self::asynchrony::NoAsynchrony;

pub mod utils;

mod wrappers;

use self::binary_heap_plus::*;

use yaml_rust::YamlLoader;
use yaml_rust::yaml;

use std::fmt;
use std::fmt::{Debug, Display};
use std::cmp::Ordering;
use std::any::Any;

use std::rc::Rc;
use std::cell::RefCell;

use std::fs;
use std::path::Path;

use rand::prelude::*;
use rand_xorshift::XorShiftRng;

use serde::{Serialize, Deserialize};

use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;


static CHURN_OUTPUT_FILE: &str = "output/churn-plot/churn.dat";
static ASYNC_OUTPUT_FILE: &str = "output/async-plot/data/original/normal-async.dat";
static LATENCY_FILE: &str = "output/network-plot/latency-matrix.dat";

pub type ProcessId = u32;

#[derive(Debug, Serialize, Deserialize)]
pub struct Conf {
    pub n: ProcessId, //number of processes
    #[serde(default)] 
    pub network: Box<Network>,
    #[serde(default)] 
    pub asynchrony: Box<Asynchrony>,
    pub op_duration: Option<Time>,
    pub churn_file: Option<String>,
    #[serde(default = "default_seed")]
    pub seed: u64,
    pub save: Option<Time>,
    #[serde(default = "default_save_and_stop")]
    pub save_and_stop: bool,
    #[serde(default = "default_save_filename")]
    pub save_filename: String,
    pub load: Option<String>,
    pub new_seed: Option<u64>
}

fn default_seed() -> u64 { 0 }
fn default_save_and_stop() -> bool {
    false
}
fn default_save_filename() -> String {
    "state.bin".to_string()
}

//process structure, holds the process id plus application
#[cfg_attr(feature = "checkpointing", derive(Serialize, Deserialize))]
pub struct Process {
    id: ProcessId,
    current_ts: Rc<RefCell<Time>>,
    generation: u16,
    asynchrony: Rc<RefCell<Box<dyn Asynchrony>>>,
    op_duration: Time,
    network: Rc<RefCell<Box<dyn Network>>>,
    #[cfg_attr(feature = "checkpointing", serde(skip, default = "rng_default"))]
    rng: Rc<RefCell<XorShiftRng>>,
    #[cfg_attr(feature = "checkpointing", serde(skip, default = "default_queue"))]
    queue: Rc<RefCell<EventQueue>>,
    #[cfg_attr(feature = "checkpointing", serde(skip, default = "default_processes"))]
    processes: Rc<RefCell<Vec<ProcessState>>>,
    simulation_stops: bool
}

fn default_queue() -> Rc<RefCell<EventQueue>> {
    Rc::new(RefCell::new(EventQueue::default()))
}
fn default_processes() -> Rc<RefCell<Vec<ProcessState>>> {
    Rc::new(RefCell::new(Vec::new()))
}

impl Debug for Process {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Process {{ id: {:?}, current_ts: {:?}, generation: {:?}, asynchrony: {:?}, network: {:?} }}", self.id, self.current_ts, self.generation, self.asynchrony, self.network)
    }
}

#[cfg_attr(feature = "checkpointing", derive(Serialize, Deserialize))]
#[derive(Debug)]
struct ProcessState {
    up: bool,
    process: Rc<RefCell<Process>>
}

#[cfg_attr(feature = "checkpointing", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct SimulationKernel {
    pub apps : Vec<Rc<RefCell<Box<dyn ApplicationBase>>>>,
    processes : Rc<RefCell<Vec<ProcessState>>>,
    #[cfg_attr(all(feature = "checkpointing", not(feature = "heap_serde1")), serde(skip, default = "default_queue"))]
    queue: Rc<RefCell<EventQueue>>,
    current_ts: Rc<RefCell<Time>>,
    #[cfg_attr(all(feature = "checkpointing", not(feature = "rng_serde1")), serde(skip, default = "rng_default"))]
    rng: Rc<RefCell<XorShiftRng>>
}

#[cfg(any(feature = "checkpointing", not(feature = "rng_serde1")))]
fn rng_default() -> Rc<RefCell<XorShiftRng>> {
    Rc::new(RefCell::new(XorShiftRng::seed_from_u64(0)))
}

impl Default for SimulationKernel {
    fn default() -> Self {
        SimulationKernel { apps: Vec::new(), processes: Rc::new(RefCell::new(Vec::new())), queue: Rc::new(RefCell::new(EventQueue::default())), current_ts: Rc::new(RefCell::new(0)), rng: Rc::new(RefCell::new(XorShiftRng::seed_from_u64(0))) }
    }
}

#[cfg_attr(feature = "heap_serde1", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct EventQueue {
    heap : BinaryHeap<Event, MinComparator>,
}

impl EventQueue {
    fn new() -> EventQueue {
        EventQueue { heap: BinaryHeap::new_min() }
    }
    fn len(&self) -> usize {
        self.heap.len()
    }
    fn add_event(&mut self, event: Event) {
        self.heap.push(event);
    }
    fn add_events(&mut self, events: Vec<Event>) {
        self.heap.extend(events);
    }
    fn next_event(&mut self) -> Option<Event> {
        self.heap.pop()
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        EventQueue::new()
    }
}

pub type Time = i32;

// Event structure. Events have a timestamp, a target process and the operation to be invoked
#[cfg_attr(feature = "checkpointing", derive(Serialize, Deserialize))]
#[derive(Eq, Debug)]
pub struct Event {
    ts: Time,
    target: ProcessId,
    op: Box<Operation>,
    kind: EventKind
}

impl Event {
    fn new_churn(ts: Time, churn_kind: ChurnKind) -> Event {
        Event { ts, target: 0, op: Box::new(Kernel), kind: EventKind::Churn(churn_kind) }
    }
    fn new_save(ts: Time) -> Event {
        Event { ts, target: 0, op: Box::new(Kernel), kind: EventKind::Save }
    }
    fn new_end(ts: Time) -> Event {
        Event { ts, target: 0, op: Box::new(Kernel), kind: EventKind::Churn(ChurnKind::End) }
    }
}

#[cfg_attr(feature = "checkpointing", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Debug)]
enum EventKind {
    Local(u16, Time, u16), // (generation , delta, count)
    Message,
    Churn(ChurnKind),
    Save
}

#[cfg_attr(feature = "checkpointing", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Debug)]
enum ChurnKind {
    /// processes can join, i.e. become "alive"
    Join { num_proc: ProcessId },
    /// processes can leave gracefully
    /// (in which case the application can do something before they "die",
    /// for e.g. tell other processes that it will become unavailable),
    /// i.e. become "dead"
    Leave { num_proc: ProcessId },
    /// processes can fail
    /// (in which case they leave without warning),
    /// i.e. become "dead"
    Fail { num_proc: ProcessId },
    /// processes can rejoin, i.e. become "alive" again
    Recover { num_proc: ProcessId },
    /// equivalent to Leave but for a specific process
    LeaveId { id: ProcessId },
    /// equivalent to Fail but for a specific process
    FailId { id: ProcessId },
    /// equivalent to Recover but for a specific process
    RecoverId { id: ProcessId },
    /// to end the simulation
    End
}

impl ToString for EventKind {
    fn to_string(&self) -> String {
        match self {
            EventKind::Local(_, _, _) => {
                "local".to_string()
            },
            EventKind::Message => {
                "message".to_string()
            },
            EventKind::Churn(k) => {
                k.to_string()
            },
            EventKind::Save => {
                "save".to_string()
            }
        }
    }
}

impl ToString for ChurnKind {
    fn to_string(&self) -> String {
        match self {
            ChurnKind::Join { num_proc: _ } => {
            	"join".to_string()
            },
            ChurnKind::Leave { num_proc: _ } | ChurnKind::LeaveId { id: _ }  => {
            	"leave".to_string()
            },
            ChurnKind::Fail { num_proc: _ } | ChurnKind::FailId { id: _ }  => {
                "fail".to_string()
            },
            ChurnKind::Recover { num_proc: _ } | ChurnKind::RecoverId { id: _ } => {
            	"recover".to_string()
            }
            ChurnKind::End => {
                "end".to_string()
            }
        }
    }
}

impl ChurnKind {
    fn to_int(&self) -> i32 {
        match self {
            ChurnKind::Join { num_proc: _ } => {
                1
            },
            ChurnKind::Leave { num_proc: _ } | ChurnKind::LeaveId { id: _ }  => {
                -1
            },
            ChurnKind::Fail { num_proc: _ } | ChurnKind::FailId { id: _ }  => {
                -1
            },
            ChurnKind::Recover { num_proc: _ } | ChurnKind::RecoverId { id: _ } => {
                1
            },
            ChurnKind::End => {
                0
            }
        }
    }
}


impl Ord for Event {
    fn cmp(&self, other: &Event) -> Ordering {
        let res = self.ts.cmp(&other.ts);
        if res == Ordering::Equal {
            // we want events that are of kind "end" to be the last executed compared to events with the same timestamp
            if let EventKind::Churn(ChurnKind::End) = self.kind {
                return Ordering::Greater;
            } else if let EventKind::Churn(ChurnKind::End) = other.kind {
                return Ordering::Less;
            }
        }
        res
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Event) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Event) -> bool {
        self.ts == other.ts
    }
}

#[cfg_attr(feature = "checkpointing", typetag::serde(tag = "type"))]
pub trait Operation {
    fn invoke(&self, app_b: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>);
}

impl PartialEq for Operation {
    fn eq(&self, _other: &Operation) -> bool {
        true
    }
}

impl Eq for Operation {}

impl Debug for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Operation")
    }
}

#[cfg_attr(feature = "checkpointing", derive(Serialize, Deserialize))]
#[derive(Debug)]
struct Kernel;

#[cfg_attr(feature = "checkpointing", typetag::serde)]
impl Operation for Kernel {
    fn invoke(&self, _: Rc<RefCell<Box<ApplicationBase>>>, _: Rc<RefCell<Process>>) {}
}

#[cfg_attr(feature = "checkpointing", typetag::serde(tag = "type"))]
pub trait ApplicationBase: Debug {
    fn init(&mut self, _process: Rc<RefCell<Process>>);
    fn leave(&mut self, _process: Rc<RefCell<Process>>);
    fn recover(&mut self, _process: Rc<RefCell<Process>>);
    fn on_load(&mut self, _process: Rc<RefCell<Process>>, _apps: &Vec<Rc<RefCell<Box<dyn ApplicationBase>>>>);

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl PartialEq for ApplicationBase {
    fn eq(&self, _other: &ApplicationBase) -> bool {
        true
    }
}
impl Eq for ApplicationBase {}


impl Process {
    fn new(id: ProcessId, current_ts: Rc<RefCell<Time>>, op_duration: Time, asynchrony: Rc<RefCell<Box<Asynchrony>>>, network: Rc<RefCell<Box<Network>>>, rng: Rc<RefCell<XorShiftRng>>, queue: Rc<RefCell<EventQueue>>, processes: Rc<RefCell<Vec<ProcessState>>>) -> Rc<RefCell<Process>> {
        let process = Process { id, current_ts, generation: 0, asynchrony, network, rng, queue, processes, op_duration, simulation_stops: false };
        Rc::new(RefCell::new(process))
    }
    pub fn send(&self, op: Box<Operation>, target: u32) {
        match self.network.borrow_mut().get_latency(self.rng.clone(), self.id, target) {
            None => return,
            Some(latency) => {
                let ts = *self.current_ts.borrow() + latency;
                self.queue.borrow_mut().add_event(Event { ts, target, op, kind: EventKind::Message });
            }
        }
    }
    /// delta is the time after which the method should execute
    pub fn call(&self, op: Box<Operation>, delta: Time) {
        self.periodic(op, delta, 1);
    }
    /// delta is the time after which the method should execute
    /// count is the number of times to repeat the execution of the method
    pub fn periodic(&self, op: Box<Operation>, delta: Time, count: u16) {
        if !self.simulation_stops && count == 0 {
            eprintln!("\n!!! Warning !!! - You are scheduling an infinite periodic local call. \nIt is mandatory to exist a stop/end event for the simulation to stop.\nThis stop/event can be 'end' in churn, or save_and_stop in config if there is a save");
            ::std::process::exit(1);
        }

        let mut ts = *self.current_ts.borrow() + delta;
        ts = self.ts_with_async(ts);
        self.queue.borrow_mut().add_event(Event { ts, target: self.id, op, kind: EventKind::Local(self.generation, delta, count) });
    }
    fn receive(&self, event: Event, _conf: &Conf, app: Rc<RefCell<Box<ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        event.op.invoke(app, process);
        if let EventKind::Local(_, delta, count) = event.kind {
            self.reschedule_periodic(event.op, delta, count);
        }

        if cfg!(feature = "test_network") {
            if let EventKind::Message = event.kind {
                #[cfg(feature = "test_network")]
                self.log_time_received(event.ts);
            }
        }
    }
    pub fn get_time(&self) -> Time {
        *self.current_ts.borrow()
    }
    /// returns a random number in the range [0.0, 1.0) 
    pub fn get_random(&self) -> f64 {
        self.rng.borrow_mut().gen_range(0.0, 1.0)
    }
    pub fn get_rng(&self) -> Rc<RefCell<XorShiftRng>> {
        self.rng.clone()
    }
    pub fn get_id(&self) -> ProcessId {
        self.id
    }
    /// returns all the processes that are up
    pub fn get_global_view(&self) -> Vec<Rc<RefCell<Process>>> {
        let mut v = Vec::new();
        for state in &*self.processes.borrow() {
            if state.up {
                v.push(state.process.clone());
            }
        }
        v
    }
    pub fn is_process_up(&self, id: ProcessId) -> bool {
        self.processes.borrow()[id as usize].up
    }
    pub fn set_simulation_stops(&mut self, simulation_stops: bool) {
        self.simulation_stops = simulation_stops;
    }
    fn reschedule_periodic(&self, op: Box<Operation>, delta: Time, count: u16) {
        if count == 1 {
            return;
        }

        let new_count = if count == 0 {
            count
        } else {
            count - 1
        };

        let mut ts = *self.current_ts.borrow() + delta;
        ts = self.ts_with_async(ts);
        self.queue.borrow_mut().add_event(Event { ts, target: self.id, op, kind: EventKind::Local(self.generation, delta, new_count) });
    }
    fn ts_with_async(&self, ts: Time) -> Time {
        let asynchrony = self.asynchrony.borrow_mut().get_async(self.rng.clone(), self.op_duration);

        #[cfg(feature = "test_async")]
        self.log_async(asynchrony, self.id);

        ts + asynchrony
    }
    #[cfg(feature = "test_async")]
    fn log_async(&self, ts: Time, id: ProcessId) {
        let asynchrony = format!("{} {}", ts, id);

        utils::save_to_file(ASYNC_OUTPUT_FILE, asynchrony, true);
    }
    #[cfg(feature = "test_network")]
    fn log_time_received(&self, time: Time) {
        utils::save_to_file(LATENCY_FILE, time.to_string(), true);
    }
    fn get_generation(&self) -> u16 {
        self.generation
    }
    fn set_rng(&mut self, rng: Rc<RefCell<XorShiftRng>>) {
        self.rng = rng;
    }
    fn set_processes(&mut self, processes: Rc<RefCell<Vec<ProcessState>>>) {
        self.processes = processes;
    }
    fn set_current_ts(&mut self, current_ts: Rc<RefCell<Time>>) {
        self.current_ts = current_ts;
    }
}

impl SimulationKernel {
    pub fn new(conf: &Conf) -> Self {
        SimulationKernel {
            apps : Vec::with_capacity(conf.n as usize),
            processes : Rc::new(RefCell::new(Vec::with_capacity(conf.n as usize))),
            queue: Rc::new(RefCell::new(EventQueue::new())),
            current_ts: Rc::new(RefCell::new(0)),
            rng: Rc::new(RefCell::new(XorShiftRng::seed_from_u64(conf.seed)))
        }
    }
    fn get_op_duration(conf: &Conf) -> Time {
        let no_async = conf.asynchrony.as_any().downcast_ref::<NoAsynchrony>();

        if no_async.is_some() {
            0
        } else if conf.op_duration.is_none() {
            eprintln!("Error: when asynchrony is enabled it is mandatory to provide op_duration in configuration file. ");
            ::std::process::exit(-1)
        } else {
            conf.op_duration.unwrap()
        }
    }
    pub fn init<P: AsRef<Path> + Display>(apps: &Vec<Rc<RefCell<Box<dyn ApplicationBase>>>>, conf_filename: P) -> Self {
        let conf: Conf = utils::yaml_from_file_to_object(&conf_filename);

        let mut kernel;
        if let Some(_) = &conf.load {
            kernel = SimulationKernel::load_state(&conf);

            if let Some(seed) = conf.new_seed {
                println!("After load, running with new seed {}", seed);
            } else {
                println!("After load, running with same Random Number Generator");
            }
        } else {
            if conf.n as usize != apps.len() {
                eprintln!("Error: in SimulationKernel::init, must receive a Vec of applications with size equal to n={}, as it is specified in the configuration file {}, but received {}", conf.n, conf_filename, apps.len());
                ::std::process::exit(-1);
            }

            let op_duration = SimulationKernel::get_op_duration(&conf);

            kernel = SimulationKernel::new(&conf);

            let asynchrony = Rc::new(RefCell::new(conf.asynchrony.clone()));
            let network = Rc::new(RefCell::new(conf.network.clone()));
            //init process state
            for i in 0..conf.n {
                kernel.add_process(i as ProcessId, apps[i as usize].clone(), op_duration, asynchrony.clone(), network.clone());
            }

            println!("Running with seed {}", conf.seed);
        }

        let simulation_stops = kernel.config(&conf);

        kernel.update_process_simulation_stops(simulation_stops);

        kernel.run(&conf);

        kernel
    }
    fn update_kernel(kernel: &mut Self, conf: &Conf) {
        if kernel.processes.borrow().len() > 0 {
            let p0 = kernel.get_process(0).unwrap();
            let asynchrony = p0.borrow().asynchrony.clone();
            let network = p0.borrow().network.clone();

            if let Some(seed) = conf.new_seed {
                kernel.rng = Rc::new(RefCell::new(XorShiftRng::seed_from_u64(seed)));
            }

            for i in 0..kernel.processes.borrow().len() {
                let process = kernel.get_process(i as ProcessId).unwrap();
                let mut p = process.borrow_mut();
                p.queue = kernel.queue.clone();
                p.set_rng(kernel.rng.clone());
                p.set_processes(kernel.processes.clone());
                p.set_current_ts(kernel.current_ts.clone());
                if i != 0 {
                    p.asynchrony = asynchrony.clone();
                    p.network = network.clone();
                }
            }
        }
    }
    #[cfg(feature = "checkpointing")]
    pub fn load_state(conf: &Conf) -> Self {
        let load_filename = conf.load.as_ref().unwrap();
        println!("Loading state from {}", load_filename);

        let mut kernel: SimulationKernel = utils::binary_from_file_to_object(load_filename);
        SimulationKernel::update_kernel(&mut kernel, &conf);

        for i in 0..kernel.apps.len() {
            let mut app = kernel.apps[i].borrow_mut();
            let process = kernel.get_process(i as ProcessId).unwrap();
            app.on_load(process, &kernel.apps);
        }

        kernel
    }
    #[cfg(not(feature = "checkpointing"))]
    pub fn load_state(_conf: &Conf) -> Self {
        eprintln!("Unable to load state, due to the checkpointing feature being disabled. ");
        ::std::process::exit(-1)
    }
    pub fn update_process_simulation_stops(&self, simulation_stops: bool) {
        for p in &*self.processes.borrow() {
            p.process.borrow_mut().set_simulation_stops(simulation_stops);
        }
    }
    pub fn add_process(&mut self, id: ProcessId, app: Rc<RefCell<Box<dyn ApplicationBase>>>, op_duration: Time, asynchrony: Rc<RefCell<Box<Asynchrony>>>, network: Rc<RefCell<Box<Network>>>) {
        let process: Rc<RefCell<Process>> = Process::new(id, self.current_ts.clone(), op_duration, asynchrony, network, self.rng.clone(),self.queue.clone(), self.processes.clone());
        self.processes.borrow_mut().push(ProcessState { up: false, process });
        self.apps.push(app);
    }
    fn add_event(&mut self, event: Event) {
        self.queue.borrow_mut().add_event(event);
    }
    fn add_events(&mut self, events: Vec<Event>) {
        self.queue.borrow_mut().add_events(events);
    }
    fn next_event(&mut self) -> Option<Event> {
        self.queue.borrow_mut().next_event()
    }
    fn id_in_use(&self, id: ProcessId) -> bool {
        id < self.processes.borrow().len() as ProcessId
    }
    fn set_process_status(&mut self, id: ProcessId, status: bool) {
        if self.id_in_use(id) {
            self.processes.borrow_mut()[id as usize].up = status;
        }
    }
    fn is_process_up(&self, id: ProcessId) -> bool {
        if self.id_in_use(id) {
            self.processes.borrow()[id as usize].up
        } else {
            false
        }
    }
    fn get_process(&self, id: ProcessId) -> Option<Rc<RefCell<Process>>> {
        if self.id_in_use(id) {
            Some(self.processes.borrow()[id as usize].process.clone())
        } else {
            None
        }
    }
    fn get_application(&self, id: ProcessId) -> Option<Rc<RefCell<Box<ApplicationBase>>>> {
        if self.id_in_use(id) {
            Some(self.apps[id as usize].clone())
        } else {
            None
        }
    }
    fn get_processes_ids_up(&self) -> Vec<ProcessId> {
        let mut v = Vec::new();
        for process_state in &*self.processes.borrow() {
            let id = process_state.process.borrow().id;
            if self.is_process_up(id) {
                v.push(id);
            }
        }
        v
    }
    fn get_processes_ids_down(&self) -> Vec<ProcessId> {
        let mut v = Vec::new();
        for process_state in &*self.processes.borrow() {
            let id = process_state.process.borrow().id;
            if !self.is_process_up(id) {
                if self.has_process_joined(id) {
                    v.push(id);
                }
            }
        }
        v
    }
    fn get_processes_ids_not_joined(&self) -> Vec<ProcessId> {
        let mut v = Vec::new();
        for process_state in &*self.processes.borrow() {
            let id = process_state.process.borrow().id;
            if !self.is_process_up(id) {
                if !self.has_process_joined(id) {
                    v.push(id);
                }
            }
        }
        v
    }
    fn has_process_joined(&self, id: ProcessId) -> bool {
        match self.get_process(id) {
            Some(p) => {
                p.borrow().generation > 0
            },
            None => false
        }
    }
    pub fn get_processes_up(&self) -> Vec<Rc<RefCell<Process>>> {
        let mut v = Vec::new();
        for process_state in &*self.processes.borrow() {
            let p = process_state.process.clone();
            if self.is_process_up(p.borrow().id) {
                v.push(p);
            }
        }
        v
    }
    pub fn get_processes(&self) -> Vec<Rc<RefCell<Process>>> {
        let mut v = Vec::new();
        for process_state in &*self.processes.borrow() {
            v.push(process_state.process.clone());
        }
        v
    }
    pub fn get_applications(&self) -> &Vec<Rc<RefCell<Box<ApplicationBase>>>> {
        &self.apps
    }
    fn get_random_from_vec(&mut self, processes_ids: &mut Vec<ProcessId>, num_proc: ProcessId) -> Vec<ProcessId> {
        processes_ids.choose_multiple(&mut *self.rng.borrow_mut(), num_proc as usize).cloned().collect()
    }
    fn join_process(&mut self, ts: Time, id: ProcessId, _conf: &Conf) {
        if self.id_in_use(id) {
            let p = self.get_process(id).unwrap();
            {
                let mut process = p.borrow_mut();
                process.generation = 1;
                *process.current_ts.borrow_mut() = ts;
            }
            let app = self.get_application(id).unwrap();
            app.borrow_mut().init(p);

            self.set_process_status(id, true);
        }
    }
    fn leave_process(&mut self, _ts: Time, id: ProcessId) {
        let app = self.get_application(id).unwrap();
        let process = self.get_process(id).unwrap();
        app.borrow_mut().leave(process);

        self.set_process_status(id, false);
    }
    fn fail_process(&mut self, _ts: Time, id: ProcessId) {
        self.set_process_status(id, false);
    }
    fn recover_process(&mut self, ts: Time, id: ProcessId, _conf: &Conf) {
        if self.id_in_use(id) {
            let p = self.get_process(id).unwrap();
            {
                let mut process = p.borrow_mut();
                process.generation += 1;
                *process.current_ts.borrow_mut() = ts;
            }
            let app = self.get_application(id).unwrap();
            app.borrow_mut().recover(p);

            self.set_process_status(id, true);
        }
    }
    fn recover_processes(&mut self, ts: Time, conf: &Conf, num_proc: ProcessId) -> Vec<ProcessId> {
        let mut ids_down = self.get_processes_ids_down();
        let ids = self.get_random_from_vec(&mut ids_down, num_proc);
        for id in &ids {
            self.recover_process(ts, *id, conf);
        }
        ids
    }
    fn leave_processes(&mut self, ts: Time, num_proc: ProcessId) -> Vec<ProcessId> {
        let mut ids_up = self.get_processes_ids_up();
        let ids = self.get_random_from_vec(&mut ids_up, num_proc);
        for id in &ids {
            self.leave_process(ts, *id);
        }
        ids
    }
    fn fail_processes(&mut self, ts: Time, num_proc: ProcessId) -> Vec<ProcessId> {
        let mut ids_up = self.get_processes_ids_up();
        let ids = self.get_random_from_vec(&mut ids_up, num_proc);
        for id in &ids {
            self.fail_process(ts, *id);
        }
        ids
    }
    fn join_processes(&mut self, ts: Time, conf: &Conf, num_proc: ProcessId) -> Vec<ProcessId> {
        let mut ids_not_joined = self.get_processes_ids_not_joined();
        let ids = self.get_random_from_vec(&mut ids_not_joined, num_proc);
        for id in &ids {
            self.join_process(ts, *id, conf);
        }
        ids
    }
    fn add_join_event(&mut self, ts: Time, num_proc: ProcessId) {
        self.add_event(Event::new_churn(ts, ChurnKind::Join { num_proc }));
    }
    fn add_leave_event(&mut self, ts: Time, num_proc: ProcessId) {
        self.add_event(Event::new_churn(ts, ChurnKind::Leave { num_proc }));
    }
    fn add_fail_event(&mut self, ts: Time, num_proc: ProcessId) {
        self.add_event(Event::new_churn(ts, ChurnKind::Fail { num_proc }));
    }
    fn add_recover_event(&mut self, ts: Time, num_proc: ProcessId) {
        self.add_event(Event::new_churn(ts, ChurnKind::Recover { num_proc }));
    }
    fn add_leave_id_event(&mut self, ts: Time, id: ProcessId) {
        self.add_event(Event::new_churn(ts, ChurnKind::LeaveId { id }));
    }
    fn add_fail_id_event(&mut self, ts: Time, id: ProcessId) {
        self.add_event(Event::new_churn(ts, ChurnKind::FailId { id }));
    }
    fn add_recover_id_event(&mut self, ts: Time, id: ProcessId) {
        self.add_event(Event::new_churn(ts, ChurnKind::RecoverId { id }));
    }
    fn add_end_event(&mut self, ts: Time) {
        self.add_event(Event::new_end(ts));
    }
    fn add_save_event(&mut self, ts: Time) {
        self.add_event(Event::new_save(ts));
    }
    fn handle_churn_num_proc(&mut self, ts: Time, action: &str, num: &yaml::Yaml, n: ProcessId) {
        let num_int = num.as_i64();
        let num_proc = match num_int {
            Some(integer) => integer as u32,
            None => {
                let percent_proc = num.as_f64().expect("3rd param was not int, so it should be float but since you're seeing this it isn't a float either");
                (percent_proc * n as f64).round() as u32
            }
        };

        match action {
            "join" => { self.add_join_event(ts, num_proc); },
            "leave" => { self.add_leave_event(ts, num_proc); },
            "fail" => { self.add_fail_event(ts, num_proc); },
            "recover" => { self.add_recover_event(ts, num_proc); },
            _ => { println!("expected join, leave, or recover"); }
        }
    }
    fn handle_churn_id(&mut self, time: Time, action: &str, num: &yaml::Yaml) {
        let id = num.as_i64().expect("id must be an integer") as ProcessId;

        match action {
            "leave-id" => { self.add_leave_id_event(time, id); },
            "fail-id" => { self.add_fail_id_event(time, id); },
            "recover-id" => { self.add_recover_id_event(time, id); },
            _ => { println!("expected leave-id, or recover-id"); }
        }
    }
    fn no_churn_specified(&mut self, conf: &Conf) {
        if let None = &conf.load {
            let len = self.processes.borrow().len();
            for i in 0..len {
                self.set_process_status(i as ProcessId, true);
            }
        }
    }
    fn config_churn(&mut self, conf: &Conf) -> bool {
        let mut exists_end = false;
        match &conf.churn_file {
            None => {
                self.no_churn_specified(conf);
            },
            Some(filename) => {
                let s = fs::read_to_string(&filename).expect(&format!("Unable to open and read from file {}", filename));

                let contents = YamlLoader::load_from_str(&s).expect(&format!("File {} does not have yaml format", filename));
                if contents.len() == 0 {
                    self.no_churn_specified(conf);
                    return exists_end;
                }
                let content = &contents[0];

                match content["churn"].as_vec() {
                    None => {},
                    Some(v) => {
                        for item in v {
                            let i = item.as_vec().expect("expected a tuple containing time, action (join/leave/recover), and number of processes");
                            let time = i[0].as_i64().expect("time must be an integer") as Time;
                            let action = i[1].as_str().expect("action (join/leave/recover/leave-id/recover-id) must be a string");

                            match action {
                                "join" | "leave" | "recover" | "fail" => {
                                    self.handle_churn_num_proc(time, action, &i[2], conf.n);
                                },
                                "leave-id" | "recover-id" | "fail-id" => {
                                    self.handle_churn_id(time, action, &i[2]);
                                },
                                "end" => {
                                    self.add_end_event(time);
                                    exists_end = true;
                                },
                                _ => {
                                    eprintln!("error: in the churn configuration file the 2nd parameter must be one of the following join, leave, recover, leave-id, recover-id, end");
                                    ::std::process::exit(-1);
                                },
                            }
                        }
                    }
                }

            }
        }
        exists_end
    }
    fn config_save(&mut self, conf: &Conf) -> bool {
        if let Some(ts) = conf.save {
            self.add_save_event(ts);
        }
        conf.save_and_stop
    }
    pub fn config(&mut self, conf: &Conf) -> bool {
        let exists_end = self.config_churn(&conf);
        let exists_save_and_stop = self.config_save(conf);
        exists_end || exists_save_and_stop
    }
    #[cfg(feature = "test_churn")]
    fn log_churn(&self, ts: Time, kind: ChurnKind, ids: Vec<ProcessId>) {
        let churn = format!("{} {} {} {} {}", ts, kind.to_string(), kind.to_int(), ids.len(), utils::vec_to_str_no_whitespace(ids));

        utils::save_to_file(CHURN_OUTPUT_FILE, churn, true);
    }
    fn clean_files(&self) {
        #[cfg(feature = "test_churn")]
        //delete churn output file, because we want to start from scratch and not append to an existing file
        let _ = fs::remove_file(CHURN_OUTPUT_FILE);

        #[cfg(feature = "test_async")]
        //delete async output file, because we want to start from scratch and not append to an existing file
        let _ = fs::remove_file(ASYNC_OUTPUT_FILE);

        #[cfg(feature = "test_network")]
        //delete latency output file, because we want to start from scratch and not append to an existing file
        let _ = fs::remove_file(LATENCY_FILE);

    }
    fn handle_churn_event(&mut self, event: Event, conf: &Conf) {
        if let EventKind::Churn(c) = event.kind {
            match c {
                ChurnKind::Join { num_proc } => {
                    let _ids = self.join_processes(event.ts, conf, num_proc);
                    #[cfg(feature = "test_churn")]
                    self.log_churn(event.ts, c, _ids);
                },
                ChurnKind::Leave { num_proc } => {
                    let _ids = self.leave_processes(event.ts, num_proc);
                    #[cfg(feature = "test_churn")]
                    self.log_churn(event.ts, c, _ids);
                },
                ChurnKind::Fail { num_proc } => {
                    let _ids = self.fail_processes(event.ts, num_proc);
                    #[cfg(feature = "test_churn")]
                    self.log_churn(event.ts, c, _ids);
                },
                ChurnKind::Recover { num_proc } => {
                    let _ids = self.recover_processes(event.ts, conf, num_proc);
                    #[cfg(feature = "test_churn")]
                    self.log_churn(event.ts, c, _ids);
                },
                ChurnKind::LeaveId { id } => {
                    self.leave_process(event.ts, id);

                    #[cfg(feature = "test_churn")] {
                        let ids = vec![id];
                        self.log_churn(event.ts, c, ids);
                    }
                },
                ChurnKind::FailId { id } => {
                    self.fail_process(event.ts, id);
                    #[cfg(feature = "test_churn")] {
                        let ids = vec![id];
                        self.log_churn(event.ts, c, ids);
                    }
                },
                ChurnKind::RecoverId { id } => {
                    self.recover_process(event.ts, id, conf);
                    #[cfg(feature = "test_churn")] {
                        let ids = vec![id];
                        self.log_churn(event.ts, c, ids);
                    }
                },
                ChurnKind::End => {
                    // not supposed to reach this
                }
            };
        }
    }
    #[cfg(feature = "checkpointing")]
    fn handle_save_event<P: AsRef<Path> + Display>(&self, ts: Time, save_filename: P) {
        println!("Time {} saving snapshot in file {}", ts, save_filename);
        utils::save_object_in_binary_file(&self, save_filename);
    }
    #[cfg(not(feature = "checkpointing"))]
    fn handle_save_event<P: AsRef<Path> + Display>(&self, _ts: Time, _save_filename: P) {
        println!("Unable to save state, due to the checkpointing feature being disabled. ");
    }
    pub fn run(&mut self, conf: &Conf) {
        let ctrlc_received = Arc::new(AtomicBool::new(false));
        #[cfg(all(feature = "checkpointing_on_ctrlc", not(test)))]
        {
            let ctrlc_r = ctrlc_received.clone();

            ctrlc::set_handler(move || {
                if !ctrlc_r.load(atomic::Ordering::SeqCst) {
                    ctrlc_r.store(true, atomic::Ordering::SeqCst);
                    println!("\nReceived ctrl-c. A snapshot will be saved. \nIf you want to quit immediately without the snapshot click ctrl-c again. \n");
                } else {
                    println!("\nReceived 2nd ctrl-c. Exiting.\n");
                    ::std::process::exit(-1);
                }
            }).expect("Error setting Ctrl-C handler");
        }

        self.clean_files();

        //counts total events processed
        let mut events_processed: u32 = 0;
        //main simulation loop, run until event queue is empty
        loop {
            /*// periodically print the simulation progress
            if events_processed % 1000000 == 0 {
                println!("Events processed: {} Events remaining: {} ", events_processed, self.queue.borrow().len());
            }
            */
            //println!("\nEvents in queue {:?}\n", self.queue);

            if cfg!(all(feature = "checkpointing_on_ctrlc", not(test))) {
                let save = ctrlc_received.load(atomic::Ordering::SeqCst);
                if save {
                    let save_filename = "saved_on_exit.bin";
                    println!("Saving state in file {}", &save_filename);
                    self.handle_save_event(*self.current_ts.borrow(), &save_filename);
                    ::std::process::exit(-1);
                }
            }

            let event = self.next_event();
            match event {
                Some(event) => {
                    *self.current_ts.borrow_mut() = event.ts;

                    // periodically print the simulation progress
                    if events_processed % 1000000 == 0 {
                        println!("Time: {} Events processed: {} Events remaining: {}", event.ts, events_processed, self.queue.borrow().len());
                    }

                    match event.kind {
                        EventKind::Churn(ChurnKind::End) => {
                            println!("Reached end event at time {}", event.ts);
                            break;
                        },
                        EventKind::Churn(_) => {
                            self.handle_churn_event(event, conf);
                        },
                        EventKind::Save => {
                            self.handle_save_event(event.ts, &conf.save_filename);
                            if conf.save_and_stop {
                                break;
                            }
                        },
                        _ => {
                            //grab a ref to the targeted process
                            let p = self.get_process(event.target).unwrap();
                            let process = p.clone();


                            //skip events for failed processes
                            if !self.is_process_up(p.borrow().id) {
                                continue;
                            }
                            if let EventKind::Local(generation, _, _) = event.kind {
                                if p.borrow().get_generation() != generation {
                                    continue;
                                }
                            }

                            //println!("Event being processed: {:?}", event);

                            let app = self.get_application(event.target).unwrap();

                            p.borrow().receive(event, &conf, app, process);
                        }
                    }
                },
                None => break, //simulation finished
            }
            events_processed += 1;
        }

        println!("Time: {}. Total events processed: {}. Events still in event queue: {}", *self.current_ts.borrow(), events_processed, self.queue.borrow().len());
    }
}

#[allow(unused_imports)]
#[cfg(test)]
mod test {
    use std::rc::Rc;
    use std::cell::RefCell;
    use rand::prelude::*;
    use rand_xorshift::XorShiftRng;

    use serde::{Serialize};
    use serde::de::DeserializeOwned;
    use serde_yaml;

    use bincode::{serialize, deserialize};

    use std::fmt::{Display, Debug};
    use std::fs;

    use std::path::Path;

    use crate::simulation;
    use crate::simulation::Conf;
    use crate::simulation::Event;
    use crate::simulation::Time;
    use crate::simulation::Process;
    use crate::simulation::ProcessId;
    use crate::simulation::EventKind;
    use crate::simulation::ChurnKind;
    use crate::simulation::EventQueue;
    use crate::simulation::ApplicationBase;
    use crate::simulation::SimulationKernel;
    use crate::simulation::asynchrony::{Asynchrony, NoAsynchrony, UniformAsynchrony, NormalAsynchrony, WeibullAsynchrony};
    use crate::simulation::network::{self, ConstantNetwork, Network, NoJitter};
    use crate::simulation::utils;

    use crate::echo_lib::Application;
    use crate::echo_lib::AppConf;
    use crate::echo_lib::stats;

    #[cfg(feature = "test_network")]
    fn network_main<P: AsRef<Path> + Display>(conf_filename: P) {
        let app_conf: Rc<AppConf> = Rc::new(utils::yaml_from_file_to_object(&conf_filename));

        let mut apps = Vec::new();
        for i in 0..app_conf.n {
            apps.push(Rc::new(RefCell::new(Box::new(Application::new(i, 0, 0, 0, app_conf.clone())) as Box<dyn ApplicationBase>)));
        }
        let kernel = SimulationKernel::init(&apps, conf_filename);
    }

    #[test]
    #[cfg(feature = "test_network")]
    fn test_matrix_network() {
        assert_eq!(simulation::LATENCY_FILE, "output/network-plot/latency-matrix.dat");

        network_main("config/test/conf-matrix-network.yaml");
    }

    #[test]
    #[cfg(feature = "test_network")]
    fn test_constant_network() {
        assert_eq!(simulation::LATENCY_FILE, "output/network-plot/latency-constant.dat");

        network_main("config/test/conf-constant-network.yaml");
    }

    #[cfg(feature = "test_churn")]
    fn churn_main() {
        let conf_filename = "config/test/conf-churn.yaml";
        let app_conf: Rc<AppConf> = Rc::new(utils::yaml_from_file_to_object(&conf_filename));

        let mut apps = Vec::new();
        for i in 0..app_conf.n {
            apps.push(Rc::new(RefCell::new(Box::new(Application::new(i, 0, 0, 0, app_conf.clone())) as Box<dyn ApplicationBase>)));
        }
        let kernel = SimulationKernel::init(&apps, conf_filename);
    }

    #[test]
    #[cfg(feature = "test_churn")]
    fn test_churn() {
        churn_main();
    }

    #[cfg(feature = "test_async")]
    fn async_main(asynchrony: Rc<RefCell<Box<Asynchrony>>>) {
        let conf_filename = "config/test/conf-async.yaml";
        let conf: Conf = utils::yaml_from_file_to_object(&conf_filename);
        let app_conf: Rc<AppConf> = Rc::new(utils::yaml_from_file_to_object(&conf_filename));

        let op_duration = conf.op_duration.unwrap();

        let mut kernel: SimulationKernel = SimulationKernel::new(&conf);

        let network: Rc<RefCell<Box<Network>>> = Rc::new(RefCell::new(conf.network.clone()));
        for i in 0..conf.n {
            let app = Rc::new(RefCell::new(Box::new(Application::new(i, 0, 0, 0, app_conf.clone())) as Box<dyn ApplicationBase>));
            kernel.add_process(i, app, op_duration, asynchrony.clone(), network.clone());
        }

        kernel.config(&conf);

        kernel.run(&conf);
    }

    #[test]
    #[cfg(feature = "test_async")]
    fn test_no_async() {
        assert_eq!(simulation::ASYNC_OUTPUT_FILE, "output/async-plot/data/original/no-async.dat");

        async_main(Rc::new(RefCell::new(Box::new(NoAsynchrony::new()))));
    }

    #[test]
    #[cfg(feature = "test_async")]
    fn test_uniform_async() {
        assert_eq!(simulation::ASYNC_OUTPUT_FILE, "output/async-plot/data/original/uniform-async.dat");

        async_main(Rc::new(RefCell::new(Box::new(UniformAsynchrony::default()))));
    }

    #[test]
    #[cfg(feature = "test_async")]
    fn test_normal_async() {
        assert_eq!(simulation::ASYNC_OUTPUT_FILE, "output/async-plot/data/original/normal-async.dat");

        async_main(Rc::new(RefCell::new(Box::new(NormalAsynchrony::default()))));
    }

    #[test]
    #[cfg(feature = "test_async")]
    fn test_weibull_async() {
        assert_eq!(simulation::ASYNC_OUTPUT_FILE, "output/async-plot/data/original/weibull-async.dat");

        async_main(Rc::new(RefCell::new(Box::new(WeibullAsynchrony::default()))));
    }

    //////////////////////////////////////////////////////////////////////////////////////////////////

    fn serialize_des_yaml<T: Serialize + DeserializeOwned + Debug>(obj: &T) -> T {
        let serialized = serde_yaml::to_string(&obj).unwrap();
        println!("\n{}\n", serialized);

        let deserialized: T = serde_yaml::from_str(&serialized).unwrap();
        println!("{:?}", deserialized);

        deserialized
    }

    fn serialize_des_bin<T: Serialize + DeserializeOwned + Debug>(obj: &T) -> T {
        let encoded: Vec<u8> = serialize(&obj).unwrap();

        let decoded: T = deserialize(&encoded[..]).unwrap();
        println!("{:?}", decoded);

        decoded
    }

    #[test]
    #[cfg(feature = "checkpointing")]
    fn test_serde_process() {
        let conf: Conf = utils::yaml_from_file_to_object("config/conf.yaml");

        let id = 0;
        let asynchrony: Rc<RefCell<Box<Asynchrony>>> = Rc::new(RefCell::new(Box::new(NoAsynchrony::new())));
        let network: Rc<RefCell<Box<Network>>> = Rc::new(RefCell::new(conf.network.clone()));

        let queue = Rc::new(RefCell::new(EventQueue::new()));
        let rng = Rc::new(RefCell::new(XorShiftRng::seed_from_u64(0)));

        let process = Process::new(id, Rc::new(RefCell::new(0)), 0, asynchrony, network, rng, queue, Rc::new(RefCell::new(Vec::new())));

        serialize_des_yaml(&process);

        serialize_des_bin(&process);

        println!("{:?}", process);
    }

    #[test]
    #[cfg(feature = "heap_serde1")]
    fn test_serde_event_queue() {
        let mut queue = EventQueue::new();
        queue.add_event(Event::new_churn(0, ChurnKind::Join { num_proc: 8 }));
        queue.add_event(Event::new_churn(140, ChurnKind::LeaveId { id: 1 }));

        serialize_des_bin(&queue);
    }
    
    fn assert_same_executed(executed1: &Vec<(Time, ProcessId, ProcessId)>, executed2: &Vec<(Time, ProcessId, ProcessId)>) -> bool {
        if executed1.len() != executed2.len() {
            return false;
        }
        for i in 0..executed1.len() {
            if executed1[i] != executed2[i] {
                return false;
            }
        }
        return true;
    }
    
    fn assert_same_executed_for_apps(apps1: &Vec<Rc<RefCell<Box<dyn ApplicationBase>>>>, apps2: &Vec<Rc<RefCell<Box<dyn ApplicationBase>>>>) -> bool {
        if apps1.len() != apps2.len() {
            return false;
        }
        for i in 0..apps1.len() {
            let app_borrow1 = apps1[i].borrow();
            let app1 = app_borrow1.as_any().downcast_ref::<Application>().unwrap();
            let app_borrow2 = apps2[i].borrow();
            let app2 = app_borrow2.as_any().downcast_ref::<Application>().unwrap();

            if !assert_same_executed(&app1.executed, &app2.executed) {
                return false;
            }
        }
        return true;
    }

    #[cfg(feature = "checkpointing")]
    fn repeat_same_experiment(conf_filename: &str) {
        let app_conf: Rc<AppConf> = Rc::new(utils::yaml_from_file_to_object(&conf_filename));

        let mut apps = Vec::new();
        for i in 0..app_conf.n {
            apps.push(Rc::new(RefCell::new(Box::new(Application::new(i, 0, 0, 0, app_conf.clone())) as Box<dyn ApplicationBase>)));
        }
        let kernel = SimulationKernel::init(&apps, conf_filename);

        let stats_run1 = stats(&kernel.get_applications());


        let app_conf2: Rc<AppConf> = Rc::new(utils::yaml_from_file_to_object(&conf_filename));

        let mut apps2 = Vec::new();
        for i in 0..app_conf2.n {
            apps2.push(Rc::new(RefCell::new(Box::new(Application::new(i, 0, 0, 0, app_conf2.clone())) as Box<dyn ApplicationBase>)));
        }
        let kernel2 = SimulationKernel::init(&apps2, conf_filename);

        let stats_run2 = stats(&kernel2.get_applications());

        assert_eq!(stats_run1, stats_run2);
        assert!(assert_same_executed_for_apps(kernel.get_applications(), kernel2.get_applications()));
    }

    #[test]
    #[cfg(feature = "checkpointing")]
    fn test_repeat_same_experiment() {
        let conf_filename = "config/test/conf-save.yaml";

        repeat_same_experiment(conf_filename);
    }

    #[test]
    #[cfg(feature = "checkpointing")]
    fn test_repeat_same_experiment_matrix_network() {
        let conf_filename = "config/test/conf-save-matrix-network.yaml";

        repeat_same_experiment(conf_filename);
    }

    #[cfg(feature = "checkpointing")]
    fn checkpointing_same_rng(conf_filename: &str) {
        let app_conf: Rc<AppConf> = Rc::new(utils::yaml_from_file_to_object(&conf_filename));

        let mut apps = Vec::new();
        for i in 0..app_conf.n {
            apps.push(Rc::new(RefCell::new(Box::new(Application::new(i, 0, 0, 0, app_conf.clone())) as Box<dyn ApplicationBase>)));
        }
        let kernel = SimulationKernel::init(&apps, conf_filename);

        let stats_run1 = stats(&kernel.get_applications());


        let conf_filename2 = "config/test/conf-load.yaml";

        //let app_conf2: Rc<AppConf> = Rc::new(utils::yaml_from_file_to_object(&conf_filename2));

        let apps2 = Vec::new();
        let kernel2 = SimulationKernel::init(&apps2, conf_filename2);

        let stats_run2 = stats(&kernel2.get_applications());

        assert_eq!(stats_run1, stats_run2);
        assert!(assert_same_executed_for_apps(kernel.get_applications(), kernel2.get_applications()));
    }

    #[test]
    #[cfg(feature = "checkpointing")]
    fn test_checkpointing_same_rng() {
        let conf_filename = "config/test/conf-save.yaml";

        checkpointing_same_rng(conf_filename);
    }

    #[test]
    #[cfg(feature = "checkpointing")]
    fn test_checkpointing_same_rng_matrix_network() {
        let conf_filename = "config/test/conf-save-matrix-network.yaml";

        checkpointing_same_rng(conf_filename);
    }

    #[cfg(feature = "checkpointing")]
    fn checkpointing_different_rng(conf_filename: &str) {
        let app_conf: Rc<AppConf> = Rc::new(utils::yaml_from_file_to_object(&conf_filename));

        let mut apps = Vec::new();
        for i in 0..app_conf.n {
            apps.push(Rc::new(RefCell::new(Box::new(Application::new(i, 0, 0, 0, app_conf.clone())) as Box<dyn ApplicationBase>)));
        }
        let kernel = SimulationKernel::init(&apps, conf_filename);

        stats(&kernel.get_applications());


        let conf_filename2 = "config/test/conf-load-new_rng.yaml";

        //let app_conf2: Rc<AppConf> = Rc::new(utils::yaml_from_file_to_object(&conf_filename2));

        let apps2 = Vec::new();
        let kernel2 = SimulationKernel::init(&apps2, conf_filename2);

        stats(&kernel2.get_applications());

        assert!(!assert_same_executed_for_apps(kernel.get_applications(), kernel2.get_applications()));
    }

    #[test]
    #[cfg(feature = "checkpointing")]
    fn test_checkpointing_different_rng() {
        let conf_filename = "config/test/conf-save.yaml";

        checkpointing_different_rng(conf_filename);
    }

    #[test]
    #[cfg(feature = "checkpointing")]
    fn test_checkpointing_different_rng_matrix_network() {
        let conf_filename = "config/test/conf-save-matrix-network.yaml";

        checkpointing_different_rng(conf_filename);
    }

    #[cfg(feature = "checkpointing")]
    fn checkpointing_different_rng2(conf_filename: &str) {
        let conf: Conf = utils::yaml_from_file_to_object(&conf_filename);
        let app_conf: Rc<AppConf> = Rc::new(utils::yaml_from_file_to_object(&conf_filename));

        let mut apps = Vec::new();
        for i in 0..app_conf.n {
            apps.push(Rc::new(RefCell::new(Box::new(Application::new(i, 0, 0, 0, app_conf.clone())) as Box<dyn ApplicationBase>)));
        }
        let kernel = SimulationKernel::init(&apps, conf_filename);

        stats(&kernel.get_applications());


        let mut conf2: Conf = utils::yaml_from_file_to_object("config/test/conf-load-new_rng.yaml");
        conf2.new_seed = Some(4370871 as u64);
        assert_ne!(&conf2.load, &None);
        if let Some(load_filename) = &conf2.load {
            assert_eq!(&conf.save_filename, load_filename);
        }
        let mut kernel2 = SimulationKernel::load_state(&conf2);

        kernel2.run(&conf2);

        stats(&kernel2.get_applications());

        assert!(!assert_same_executed_for_apps(kernel.get_applications(), kernel2.get_applications()));
    }


    #[test]
    #[cfg(feature = "checkpointing")]
    fn test_checkpointing_different_rng2() {
        let conf_filename = "config/test/conf-save.yaml";

        checkpointing_different_rng2(conf_filename);
    }

    #[test]
    #[cfg(feature = "checkpointing")]
    fn test_checkpointing_different_rng2_matrix_network() {
        let conf_filename = "config/test/conf-save-matrix-network.yaml";

        checkpointing_different_rng2(conf_filename);
    }
}
