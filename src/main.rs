use actix::prelude::*;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder,
                middleware::Logger, http::header};
use actix_cors::Cors;
use actix_web_actors::ws;
use actix_files as fs;
use std::env;
use futures::{Future};
use std::sync::Arc;
use config;
use std::collections::HashMap;
use serde::{Deserialize};

use raftor::{
    network::{Network, GetNode},
    server::{self, Server},
    session::Session,
    config::{ConfigSchema},
    hash_ring,
    utils,
};

fn index_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Arc<ServerData>>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let uid = req.match_info().get("uid").unwrap_or("");

    srv.net.send(GetNode(uid.to_string()))
        .map_err(Error::from)
        .and_then(|res| {
            Ok(HttpResponse::Ok().json(res))
        })
}

fn members_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Arc<ServerData>>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let room_id = req.match_info().get("room_id").unwrap_or("");

    srv.server.send(server::GetMembers{ room_id: room_id.to_owned() })
    .map_err(Error::from)
        .and_then(|res| {
            Ok(HttpResponse::Ok().json(res))
        })
}


fn room_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Arc<ServerData>>,
) -> HttpResponse {
    let room_id = req.match_info().get("room_id").unwrap_or("");

    srv.server.do_send(server::CreateRoom{ room_id: room_id.to_owned() });
    HttpResponse::Ok().body("room created")
}

fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Arc<ServerData>>
) -> Result<HttpResponse, Error> {
    let uid = req.match_info().get("uid").unwrap_or("");

    ws::start(
        Session::new(uid, "main", srv.server.clone()),
        &req,
        stream,
    )
}

struct ServerData {
    server: Addr<Server>,
    net: Addr<Network>,
}

fn main() {
    /*
    let mut config = config::Config::default();

    config
        .merge(config::File::with_name("Config")).unwrap()
        .merge(config::Environment::with_prefix("APP")).unwrap();

    let config = config.try_into::<ConfigSchema>().unwrap();

    let ring = hash_ring::Ring::new(10);

    let sys = System::new("testing");
    // let mut net = Network::new(ring.clone());

    let args: Vec<String> = env::args().collect();
    let local_address = args[1].as_str();
    let public_address = args[2].as_str();

    let node_id = utils::generate_node_id(local_address);

    // net.configure(config);
    // listen on ip and port
    // net.bind(local_address);

    // let net_addr = net.start();
    let server = Server::new(net_addr.clone(), ring.clone(), node_id).start();
    net_addr.do_send(SetServer(server.clone()));


    let state = Arc::new(ServerData{server: server.clone(), net: net_addr.clone()});

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::new()
                    .allowed_methods(vec!["GET", "POST"])
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
     */
}
