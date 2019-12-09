# Corten: large scale distributed algorithms simulator

Corten in a discrete event-based distributed algorithms simulator.
The main motivation behind Corten is to build a correct, complete and efficient simulator.

By correct, we mean that it prevents programmers from inadvertently introducing errors and unrealistic conditions in their simulations, such as atomic message exchanges or sharing objects among different processes, as one can inadvertently do in other tools.

By complete, we mean the ability to express different network, process and fault models, allowing to simulate applications in a wide range of scenarios.

By efficient, we mean memory and time efficient, allowing researchers to simulate systems with millions of processes in a laptop.

The main features of Corten are:
* network models - which allows to evaluate the application behaviour under different  for latency, jitter, packet loss and link asymmetry models
* process asynchrony - which allows to explore key interleavings in the application logic
* process churn - which allows to explore application behaviour under faults
* checkpointing - which allows snapshotting the state of the system and re-running from that point at a later point in time

As a starting point, we suggest you look at the *echo* directory which contains a simple Echo application that showcases Corten's approach.


## Directory structure

* corten - the simulator implementation
* echo - simple Echo application
* chord - an implementation of the Chord DHT

See each directory for further details.

## Authors

Corten's design is described in more detail in the Master thesis "Large Scale Distributed Algorithms Simulator" by Inês Sequeira, IST, U. Lisboa.

- Inês Sequeira
- Miguel Matos <miguel.marques.matos@tecnico.ulisboa.pt>

## License

`corten` is distributed under the terms of the Apache License (Version 2.0).

See [LICENSE](LICENSE) and [COPYRIGHT](COPYRIGHT) for details.
