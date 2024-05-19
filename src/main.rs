mod node;

use clap::Parser;

use serde::{Deserialize, Serialize};
use std::time::Duration;
use ulid::Ulid;

use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};

use crate::node::Node;

/// Request schema for already connected nodes
#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
enum LinkedSchema {
    Connection { address: String },
    // We cannot get a port of node that sends a request, on the other node due to TCP restrictions
    // So, let's instead send an ip address together with port to identify ourselves
    Message { message: String, author: String },
}

/// Response for
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConnectionResponse {
    connected_nodes: Vec<String>,
}

async fn send_periodic_requests(args: Args, node: web::Data<Node>) {
    let mut interval = tokio::time::interval(Duration::from_secs(args.period as u64));
    let client = reqwest::Client::new();

    interval.tick().await; // Do not send message immediately, skip first one
    loop {
        interval.tick().await;

        let connected_nodes = {
            let connected_nodes = match node.connected_nodes.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };

            connected_nodes.clone()
        };

        let message = Ulid::new().to_string();

        for ip in connected_nodes.clone() {
            let client = client.clone();
            let payload = LinkedSchema::Message {
                message: message.clone(),
                author: format!("127.0.0.1:{}", args.port),
            };

            tokio::spawn(async move {
                let _ = client
                    .post(format!("http://{ip}/"))
                    .json(&payload)
                    .send()
                    .await;
            });
        }

        println!("Sending message \"{}\" to {:?}", message, connected_nodes);
    }
}

#[post("/")]
async fn index(req: String, node: web::Data<Node>) -> impl Responder {
    let request_payload: LinkedSchema = match serde_json::from_str(&req) {
        Ok(payload) => payload,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    return match request_payload {
        LinkedSchema::Connection { address } => {
            let response = &node.process_connection(address).await;

            HttpResponse::Ok().json(response)
        }
        LinkedSchema::Message { message, author } => {
            node.process_message(message, author).await;

            HttpResponse::Ok().finish()
        }
    };
}

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    port: String,

    #[arg(long)]
    connect: Option<String>,

    #[arg(long)]
    period: u16,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let port = args.port.parse::<u16>().expect("Undindable port");
    let node = web::Data::new(Node::new(args.clone()));

    println!("My address is \"127.0.0.1:{}\"", port);
    node.connect().await;

    // Setup periodic task to send messages
    let periodic_node = node.clone();
    tokio::spawn(async move {
        send_periodic_requests(args, periodic_node).await;
    });

    HttpServer::new(move || App::new().app_data(node.clone()).service(index))
        .bind(("127.0.0.1", port))?
        .run()
        .await
}
