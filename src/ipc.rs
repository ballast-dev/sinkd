use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use paho_mqtt as mqtt;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::{
    bad, config, ipc,
    outcome::Outcome,
    parameters::{DaemonType, Parameters},
    shiplog,
};

pub type Rx = mqtt::Receiver<Option<mqtt::Message>>;

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Reason {
    Sinking,
    Behind,
    Other,
}

#[derive(PartialEq, Default, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Status {
    NotReady(Reason),
    #[default]
    Ready,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::NotReady(reason) => {
                write!(f, "NotReady(").unwrap();
                match reason {
                    Reason::Sinking => write!(f, "Sinking").unwrap(),
                    Reason::Behind => write!(f, "Behind").unwrap(),
                    Reason::Other => write!(f, "Other").unwrap(),
                };
                write!(f, ")") // return result of write
            }
            Status::Ready => write!(f, "Ready"),
        }
    }
}

/// Only time a Payload is sent is to say "new edits"
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Payload {
    pub hostname: String,
    pub username: String,
    pub src_paths: Vec<PathBuf>,
    pub date: String,
    pub cycle: u32,
    pub status: Status,
    pub dest_path: String,
}

#[allow(dead_code)]
impl Payload {
    pub fn new() -> Outcome<Payload> {
        //Ok(Payload::default())
        Ok(Payload {
            hostname: config::get_hostname()?,
            username: config::get_username()?,
            src_paths: vec![],
            date: String::from("2022Jan4"),
            cycle: 0,
            status: Status::Ready,
            dest_path: String::from("server"),
        })
    }

    pub fn from(
        hostname: String,
        username: String,
        src_paths: Vec<PathBuf>,
        dest_path: String,
        date: String,
        cycle: u32,
        status: Status,
    ) -> Payload {
        Payload {
            hostname,
            username,
            src_paths,
            dest_path,
            date,
            cycle,
            status,
        }
    }

    pub fn hostname<S: Into<String>>(mut self, hostname: S) -> Self {
        self.hostname = hostname.into();
        self
    }

    pub fn username<S: Into<String>>(mut self, username: S) -> Self {
        self.username = username.into();
        self
    }

    pub fn src_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.src_paths = paths;
        self
    }

    /// Must be in form YYYYMMMDD, i.e. 2022Jan4
    pub fn date<S: Into<String>>(mut self, date: S) -> Self {
        self.date = date.into();
        self
    }

    pub fn cycle(mut self, cycle: u32) -> Self {
        self.cycle = cycle;
        self
    }

    pub fn status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    pub fn dest_path<S: Into<String>>(mut self, dest_path: S) -> Self {
        self.dest_path = dest_path.into();
        self
    }
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "hostname: {}, username: {}, paths: [",
            self.hostname, self.username,
        )
        .unwrap();
        for path in &self.src_paths {
            write!(f, "{}, ", path.display()).unwrap();
        }
        write!(
            f,
            "], date: {}, cycle: {}, status: {}",
            self.date, self.cycle, self.status
        )
    }
}

/// Adds timestamp and serializes payload for transfer
pub fn encode(payload: &mut Payload) -> Result<Vec<u8>, mqtt::Error> {
    payload.date = shiplog::get_timestamp("%Y%m%d");
    match bincode::serialize(payload) {
        Err(e) => Err(mqtt::Error::GeneralString(format!(
            "FATAL, bincode::serialize >> {e}"
        ))),
        Ok(stream) => Ok(stream),
    }
}

pub fn decode(bytes: &[u8]) -> Result<Payload, mqtt::Error> {
    match bincode::deserialize(bytes) {
        Err(_) => Err(mqtt::Error::General("FATAL, bincode could not deserialize")),
        Ok(payload) => Ok(payload),
    }
}

/// A wrapper for sinkd implementation
pub struct MqttClient {
    pub client: mqtt::Client,
    publish_topic: String,
}

impl MqttClient {
    pub fn new(
        host: Option<&str>,
        subscriptions: &[&str],
        publish_topic: &str,
    ) -> Result<(Self, mqtt::Receiver<Option<mqtt::Message>>), mqtt::Error> {
        let hostname = config::get_hostname()
            .map_err(|_| mqtt::Error::GeneralString(String::from("unable to get hostname")))?;
        let opts = mqtt::CreateOptionsBuilder::new()
            .server_uri(resolve_host(host)?)
            // TODO: should pass params
            //.client_id(hostname)  // MUST BE UNIQUE
            .finalize();
        let cli = mqtt::Client::new(opts)?;

        let rx = cli.start_consuming();

        let lwt = mqtt::MessageBuilder::new()
            .topic("sinkd/server")
            .payload("Sync consumer lost connection")
            .finalize();

        let conn_opts = mqtt::ConnectOptionsBuilder::new_v3()
            .keep_alive_interval(Duration::from_secs(20))
            .clean_session(true)
            .will_message(lwt)
            .finalize();

        let qos = vec![1; subscriptions.len()]; // Ensure QoS length matches subscriptions

        debug!(
            "Connecting to MQTT broker at host: {}, subscriptions: [{}], publish_topic: {}",
            host.unwrap_or("unknown"),
            subscriptions.to_vec().join(", "),
            publish_topic
        );

        match cli.connect(conn_opts) {
            Ok(rsp) => {
                if let Some(con_rsp) = rsp.connect_response() {
                    debug!(
                        "Connected to: '{}' with MQTT version {}",
                        con_rsp.server_uri, con_rsp.mqtt_version
                    );
                    if con_rsp.session_present {
                        return Err(mqtt::Error::General("Client session already present on broker"));
                    }

                    debug!("Subscribing to topics: {:?} with QoS {:?}", subscriptions, qos);
                    cli.subscribe_many(subscriptions, &qos)
                        .map_err(|_| mqtt::Error::General("Failed to subscribe to topics"))?;

                    Ok((
                        MqttClient {
                            client: cli,
                            publish_topic: publish_topic.to_owned(),
                        },
                        rx,
                    ))
                } else {
                    Err(mqtt::Error::General("No connection response from broker"))
                }
            }
            Err(e) => Err(mqtt::Error::GeneralString(format!(
                "Could not connect to broker '{}': {:?}. Ensure the broker is running and reachable.",
                host.unwrap_or("unknown"),
                e
            ))),
        }
    }

    pub fn publish(&self, payload: &mut Payload) -> Outcome<()> {
        match self.client.publish(mqtt::Message::new(
            &self.publish_topic,
            encode(payload)?,
            mqtt::QOS_1, // at least once
        )) {
            Ok(()) => {
                info!("published payload: {}", payload);
                Ok(())
            }
            Err(e) => {
                error!("could not publish payload {}, {}", payload, e);
                bad!("could not publish payload {}, {}", payload, e)
            }
        }
    }
}

fn resolve_host(host: Option<&str>) -> Result<String, mqtt::Error> {
    match host {
        Some(h) if h.starts_with('/') => Err(mqtt::Error::General(
            "Invalid hostname: it looks like a path. Did you mean 'localhost'?",
        )),
        Some(h) => {
            let fq_host = format!("tcp://{}:1883", h);
            debug!("Fully qualified host: {}", fq_host);
            Ok(fq_host)
        }
        None => Err(mqtt::Error::General("Host string is required but missing")),
    }
}

pub fn start_mosquitto() -> Outcome<()> {
    debug!(">> spawn mosquitto daemon");
    //? This command will not spawn new instances
    //? if mosquitto already active.
    if let Err(spawn_error) = std::process::Command::new("mosquitto").arg("-d").spawn() {
        return bad!(format!(
            "Is mosquitto installed and in path? >> {}",
            spawn_error
        ));
    }
    Ok(())
}

pub fn daemon(
    func: fn(&Parameters) -> Outcome<()>,
    app_type: &str,
    params: &Parameters,
) -> Outcome<()> {
    use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
    use nix::unistd::{fork, ForkResult};

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child, .. }) => {
            let start_time = Instant::now();
            let timeout = Duration::from_secs(2);

            while start_time.elapsed() < timeout {
                match waitpid(child, Some(WaitPidFlag::WNOHANG)) {
                    Ok(status) => match status {
                        WaitStatus::Exited(_, _) => {
                            return bad!(format!("{} encountered error", app_type))
                        }
                        _ => shiplog::set_pid(params, child.as_raw() as u32)?,
                    },
                    Err(e) => eprintln!("Failed to wait on child?: {e}"),
                }
                std::thread::sleep(Duration::from_secs(1));
            }
            println!("spawned, logging to '{}'", params.log_path.display());
            Ok(())
        }
        Ok(ForkResult::Child) => {
            info!("about to start daemon...");
            func(params)
        }
        Err(_) => {
            bad!("Failed to fork process")
        }
    }
}

pub fn end_process(params: &Parameters) -> Outcome<()> {
    if !params.debug && !config::have_permissions() {
        return bad!("Need to be root");
    }

    // TODO: return PID_NOT_FOUND
    if !params.pid_path.exists() {
        println!(
            "sinkd {} is not running",
            if params.daemon_type == DaemonType::Client {
                "client"
            } else {
                "server"
            }
        );
        return Ok(());
    }

    let pid = shiplog::get_pid(params)?;
    let nix_pid = Pid::from_raw(pid as i32);

    match kill(nix_pid, Some(Signal::SIGTERM)) {
        Ok(()) => {
            // Process exists and can be signaled
            if let Err(e) = std::process::Command::new("kill")
                .arg("-15") // SIGTERM
                .arg(format!("{pid}"))
                .output()
            {
                return bad!("Couldn't kill process {} {}", pid, e);
            }
            shiplog::rm_pid(params)?;
            Ok(())
        }
        Err(_) => {
            bad!(
                "Process with PID {} does not exist or cannot be signaled",
                pid
            )
        }
    }
}

pub fn rsync(payload: &ipc::Payload) {
    // Agnostic pathing allows sinkd not to care about user folder structure

    debug!("{}", payload);

    // FIXME:
    // TODO:
    // NOTE:
    // HACK:
    // WARNING:

    let pull = payload.status == ipc::Status::NotReady(ipc::Reason::Behind);

    let src: Vec<PathBuf> = if pull {
        payload
            .src_paths
            .iter()
            .map(|p| PathBuf::from(format!("{}:{}", payload.hostname, p.display())))
            .collect()
    } else {
        payload.src_paths.clone()
    };

    // NOTE: this is assuming that dest will always be a single point
    let dest: String = if pull {
        payload.dest_path.clone()
    } else {
        format!("{}:{}", payload.hostname, payload.dest_path)
    };

    // need to account for shared folders
    // and local sync? maybe useful for testing
    let mut cmd = std::process::Command::new("rsync"); // have to bind at .new()
    cmd.arg("-atR") // archive, timestamps, relative
        .arg("--delete") // delete on destination if not reflected in source
        //? RSYNC options to consider
        // .arg("--delete-excluded")
        // .arg("--max-size=SIZE") // (limit size of transfers)
        // .arg("--exclude=PATTERN") // loop through to all all paths
        .args(&src)
        .arg(&dest);

    match cmd.spawn() {
        Err(x) => {
            error!("{:?}", x);
        }
        Ok(_) => {
            debug!("called rsync! src_paths: {:?}  dest_path: {}", src, dest);
        }
    }
}

/*
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{fork, ForkResult};

pub fn detach() {
    // TODO: need packager to setup file with correct permisions

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child, .. }) => {
            let start_time = Instant::now();
            let timeout = Duration::from_secs(2);

            while start_time.elapsed() < timeout {
                match waitpid(child, Some(WaitPidFlag::WNOHANG)) {
                    Ok(status) => match status {
                        WaitStatus::Exited(_, _) => return bad!("client encountered error"),
                        _ => (),
                    },
                    Err(e) => eprintln!("Failed to wait on child?: {}", e),
                }
                std::thread::sleep(Duration::from_secs(1));
            }
            println!("spawned, logging to '{}'", params.log_path.display());
            Ok(())
        }
        Ok(ForkResult::Child) => {
            info!("about to start daemon...");
            //TODO: pass in a function pointer
            init(params)
        }
        Err(_) => {
            bad!("Failed to fork process")
        }
    }
}
*/
