# Echo specification

This is a simple application to demonstrate Corten's features.

The EchoApplication has the following specification

* Cycle: period event, when invoked, the process will send an Echo message to *fanout* random process
* Echo: when receving an echo message, the receivers replies with an EchoReply message to the sender
* EchoReply: receives the message, no further processing is done

# Execution
To run the Echo application:

```
cargo run
```

The parameters of the environment and the application itself (number of cycles, fanout, number of processes) are specified in the
config/conf-echo.yaml file.

# Implementation
To implement the specification above, we need to:
* specify a struct for each event/message with the information it contains, for example

```
struct Echo {
    sender: ProcessId,
    target: ProcessId,
    msg: i32,
}
```

indicates the Echo message has a sender, a target, and the message contents itself.

* specify the application logic to be carried when an event is triggered. This is done by implementing the *Operation* trait. For example, when an Echo is received the following is invoked

```
impl Operation for Echo {
    fn invoke(&self, app_b: Rc<RefCell<Box<dyn ApplicationBase>>>, process: Rc<RefCell<Process>>) {
        //obtain the application state
        let mut app_borrow = app_b.borrow_mut();
        let app: &mut EchoApplication = app_borrow.as_any_mut().downcast_mut::<EchoApplication>().unwrap();

        //echo logic
        ....
    }
}
```

* the state of the Application is also encoded in a struct, for example

```
pub struct EchoApplication {
    id: ProcessId,
    cycle: u16, //number of cycles executed
    nb_echos_sent: i32,
    nb_echos_received: i32,
    conf: Rc<AppConf>,
}
```


* the application initializer is defined by implemeting the ApplicationBase trait, for example

```
impl ApplicationBase for EchoApplication {
    fn init(&mut self, process: Rc<RefCell<Process>>) {
        //we initialize the application by schedulling a new Cycle event
        process.borrow().periodic(Box::new(Cycle {}), self.conf.period, self.conf.cycles);
    }
```

* finaly, the main function reads the configuration file, initializes the simulator, runs the simulation, and collects the statistics.


## License

`echo` is distributed under the terms of the Apache License (Version 2.0).

See [LICENSE](LICENSE) and [COPYRIGHT](COPYRIGHT) for details.
