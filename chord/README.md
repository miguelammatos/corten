# Corten implementation of the Chord DHT
For details on how to implement you applications, check the simpler *echo* application in the top-level Directory

To run Chord :

```
cargo run -- --conf <configuration file>
```

For example,

```
cargo run -- --conf config/conf-chord.yaml
```

For faster executions use the flag --release as follows:

```
cargo run --release -- --conf config/conf-chord.yaml
```

In the *config* directory there are several configurations under different scenarios.

## Authors
- Inês Sequeira
- Miguel Matos <miguel.marques.matos@tecnico.ulisboa.pt>

## License

`chord` is distributed under the terms of the Apache License (Version 2.0).

See [LICENSE](LICENSE) and [COPYRIGHT](COPYRIGHT) for details.
