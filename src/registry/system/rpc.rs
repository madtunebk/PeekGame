use super::super::Export;
use crate::api::system::rpc;

exports! {
    export!("rpcrt4.dll", "UuidFromStringA", "system", "implemented", "canonical 36-character UUID or NULL for nil UUID; invalid text returns RPC_S_INVALID_STRING_UUID", rpc::uuid_from_string_a),
    export!("rpcrt4.dll", "UuidCreate", "system", "implemented", "random RFC 4122 version 4 UUID", rpc::uuid_create),
}
