use std::collections::{HashMap};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct NodeInfo {
    pub private_addr: String,
    pub public_addr: String,
}

pub type NodeList = Vec<NodeInfo>;

#[derive(Deserialize, Debug)]
pub struct ConfigSchema {
    pub nodes: NodeList,
}
