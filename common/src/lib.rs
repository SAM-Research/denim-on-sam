mod receiving_buffer;
mod sending_buffer;

include!("../denim_message_generated.rs");
include!(concat!(env!("OUT_DIR"), "/_includes.rs"));
