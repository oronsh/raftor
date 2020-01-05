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


use crate::{
    config::{ConfigSchema, NodeInfo},
    hash_ring,
    network::{GetNode, GetNodes, GetClusterState, Network},
    raftor::Raftor,
    server::{self, Server},
    session::Session,
    utils,
    raft::{RaftClient, ChangeRaftClusterConfig},
    channel::{Channel},
};

pub struct Http;

impl Http {
    fn new(state: Arc<ServerData>) {

    }
}
