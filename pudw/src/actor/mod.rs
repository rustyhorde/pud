// Copyright (c) 2022 pud developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// The worker actix actor

use actix::{
    io::{SinkWrite, WriteHandler},
    Actor, ActorContext, AsyncContext, Context, Handler, SpawnHandle, StreamHandler, System,
};
use actix_codec::Framed;
use actix_http::ws::{CloseReason, Item};
use awc::{
    cookie::time::OffsetDateTime,
    error::WsProtocolError,
    ws::{Codec, Frame, Message},
    BoxedSocket,
};
use bincode::{deserialize, serialize};
use bytes::{Bytes, BytesMut};
use futures::stream::SplitSink;
use pudlib::{
    parse_calendar, parse_ts_ping, send_ts_ping, Command, Realtime, Schedule, ServerToWorkerClient,
    WorkerClientToWorkerSession,
};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    env,
    io::{BufRead, BufReader},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};
use typed_builder::TypedBuilder;
use uuid::Uuid;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(TypedBuilder)]
pub(crate) struct Worker {
    // current heartbeat instant
    #[builder(default = Instant::now())]
    hb: Instant,
    // The addr used to send messages back to the worker session
    addr: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
    // the sender for stdout from running commands
    tx_stdout: UnboundedSender<WorkerClientToWorkerSession>,
    // the sender for stderr from running commands
    tx_stderr: UnboundedSender<WorkerClientToWorkerSession>,
    // the sender for a command status
    tx_status: UnboundedSender<WorkerClientToWorkerSession>,
    // handle to the stdout queue future
    #[builder(default = Arc::new(Mutex::new(None)))]
    stdout_handle: Arc<Mutex<Option<SpawnHandle>>>,
    #[builder(default = VecDeque::new())]
    // the stdout queue
    stdout_queue: VecDeque<Vec<u8>>,
    // the last instant a queue message was drained
    #[builder(default = Instant::now())]
    stdout_last: Instant,
    // the last instant a stdout message was received
    #[builder(default = AtomicBool::new(false))]
    queue_running: AtomicBool,
    // continuation bytes
    #[builder(default = BytesMut::new())]
    cont_bytes: BytesMut,
    // The start instant of this session
    #[builder(default = Instant::now())]
    origin: Instant,
    // The commands loaded in this worker
    #[builder(default = BTreeMap::new())]
    commands: BTreeMap<String, Command>,
    // The schedules for the commands
    #[builder(default = Vec::new())]
    schedules: Vec<Schedule>,
    // The realtime schedules
    #[builder(default = HashMap::new())]
    rt: HashMap<Realtime, Vec<String>>,
    // Current futures handles
    #[builder(default = Vec::new())]
    fut_handles: Vec<SpawnHandle>,
    // Running condvar for stopping child process
    #[builder(default = Arc::new((Mutex::new(false), Condvar::new())))]
    running_pair: Arc<(Mutex<bool>, Condvar)>,
}

impl Worker {
    // Heartbeat that sends ping to the server every HEARTBEAT_INTERVAL seconds (5)
    // Also check for activity from the worker in the past CLIENT_TIMEOUT seconds (10)
    #[allow(clippy::unused_self)]
    fn hb(&self, ctx: &mut Context<Self>) {
        debug!("Starting worker session heartbeat");
        let origin_c = self.origin;
        let _ = ctx.run_interval(HEARTBEAT_INTERVAL, move |act, ctx| {
            debug!("checking heartbeat");
            // check heartbeat
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                error!("heartbeat timed out, disconnecting!");

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }
            debug!("sending heartbeat ping");
            let bytes = send_ts_ping(origin_c);
            if let Err(e) = act
                .addr
                .write(Message::Ping(Bytes::copy_from_slice(&bytes)))
            {
                error!("unable to send ping: {e:?}");
            }
        });
    }

    fn start_rt_monitor(&mut self, ctx: &mut Context<Self>) {
        info!("starting realtime schedule monitor");
        let rt_handle = ctx.run_interval(Duration::from_secs(1), move |act, _ctx| {
            let now = OffsetDateTime::now_utc();
            for (rt, cmds) in &act.rt {
                if rt.should_run(now) {
                    let cmds_thread = cmds.clone();
                    let commands_thread = act.commands.clone();
                    let tx_stdout_thread = act.tx_stdout.clone();
                    let tx_stderr_thread = act.tx_stderr.clone();
                    let tx_status_thread = act.tx_status.clone();
                    let running_pair_c = act.running_pair.clone();

                    // Run the long running commands in a separate thread
                    let _b = thread::spawn(move || {
                        // Run the commands sequentially
                        for cmd_name in &cmds_thread {
                            if let Some(cmd) = commands_thread.get(cmd_name) {
                                run_cmd(
                                    cmd_name,
                                    cmd.cmd(),
                                    &running_pair_c,
                                    &tx_stdout_thread,
                                    &tx_stderr_thread,
                                    &tx_status_thread,
                                );
                            }
                        }
                    });
                }
            }
        });
        self.fut_handles.push(rt_handle);
    }

    fn queue_monitor(&mut self, ctx: &mut Context<Self>) {
        let queue_handle = ctx.run_interval(Duration::from_secs(2), move |act, ctx| {
            if !act.stdout_queue.is_empty() && !act.queue_running.load(Ordering::SeqCst) {
                debug!("Starting stdout queue drain");
                act.queue_running.store(true, Ordering::SeqCst);
                let handle = Worker::start_stdout_drain(ctx);
                let mut sh_opt = match act.stdout_handle.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                *sh_opt = Some(handle);
            } else if act.stdout_queue.is_empty()
                && act.queue_running.load(Ordering::SeqCst)
                && Instant::now().duration_since(act.stdout_last) > Duration::from_secs(30)
            {
                let mut sh_opt = match act.stdout_handle.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                if let Some(sh) = *sh_opt {
                    debug!("Stopping stdout queue drain");
                    if !ctx.cancel_future(sh) {
                        error!("Unable to kill stdout_queue");
                    }
                    act.queue_running.store(false, Ordering::SeqCst);
                    *sh_opt = None;
                }
            }
        });
        self.fut_handles.push(queue_handle);
    }

    fn start_stdout_drain(ctx: &mut Context<Self>) -> SpawnHandle {
        ctx.run_interval(Duration::from_millis(1), |act, _ctx| {
            if let Some(front) = act.stdout_queue.pop_front() {
                act.stdout_last = Instant::now();
                if let Err(e) = act.addr.write(Message::Binary(Bytes::from(front))) {
                    error!("Unable to write command: {e:?}");
                }
            }
        })
    }

    #[allow(clippy::unused_self)]
    fn handle_text(&mut self, bytes: &Bytes) {
        debug!("handling text message");
        error!("invalid text received: {}", String::from_utf8_lossy(bytes));
    }

    fn handle_binary(&mut self, ctx: &mut Context<Self>, bytes: &Bytes) {
        if let Ok(msg) = deserialize::<ServerToWorkerClient>(bytes) {
            match msg {
                ServerToWorkerClient::Status(status) => info!("Status: {status}"),
                ServerToWorkerClient::Initialize(commands, schedules) => {
                    self.commands = commands;
                    self.schedules = schedules;
                    info!("worker loaded {} commands", self.commands.len());
                    info!("worker loaded {} schedules", self.schedules.len());
                    info!("worker initialization complete");
                    self.start_schedules(ctx);
                    // initialize the condvar pair
                    let (lock, _cvar) = &*self.running_pair;
                    let mut running = match lock.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    *running = true;
                }
                ServerToWorkerClient::Reload => {
                    info!("a reload has been requested, sending initialization");
                    self.stop_schedules(ctx);
                    // request initialization from the server
                    self.initialize(ctx);
                }
            }
        }
    }

    fn handle_ping(&mut self, bytes: Bytes) {
        debug!("handling ping message");
        if let Some(dur) = parse_ts_ping(&bytes) {
            debug!("ping duration: {}s", dur.as_secs_f64());
        }
        self.hb = Instant::now();
        if let Err(e) = self.addr.write(Message::Pong(bytes)) {
            error!("unable to send pong: {e:?}");
        }
    }

    fn handle_pong(&mut self, bytes: &Bytes) {
        debug!("handling pong message");
        if let Some(dur) = parse_ts_ping(bytes) {
            debug!("pong duration: {}s", dur.as_secs_f64());
        }
        self.hb = Instant::now();
    }

    #[allow(clippy::unused_self)]
    fn handle_close(&mut self, ctx: &mut Context<Self>, reason: Option<CloseReason>) {
        debug!("handling close message");
        if let Some(reason) = reason {
            info!("close reason: {reason:?}");
        }
        ctx.stop();
    }

    fn handle_continuation(&mut self, ctx: &mut Context<Self>, item: Item) {
        debug!("handling continuation message");
        match item {
            Item::FirstText(_bytes) => error!("unexpected text continuation"),
            Item::FirstBinary(bytes) | Item::Continue(bytes) => {
                self.cont_bytes.extend_from_slice(&bytes);
            }
            Item::Last(bytes) => {
                self.cont_bytes.extend_from_slice(&bytes);
                self.handle_binary(ctx, &bytes);
                self.cont_bytes.clear();
            }
        }
    }

    fn stop_schedules(&mut self, ctx: &mut Context<Self>) {
        let (lock, cvar) = &*self.running_pair;
        let mut running = match lock.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *running = false;
        cvar.notify_all();

        while let Some(handle) = self.fut_handles.pop() {
            if ctx.cancel_future(handle) {
                debug!("future cancelled successfully");
            }
        }
        self.rt.clear();
    }

    fn start_schedules(&mut self, ctx: &mut Context<Self>) {
        let schedules_c = self.schedules.clone();
        let mut has_realtime = false;

        for schedule in &schedules_c {
            match schedule {
                Schedule::Monotonic {
                    on_boot_sec,
                    on_unit_active_sec,
                    cmds,
                } => self.launch_monotonic(ctx, *on_boot_sec, *on_unit_active_sec, cmds),
                Schedule::Realtime {
                    on_calendar,
                    persistent,
                    cmds,
                } => {
                    has_realtime = true;
                    self.store_realtime(on_calendar, *persistent, cmds);
                }
            }
        }

        if has_realtime {
            self.start_rt_monitor(ctx);
        }
    }

    fn launch_monotonic(
        &mut self,
        ctx: &mut Context<Worker>,
        on_boot_sec: Duration,
        on_unit_active_sec: Duration,
        cmds: &[String],
    ) {
        info!(
            "launching monotonic schedule in {}s, re-running every {}s",
            on_boot_sec.as_secs_f64(),
            on_unit_active_sec.as_secs_f64(),
        );
        // clone everything to move into the initial run later future
        let cmds_later = cmds.to_owned();
        let commands_later = self.commands.clone();
        let tx_stdout_later = self.tx_stdout.clone();
        let tx_stderr_later = self.tx_stderr.clone();
        let tx_status_later = self.tx_status.clone();
        let running_pair_later = self.running_pair.clone();

        let later_handle = ctx.run_later(on_boot_sec, move |act, ctx| {
            // clone everything to move into the interval future
            let cmds_interval = cmds_later.clone();
            let commands_interval = commands_later.clone();

            let mono_handle = ctx.run_interval(on_unit_active_sec, move |act, _ctx| {
                // clone everything to move into the command thread
                let cmds_thread = cmds_interval.clone();
                let commands_thread = commands_interval.clone();
                let tx_stdout_thread = act.tx_stdout.clone();
                let tx_stderr_thread = act.tx_stderr.clone();
                let tx_status_thread = act.tx_status.clone();
                let running_pair_c = act.running_pair.clone();

                // Run the long running commands in a separate thread
                let _b = thread::spawn(move || {
                    // Run the commands sequentially
                    for cmd_name in &cmds_thread {
                        if let Some(cmd) = commands_thread.get(cmd_name) {
                            run_cmd(
                                cmd_name,
                                cmd.cmd(),
                                &running_pair_c,
                                &tx_stdout_thread,
                                &tx_stderr_thread,
                                &tx_status_thread,
                            );
                        }
                    }
                });
            });

            act.fut_handles.push(mono_handle);

            // Run the long running commands in a separate thread
            let _b = thread::spawn(move || {
                // Run the commands sequentially
                for cmd_name in &cmds_later {
                    if let Some(cmd) = commands_later.get(cmd_name) {
                        run_cmd(
                            cmd_name,
                            cmd.cmd(),
                            &running_pair_later,
                            &tx_stdout_later,
                            &tx_stderr_later,
                            &tx_status_later,
                        );
                    }
                }
            });
        });

        self.fut_handles.push(later_handle);
    }

    fn store_realtime(&mut self, on_calendar: &str, _persistent: bool, cmds: &[String]) {
        match parse_calendar(on_calendar) {
            Ok(rt) => {
                info!("adding realtime schedule {rt:?}");
                let _prev = self.rt.insert(rt, cmds.to_vec());
            }
            Err(e) => error!("{e}"),
        }
    }

    fn initialize(&mut self, ctx: &mut Context<Self>) {
        // initialze the queue monitor
        self.queue_monitor(ctx);
        // request initialization from the server
        if let Ok(init) = serialize(&WorkerClientToWorkerSession::Initialize) {
            if let Err(_e) = self.addr.write(Message::Binary(Bytes::from(init))) {
                error!("Unable to send initialize message");
            }
        } else {
            error!("Unable to serialize initialize message");
        }
    }
}

impl Actor for Worker {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("worker actor started");
        // start heartbeat otherwise server will disconnect after 10 seconds
        self.hb(ctx);
        // request initialization from the server
        self.initialize(ctx);
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        info!("worker actor stopped");
        // Stop application on disconnect
        System::current().stop();
    }
}

/// Handle server websocket messages
impl StreamHandler<Result<Frame, WsProtocolError>> for Worker {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, ctx: &mut Self::Context) {
        if let Ok(message) = msg {
            match message {
                Frame::Binary(bytes) => self.handle_binary(ctx, &bytes),
                Frame::Text(bytes) => self.handle_text(&bytes),
                Frame::Ping(bytes) => self.handle_ping(bytes),
                Frame::Pong(bytes) => self.handle_pong(&bytes),
                Frame::Close(reason) => self.handle_close(ctx, reason),
                Frame::Continuation(item) => self.handle_continuation(ctx, item),
            }
        }
    }

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("worker stream handler started");
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        info!("worker stream handler finished");
        ctx.stop();
    }
}

impl WriteHandler<WsProtocolError> for Worker {}

impl Handler<WorkerClientToWorkerSession> for Worker {
    type Result = ();

    fn handle(&mut self, msg: WorkerClientToWorkerSession, _ctx: &mut Context<Self>) {
        match serialize(&msg) {
            Ok(msg_bytes) => self.stdout_queue.push_back(msg_bytes),
            Err(e) => error!("{e}"),
        }
    }
}

fn run_cmd(
    name: &str,
    command: &str,
    running_pair: &Arc<(Mutex<bool>, Condvar)>,
    tx_stdout: &UnboundedSender<WorkerClientToWorkerSession>,
    tx_stderr: &UnboundedSender<WorkerClientToWorkerSession>,
    tx_status: &UnboundedSender<WorkerClientToWorkerSession>,
) {
    if let Some(shell_path) = env::var_os("SHELL") {
        let command_id = Uuid::new_v4();
        info!("Running '{name}'");
        let shell = shell_path.to_string_lossy().to_string();
        let mut cmd = std::process::Command::new(shell);
        let _ = cmd.arg("-c");
        let _ = cmd.arg(command);
        let _ = cmd.stdout(Stdio::piped());
        let _ = cmd.stderr(Stdio::piped());

        if let Ok(mut child) = cmd.spawn() {
            let _stdout_handle_opt = if let Some(child_stdout) = child.stdout.take() {
                let tx_stdout = tx_stdout.clone();
                let stdout_handle = thread::spawn(move || {
                    let stdout_reader = BufReader::new(child_stdout);
                    for line in stdout_reader.lines().flatten() {
                        let stdout_m = WorkerClientToWorkerSession::Stdout {
                            id: command_id,
                            line,
                        };
                        if let Err(e) = tx_stdout.send(stdout_m) {
                            error!("{e}");
                        }
                    }
                });
                Some(stdout_handle)
            } else {
                error!("Unable to produce stdout!");
                None
            };

            let _stderr_handle_opt = if let Some(child_stderr) = child.stderr.take() {
                let tx_stderr = tx_stderr.clone();
                let stderr_handle = thread::spawn(move || {
                    let stderr_reader = BufReader::new(child_stderr);
                    for line in stderr_reader.lines().flatten() {
                        let stderr_m = WorkerClientToWorkerSession::Stderr {
                            id: command_id,
                            line,
                        };
                        if let Err(e) = tx_stderr.send(stderr_m) {
                            error!("{e}");
                        }
                    }
                });
                Some(stderr_handle)
            } else {
                error!("Unable to produce stderr!");
                None
            };

            let pair = running_pair.clone();

            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        if let Some(code) = status.code() {
                            info!("command result: {}", code);
                            let status_msg = WorkerClientToWorkerSession::Status {
                                id: command_id,
                                code,
                            };
                            if let Err(e) = tx_status.send(status_msg) {
                                error!("{e}");
                            }
                        }
                        break;
                    }
                    Ok(None) => {
                        let (lock, cvar) = &*pair;
                        let running = match lock.lock() {
                            Ok(guard) => guard,
                            Err(poisoned) => poisoned.into_inner(),
                        };
                        if let Ok((res, wt_res)) =
                            cvar.wait_timeout(running, Duration::from_millis(500))
                        {
                            if wt_res.timed_out() {
                                debug!("timed out waiting on cvar, checking running flag");
                            }
                            // If we aren't in a running state, try to kill the child process
                            if !(*res) {
                                if let Err(e) = child.kill() {
                                    error!("Unable to kill child process: {e}");
                                }
                                break;
                            }
                        } else {
                            error!("condvar wait timeout error");
                        }
                    }
                    Err(e) => error!("{e}"),
                }
            }
        } else {
            error!("unable to spawn command");
        }
    } else {
        error!("no shell defined!");
    }
}
