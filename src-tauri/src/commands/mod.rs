use std::sync::{Arc, Mutex};

pub struct AppState {
    pub git_op_pid: Arc<Mutex<Option<u32>>>,
}

mod branches;
mod commits;
mod config;
mod helpers;
mod remote;
mod scan;
mod staging;
mod stash;
mod status;
mod types;

pub use branches::*;
pub use commits::*;
pub use config::*;
pub use remote::*;
pub use scan::*;
pub use staging::*;
pub use stash::*;
pub use status::*;
pub use types::*;
