use crypto::digest::Digest;
use crypto::sha2::Sha256;

/// Generating node id from node's remote address
pub(crate) fn generate_node_id(node_address: &str) -> u64 {
    let mut hasher = Sha256::new();
    hasher.input_str(node_address);

    let hash_prefix = &hasher.result_str()[..8];
    let mut buf: [u8; 8] = [0; 8];
    buf.copy_from_slice(hash_prefix.as_bytes());
    let id: u64 = u64::from_be_bytes(buf);

    id
}
