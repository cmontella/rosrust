pub use self::client::{Client, ClientResponse};
pub use self::error::Error;
pub use self::publisher::{Publisher, PublisherStream};
pub use self::service::Service;
pub use self::subscriber::SubscriberRosConnection;

use crate::rosmsg::RosMsg;
use crate::Clock;
use std::fmt::Debug;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

mod client;
pub mod error;
mod header;
mod publisher;
mod service;
mod subscriber;
mod util;

pub type ServiceResult<T> = Result<T, String>;

/// Sets the maximum accepted TCPROS topic-message body length.
///
/// The limit is checked immediately after reading the four-byte length prefix
/// and before allocating the message body. It applies process-wide to new
/// incoming topic messages. The default is unlimited for compatibility.
pub fn set_max_message_bytes(maximum_message_bytes: usize) {
    subscriber::set_max_message_bytes(maximum_message_bytes);
}

/// Sets the maximum accepted TCPROS connection-header length.
///
/// The limit is checked immediately after reading the four-byte length prefix
/// and before allocating the connection header. It applies process-wide to new
/// incoming connection headers. The default is unlimited for compatibility.
pub fn set_max_connection_header_bytes(maximum_header_bytes: usize) {
    header::set_max_header_bytes(maximum_header_bytes);
}

pub trait Message: Clone + Debug + Default + PartialEq + RosMsg + Send + Sync + 'static {
    fn msg_definition() -> String;
    fn md5sum() -> String;
    fn msg_type() -> String;
    fn set_header(&mut self, _clock: &Arc<dyn Clock>, _seq: &Arc<AtomicUsize>) {}
}

pub trait ServicePair: Clone + Debug + Default + PartialEq + Message {
    type Request: RosMsg + Send + 'static;
    type Response: RosMsg + Send + 'static;
}

#[derive(Clone, Debug)]
pub struct Topic {
    pub name: String,
    pub msg_type: String,
    pub md5sum: String,
}
