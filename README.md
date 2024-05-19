# Tech task for Suramitsu

Running instructions:

1. Clone the repository
2. Run the project

```bash
# Run this to start first node
cargo run -- --port 8080 --period 5
```

3. Connect other nodes using a different console

```bash
# Run this to connect more nodes into the network
cargo run -- --port 8081 --period 5 --connect 127.0.0.1:8080
```

4. Run previous command to lauch more nodes, increment port number manually
