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
    model::config::{Config, TomlConfig},
    worker::{
        message::{Connect as WorkerConnect, Disconnect as WorkerDisconnect},
        Worker,
    },
};
use actix::{Actor, Context, Handler, MessageResult};
use getset::Getters;
use pudlib::{
    reload, ManagerSessionToServer, Schedules, ServerToManagerClient, ServerToWorkerClient,
    WorkerSessionToServer,
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use time::OffsetDateTime;
use tracing::{debug, error, info};
use typed_builder::TypedBuilder;
use uuid::Uuid;

/// `Server` coordinates workers and managers communication
#[derive(Clone, Debug, Getters, TypedBuilder)]
#[getset(get = "pub(crate)")]
pub(crate) struct Server {
    config: Config,
    #[builder(default = HashMap::new())]
    workers: HashMap<Uuid, Worker>,
    #[builder(default = HashMap::new())]
    managers: HashMap<Uuid, Manager>,
    #[builder(default = Arc::new(AtomicUsize::new(0)))]
    worker_count: Arc<AtomicUsize>,
    #[builder(default = Arc::new(AtomicUsize::new(0)))]
    manager_count: Arc<AtomicUsize>,
}

impl Server {
    /// Send message to everyone, except those in skip
    fn broadcast<T>(&self, message: T, skip_ids: &Option<Vec<Uuid>>)
    where
        T: Into<ServerToWorkerClient> + Into<ServerToManagerClient> + std::fmt::Debug + Clone,
    {
        debug!("broadcast message");
        let server_to_worker_client: ServerToWorkerClient = message.clone().into();
        let server_to_manager_client: ServerToManagerClient = message.into();
        self.broadcast_workers_message(&server_to_worker_client, skip_ids);
        self.broadcast_managers_message(&server_to_manager_client, skip_ids);
    }

    pub(crate) fn broadcast_workers_message(
        &self,
        message: &ServerToWorkerClient,
        skip_ids: &Option<Vec<Uuid>>,
    ) {
        debug!("broadcast message workers");
        for id in self.workers.keys() {
            let message_c = message.clone();
            if let Some(skip_ids) = &skip_ids {
                if !skip_ids.contains(id) {
                    self.direct_worker_message(message_c, id);
                }
            } else {
                self.direct_worker_message(message_c, id);
            }
        }
    }

    pub(crate) fn broadcast_managers_message(
        &self,
        message: &ServerToManagerClient,
        skip_ids: &Option<Vec<Uuid>>,
    ) {
        debug!("broadcast message managers");
        for id in self.managers.keys() {
            let message_c = message.clone();
            if let Some(skip_ids) = &skip_ids {
                if !skip_ids.contains(id) {
                    self.direct_manager_message(message_c, id);
                }
            } else {
                self.direct_manager_message(message_c, id);
            }
        }
    }

    pub(crate) fn direct_worker_message(&self, message: ServerToWorkerClient, id: &Uuid) {
        if let Some(worker) = self.workers.get(id) {
            worker.addr().do_send(message);
        } else {
            error!("cannont send message to worker: {}", id);
        }
    }

    pub(crate) fn direct_manager_message(&self, message: ServerToManagerClient, id: &Uuid) {
        if let Some(manager) = self.managers.get(id) {
            manager.addr().do_send(message);
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

    fn handle(&mut self, connect: WorkerConnect, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("handling connect message from worker");
        // register session with unique id
        let id = Uuid::new_v4();
        let worker = Worker::from(connect);
        let _b = self.workers.insert(id, worker);

        // broadcast new worker to all
        self.broadcast(format!("worker joined: {id}"), &Some(vec![id]));

        // broadcast worker count to all
        let count = self.worker_count.fetch_add(1, Ordering::SeqCst);
        self.broadcast(format!("total workers {}", count + 1), &None);

        // send id back
        MessageResult(id)
    }
}

// Handler for manager `Connect` message.
impl Handler<ManagerConnect> for Server {
    type Result = MessageResult<ManagerConnect>;

    fn handle(&mut self, connect: ManagerConnect, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("handling connect message from manager");
        // register session with unique id
        let id = Uuid::new_v4();
        let manager = Manager::from(connect);
        let _b = self.managers.insert(id, manager);

        // broadcast new worker to all
        self.broadcast(format!("manager joined: {id}"), &Some(vec![id]));

        // broadcast worker count to all
        let count = self.manager_count.fetch_add(1, Ordering::SeqCst);
        self.broadcast(format!("total managers {}", count + 1), &None);

        // send id back
        MessageResult(id)
    }
}

// Handler for worker `Disconnect` message.
impl Handler<WorkerDisconnect> for Server {
    type Result = ();

    fn handle(&mut self, msg: WorkerDisconnect, _ctx: &mut Context<Self>) {
        debug!("handling disconnect message from worker");
        // remove worker
        if self.workers.remove(&msg.id()).is_some() {
            // broadcast disconnect to all
            self.broadcast(format!("worker disconnected: {}", msg.id()), &None);

            // broadcast worker count to all
            let count = self.worker_count.fetch_sub(1, Ordering::SeqCst);
            self.broadcast(format!("total workers {}", count - 1), &None);
        }
    }
}

// Handler for manager `Disconnect` message.
impl Handler<ManagerDisconnect> for Server {
    type Result = ();

    fn handle(&mut self, msg: ManagerDisconnect, _ctx: &mut Context<Self>) {
        debug!("handling disconnect message from manager");
        // remove manager
        if self.managers.remove(&msg.id()).is_some() {
            // broadcast disconnect to all
            self.broadcast(format!("manager disconnected: {}", msg.id()), &None);

            // broadcast manager count to all
            let count = self.manager_count.fetch_sub(1, Ordering::SeqCst);
            self.broadcast(format!("total managers {}", count - 1), &None);
        }
    }
}

// Handler for message bound for a worker
impl Handler<WorkerSessionToServer> for Server {
    type Result = ();

    fn handle(&mut self, msg: WorkerSessionToServer, _ctx: &mut Context<Self>) {
        debug!("handling message from a worker session");
        match msg {
            WorkerSessionToServer::Initialize { id, name } => {
                let mut commands = self.config.default().clone();
                if let Some(overrides) = self.config.overrides().get(&name) {
                    for (name, cmd) in overrides {
                        let cmd_c = cmd.clone();
                        *commands.entry(name.clone()).or_insert_with(|| cmd.clone()) = cmd_c;
                    }
                }
                let mut schedules = self.config.schedules().clone();
                let schedule = schedules
                    .remove(&name)
                    .map(Schedules::take)
                    .unwrap_or_default();
                self.direct_worker_message(
                    ServerToWorkerClient::Initialize(commands, schedule),
                    &id,
                );
            }
            WorkerSessionToServer::Schedules {
                manager_id,
                name,
                schedules,
            } => {
                self.direct_manager_message(
                    ServerToManagerClient::Schedules { name, schedules },
                    &manager_id,
                );
            }
        }
    }
}

impl Handler<ManagerSessionToServer> for Server {
    type Result = ();

    fn handle(&mut self, msg: ManagerSessionToServer, _ctx: &mut Context<Self>) {
        debug!("handling message from a manager session");

        match msg {
            ManagerSessionToServer::Initialize { id, name: _ } => {
                self.direct_manager_message(ServerToManagerClient::Initialize, &id);
            }
            ManagerSessionToServer::Reload(id) => {
                let path = self.config.path();
                let quiet = self.config.quiet();
                let verbose = self.config.verbose();

                if let Ok(config) = reload::<TomlConfig, Config>(path.clone(), *quiet, *verbose) {
                    info!("server configuration reloaded");
                    self.config = config;
                }

                self.direct_manager_message(ServerToManagerClient::Reload(true), &id);
                self.broadcast_workers_message(&ServerToWorkerClient::Reload, &None);
            }
            ManagerSessionToServer::ListWorkers(id) => {
                let workers: HashMap<Uuid, (String, String)> = self
                    .workers
                    .iter()
                    .map(|(id, worker)| (*id, (worker.ip().clone(), worker.name().clone())))
                    .collect();
                self.direct_manager_message(ServerToManagerClient::WorkersList(workers), &id);
            }
            ManagerSessionToServer::Schedules { id, name } => {
                if let Some((worker_id, _worker)) =
                    self.workers.iter().find(|(_k, v)| *v.name() == name)
                {
                    self.direct_worker_message(ServerToWorkerClient::Schedules(id), worker_id);
                } else {
                    self.direct_manager_message(
                        ServerToManagerClient::Schedules {
                            name,
                            schedules: vec![],
                        },
                        &id,
                    );
                }
            }
            ManagerSessionToServer::Query { id, output } => {
                if output.is_empty() {
                    self.direct_manager_message(
                        ServerToManagerClient::QueryReturn {
                            stdout: vec![],
                            stderr: vec![],
                            status: 0,
                            start_time: OffsetDateTime::now_utc(),
                            end_time: OffsetDateTime::now_utc(),
                            done: true,
                        },
                        &id,
                    );
                } else {
                    let output_len = output.len();
                    for (idx, job_doc) in output.iter().enumerate() {
                        self.direct_manager_message(
                            ServerToManagerClient::QueryReturn {
                                stderr: job_doc.stderr().clone(),
                                stdout: job_doc.stdout().clone(),
                                status: *job_doc.status(),
                                start_time: *job_doc.start_time(),
                                end_time: *job_doc.end_time(),
                                done: idx == (output_len - 1),
                            },
                            &id,
                        );
                    }
                }
            }
        }
    }
}
