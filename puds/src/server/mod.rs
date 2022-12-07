// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Server Actor

use crate::{
    manager::{
        message::{Connect as ManagerConnect, Disconnect as ManagerDisconnect},
        Manager,
    },
    worker::{
        message::{Connect as WorkerConnect, Disconnect as WorkerDisconnect},
        Worker,
    },
};
use actix::{Actor, Context, Handler, MessageResult};
use getset::Getters;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tracing::{debug, error};
use typed_builder::TypedBuilder;
use uuid::Uuid;

/// `Server` coordinates workers and managers communication
#[derive(Clone, Debug, Getters, TypedBuilder)]
#[getset(get = "pub(crate)")]
pub(crate) struct Server {
    // config: Config,
    workers: HashMap<Uuid, Worker>,
    managers: HashMap<Uuid, Manager>,
    worker_count: Arc<AtomicUsize>,
    manager_count: Arc<AtomicUsize>,
}

impl Server {
    /// Send message to everyone, except those in skip
    fn broadcast(&self, message: &str, skip_ids: &Option<Vec<Uuid>>) {
        debug!("broadcast message: {}", message);
        self.broadcast_workers_message(message, skip_ids);
        self.broadcast_managers_message(message, skip_ids);
    }

    pub(crate) fn broadcast_workers_message(&self, message: &str, skip_ids: &Option<Vec<Uuid>>) {
        debug!("broadcast message workers: {}", message);
        for id in self.workers.keys() {
            if let Some(skip_ids) = &skip_ids {
                if !skip_ids.contains(id) {
                    self.direct_worker_message(message, id);
                }
            } else {
                self.direct_worker_message(message, id);
            }
        }
    }

    pub(crate) fn broadcast_managers_message(&self, message: &str, skip_ids: &Option<Vec<Uuid>>) {
        debug!("broadcast message managers: {}", message);
        for id in self.managers.keys() {
            if let Some(skip_ids) = &skip_ids {
                if !skip_ids.contains(id) {
                    self.direct_manager_message(message, id);
                }
            } else {
                self.direct_manager_message(message, id);
            }
        }
    }

    pub(crate) fn direct_worker_message(&self, message: &str, id: &Uuid) {
        if let Some(worker) = self.workers.get(id) {
            let wm = pudlib::Worker::Text(message.to_string());
            worker.addr().do_send(wm);
        } else {
            error!("cannont send message to worker: {}", id);
        }
    }

    pub(crate) fn direct_manager_message(&self, message: &str, id: &Uuid) {
        if let Some(manager) = self.managers.get(id) {
            let mm = pudlib::Manager::Text(message.to_string());
            manager.addr().do_send(mm);
        } else {
            error!("cannont send message to manager: {}", id);
        }
    }
}
// `Server` is an `actix::Actor`
impl Actor for Server {
    type Context = Context<Self>;
}

// Handler for worker `Connect` message.
impl Handler<WorkerConnect> for Server {
    type Result = MessageResult<WorkerConnect>;

    fn handle(&mut self, connect: WorkerConnect, _: &mut Context<Self>) -> Self::Result {
        debug!("Connect message received.  Adding worker");
        // register session with unique id
        let id = Uuid::new_v4();
        let worker = Worker::from(connect);
        let _b = self.workers.insert(id, worker);

        // broadcast new worker to all
        self.broadcast(&format!("worker joined: {id}"), &Some(vec![id]));

        // broadcast worker count to all
        let count = self.worker_count.fetch_add(1, Ordering::SeqCst);
        self.broadcast(&format!("total workers {}", count + 1), &None);

        // send id back
        MessageResult(id)
    }
}

// Handler for manager `Connect` message.
impl Handler<ManagerConnect> for Server {
    type Result = MessageResult<ManagerConnect>;

    fn handle(&mut self, connect: ManagerConnect, _: &mut Context<Self>) -> Self::Result {
        debug!("Connect message received.  Adding manager");
        // register session with unique id
        let id = Uuid::new_v4();
        let manager = Manager::from(connect);
        let _b = self.managers.insert(id, manager);

        // broadcast new worker to all
        self.broadcast(&format!("manager joined: {id}"), &Some(vec![id]));

        // broadcast worker count to all
        let count = self.manager_count.fetch_add(1, Ordering::SeqCst);
        self.broadcast(&format!("total managers {}", count + 1), &None);

        // send id back
        MessageResult(id)
    }
}

// Handler for worker `Disconnect` message.
impl Handler<WorkerDisconnect> for Server {
    type Result = ();

    fn handle(&mut self, msg: WorkerDisconnect, _: &mut Context<Self>) {
        // remove worker
        if self.workers.remove(&msg.id()).is_some() {
            // broadcast disconnect to all
            self.broadcast(&format!("worker disconnected: {}", msg.id()), &None);

            // broadcast worker count to all
            let count = self.worker_count.fetch_sub(1, Ordering::SeqCst);
            self.broadcast(&format!("total workers {}", count - 1), &None);
        }
    }
}

// Handler for manager `Disconnect` message.
impl Handler<ManagerDisconnect> for Server {
    type Result = ();

    fn handle(&mut self, msg: ManagerDisconnect, _: &mut Context<Self>) {
        // remove manager
        if self.managers.remove(&msg.id()).is_some() {
            // broadcast disconnect to all
            self.broadcast(&format!("manager disconnected: {}", msg.id()), &None);

            // broadcast manager count to all
            let count = self.manager_count.fetch_sub(1, Ordering::SeqCst);
            self.broadcast(&format!("total managers {}", count - 1), &None);
        }
    }
}
