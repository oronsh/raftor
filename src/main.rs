use actix::prelude::*;
use actix_cors::Cors;
use actix_files as fs;
use actix_web::{
    http::header, middleware::Logger, web, App, Error, HttpRequest, HttpResponse, HttpServer,
    Responder,
    error
};
use actix_web_actors::ws;
use config;
use futures::Future;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use actix_raft::NodeId;
use evmap::ShallowCopy;

use raftor::{
    config::{ConfigSchema, NodeInfo},
    hash_ring,
    network::{GetNode, GetNodes, GetClusterState, Network},
    raftor::Raftor,
    server::{self, Server},
    session::Session,
    utils,
    raft::{RaftClient, ChangeRaftClusterConfig},
};

fn index_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Arc<ServerData>>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let uid = req.match_info().get("uid").unwrap_or("");

    srv.net
        .send(GetNode(uid.to_string()))
        .map_err(Error::from)
        .and_then(|res| Ok(HttpResponse::Ok().json(res)))
}

fn join_cluster_route(
    node_id: web::Json<NodeId>,
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Arc<ServerData>>,
) ->  HttpResponse {

    println!("got join request with id {:#?}", node_id);
    srv.raft.do_send(ChangeRaftClusterConfig(vec![*node_id], vec![]));
    HttpResponse::Ok().json(()) // <- send json response
}

fn members_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Arc<ServerData>>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let room_id = req.match_info().get("room_id").unwrap_or("");

    srv.server
        .send(server::GetMembers {
            room_id: room_id.to_owned(),
        })
        .map_err(Error::from)
        .and_then(|res| Ok(HttpResponse::Ok().json(res)))
}

fn nodes_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Arc<ServerData>>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    srv.net
        .send(GetNodes)
        .map_err(Error::from)
        .and_then(|res| Ok(HttpResponse::Ok().json(res)))
}

fn state_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Arc<ServerData>>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    srv.net
        .send(GetClusterState)
        .map_err(Error::from)
        .and_then(|res| Ok(HttpResponse::Ok().json(res)))
}

fn room_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Arc<ServerData>>,
) -> HttpResponse {
    let room_id = req.match_info().get("room_id").unwrap_or("");

    srv.server.do_send(server::CreateRoom {
        room_id: room_id.to_owned(),
    });
    HttpResponse::Ok().body("room created")
}

fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Arc<ServerData>>,
) -> Result<HttpResponse, Error> {
    let uid = req.match_info().get("uid").unwrap_or("");

    ws::start(Session::new(uid, "main", srv.server.clone()), &req, stream)
}

struct ServerData {
    server: Addr<Server>,
    net: Addr<Network>,
    raft: Addr<RaftClient>,
}

fn main() {
    env_logger::init();

    let sys = System::new("raftor");

    let args: Vec<String> = env::args().collect();
    let public_address = args[3].as_str();

    let mut raftor = Raftor::new();

    let state = Arc::new(ServerData {
        server: raftor.server.clone(),
        net: raftor.app_net.clone(),
        raft: raftor.raft.clone(),
    });

    let _ = raftor.start();

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::new()
                    .allowed_methods(vec!["GET", "POST", "PUT"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .max_age(3600),
            )
            .wrap(Logger::default())
            .data(state.clone())
            .service(web::resource("/").route(web::get().to(|| {
                HttpResponse::Found()
                    .header("LOCATION", "/static/index.html")
                    .finish()
            })))
            .service(web::resource("/node/{uid}").to_async(index_route))
            .service(web::resource("/cluster/nodes").to_async(nodes_route))
            .service(web::resource("/cluster/state").to_async(state_route))
            .service(web::resource("/cluster/join").route(web::put().to_async(join_cluster_route)))
            .service(web::resource("/room/{room_id}").to_async(room_route))
            .service(web::resource("/members/{room_id}").to_async(members_route))
            .service(web::resource("/ws/{uid}").to_async(ws_route))
            // static resources
            .service(fs::Files::new("/static/", "static/"))
    })
    .bind(public_address)
    .unwrap()
    .start();

    let _ = sys.run();
}
