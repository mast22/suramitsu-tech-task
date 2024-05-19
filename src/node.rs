use std::sync::Mutex;

use crate::{Args, ConnectionResponse, LinkedSchema};

#[derive(Debug)]
pub struct Node {
    pub connected_nodes: Mutex<Vec<String>>,
    args: Args,
}

impl Clone for Node {
    fn clone(&self) -> Self {
        Node {
            connected_nodes: Mutex::new(self.connected_nodes.lock().unwrap().clone()),
            args: self.args.clone(),
        }
    }
}

impl Node {
    pub fn new(args: Args) -> Self {
        Self {
            connected_nodes: Mutex::new(vec![]),
            args,
        }
    }

    /// We need to propagate over the network
    /// We could have multiple ways to do that, like notifying neighbours,
    /// but in such a simple distributed system I would rather have a list of all
    /// node that I could connect with, yes, it has its downsides, but there's one big upside
    /// it's just simple
    ///
    /// We notify linked node about our IP address
    pub async fn connect(&self) {
        let connect_ip = match self.args.clone().connect {
            Some(connect_ip) => connect_ip,
            None => return,
        };

        // Send request to connect, to the first node
        let request_payload = LinkedSchema::Connection {
            address: format!("127.0.0.1:{}", self.args.port),
        };

        let url = format!("http://{connect_ip}/");
        let client = reqwest::Client::new();
        let res = client
            .post(url)
            .json(&request_payload.clone())
            .send()
            .await
            .expect(&format!(
                "Got an error trying to connect to a node, make sure node with port {} exist",
                self.args.port
            ));

        let response_connection = res
            .json::<ConnectionResponse>()
            .await
            .expect("Failed to get a response from linked node while connecting");

        // Store all nodes we've connected to,
        // the first connection node does not have it's own ip
        // So we need to add it separatelly
        let connected_nodes = {
            let mut connected_nodes = self
                .connected_nodes
                .lock()
                .expect("Failed to update a node list while connecting");
            connected_nodes.extend_from_slice(&response_connection.connected_nodes);
            connected_nodes.push(connect_ip.to_string());

            connected_nodes.clone()
        };

        // Connect to all nodes that first node has returned to us
        for ip in response_connection.connected_nodes.clone() {
            let client = client.clone();
            let payload = request_payload.clone();

            tokio::spawn(async move {
                client
                    .post(format!("http://{ip}/"))
                    .json(&payload)
                    .send()
                    .await
                    .expect("Failed to send message");
            });
        }

        println!("Connected to the peers at {:?}", connected_nodes);
    }

    /// Add new node to the connected nodes list and send him nodes it should
    /// establish a connection with as well.
    pub async fn process_connection(&self, address: String) -> ConnectionResponse {
        let mut connected_nodes = self
            .connected_nodes
            .lock()
            .expect("Failed to process a connection request");

        let response = ConnectionResponse {
            connected_nodes: connected_nodes.to_vec(),
        };

        connected_nodes.push(address);

        response
    }

    /// Just logging that we got a message
    pub async fn process_message(&self, message: String, addr: String) {
        println!("Received message \"{}\" from \"{}\"", message, addr);
    }
}
