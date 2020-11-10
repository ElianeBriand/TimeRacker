use serde::{Serialize, Deserialize};
use std::sync::{Arc, Weak};
use std::time::Instant;


pub const CLIENTREQUEST_MAX_PARTS: usize = 4;
pub enum ClientRequest {
    GetState {  },
    SwitchTopic { id: u64},
    CreateTopic {name: String, parent_id: u64},
    UpdateTopic {id: u64, name: String, parent_id: u64, duration: u64},
    DeleteTopic {id: u64},
    Bye {},
    Terminate {}
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseToClient {
    State {
        value: String
    },
    Success {
        details: String
    },
    Error {
        error_code: u64,
        msg: String,
    },
    Bye {},
    Terminating {}
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub struct TimeTrackingTopic {
    pub id: u64,
    pub name: String,
    pub duration: u64,
    pub parent: Weak<TimeTrackingTopic>,
    pub dependants: Vec<Arc<TimeTrackingTopic>>,
}

pub struct TimeTrackingImplDetails {
    pub current_topic_start_instant: Instant
}

impl TimeTrackingImplDetails {
    pub fn new() -> TimeTrackingImplDetails {
        TimeTrackingImplDetails {
            current_topic_start_instant: Instant::now()
        }
    }
}

impl ::std::default::Default for TimeTrackingImplDetails {
    fn default() -> Self { Self::new() }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub struct TimeTrackingState {
    pub last_assigned_topic_id: u64,
    pub current_topic_id: u64,
    pub topics_tree: Vec<TimeTrackingTopic>,

    #[serde(skip)]
    pub details: TimeTrackingImplDetails
}





impl ClientRequest {
    pub fn emit(&self) -> String {
        match self {
            ClientRequest::GetState{} => {"GET_STATE".to_string()},
            ClientRequest::SwitchTopic{id} => {format!("SWITCH_TOPIC {}", id)},
            ClientRequest::CreateTopic{name, parent_id} => {format!("CREATE_TOPIC {} {}", name, parent_id)},
            ClientRequest::UpdateTopic{id, name, parent_id, duration} => {format!("UPDATE_TOPIC {} {} {} {}", id, name, parent_id, duration)},
            ClientRequest::DeleteTopic{id} => {format!("DELETE_TOPIC {} ", id)},
            ClientRequest::Bye{} => {"BYE".to_string()},
            ClientRequest::Terminate{} => {"TERMINATE".to_string()},
        }
    }

    pub fn parse(input: &str) -> Result<ClientRequest, String> {
        let mut parts = input.splitn(CLIENTREQUEST_MAX_PARTS, ' ');
        match parts.next() {
            Some("GET_STATE") => {
                if parts.next().is_some() {
                    return Err("GET_STATE does not take arguments".into());
                }
                Ok(ClientRequest::GetState { })
            }

            Some("BYE") => {
                if parts.next().is_some() {
                    return Err("BYE does not take arguments".into());
                }
                Ok(ClientRequest::Bye { })
            }

            Some("TERMINATE") => {
                if parts.next().is_some() {
                    return Err("BYE does not take arguments".into());
                }
                Ok(ClientRequest::Terminate { })
            }

            Some("SWITCH_TOPIC") => {
                let id_str = parts.next().ok_or("SWITCH_TOPIC must be followed by an id")?;
                if parts.next().is_some() {
                    return Err("SWITCH_TOPIC takes exactly one argument".into());
                }
                let id = id_str.parse();
                if id.is_err() {
                    return Err("SWITCH_TOPIC argument must be an unsigned integer (u64)".into());
                }
                let id= id.unwrap();
                Ok(ClientRequest::SwitchTopic { id })
            }

            Some("CREATE_TOPIC") => {
                let name_str = parts.next();
                let id_str = parts.next();
                if parts.next().is_some() || id_str.is_none() || name_str.is_none() {
                    return Err("CREATE_TOPIC must be followed by exactly two arguments (name and parent id)".into());
                }
                let id_str = id_str.unwrap();
                let id = id_str.parse();
                if id.is_err() {
                    return Err("CREATE_TOPIC second argument must be an unsigned integer (u64)".into());
                }
                let id= id.unwrap();
                Ok(ClientRequest::CreateTopic {
                    name: name_str.unwrap().to_string(), parent_id: id
                })
            }

            Some("UPDATE_TOPIC") => {
                let id_str = parts.next();
                let name_str = parts.next();
                let parent_id_str = parts.next();
                let duration_str = parts.next();
                if parts.next().is_some() || name_str.is_none() || parent_id_str.is_none() || duration_str.is_none() {
                    return Err("UPDATE_TOPIC must be followed by exactly four arguments (id, name, parent id, duration)".into());
                }

                let name_str = name_str.unwrap();
                let id_str = id_str.unwrap();
                let parent_id_str = parent_id_str.unwrap();
                let duration_str = duration_str.unwrap();

                let id = id_str.parse();
                if id.is_err() {
                    return Err("UPDATE_TOPIC first argument must be an unsigned integer (u64)".into());
                }
                let id= id.unwrap();

                let parent_id = parent_id_str.parse();
                if parent_id.is_err() {
                    return Err("UPDATE_TOPIC third argument must be an unsigned integer (u64)".into());
                }
                let parent_id= parent_id.unwrap();

                let duration = duration_str.parse();
                if duration.is_err() {
                    return Err("UPDATE_TOPIC fourth argument must be an unsigned integer (u64)".into());
                }
                let duration= duration.unwrap();


                Ok(ClientRequest::UpdateTopic {
                    id, name: name_str.to_string(), parent_id, duration
                })
            }

            Some("DELETE_TOPIC") => {
                let id_str = parts.next().ok_or("DELETE_TOPIC must be followed by an id")?;
                if parts.next().is_some() {
                    return Err("DELETE_TOPIC takes exactly one argument".into());
                }

                let id = id_str.parse();
                if id.is_err() {
                    return Err("DELETE_TOPIC argument must be an unsigned integer (u64)".into());
                }
                let id= id.unwrap();

                Ok(ClientRequest::DeleteTopic { id })
            }

            Some(cmd) => Err(format!("unknown command: {}", cmd)),
            None => Err("empty input".into()),
        }
    }
}
