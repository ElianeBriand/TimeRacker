use std::env;
use std::sync::{Arc, Mutex, Weak, MutexGuard};
use tokio::net::TcpListener;
use tokio::stream::StreamExt;
use tokio_util::codec::{Framed, LinesCodec};
use futures::SinkExt;




use timeracker_common::{ClientRequest, ResponseToClient, TimeTrackingTopic, TimeTrackingState, TimeTrackingImplDetails};
use std::time::Instant;


fn terminate_server() {
    println!("    TimeRacker core exiting...");
    std::process::exit(0);
}

fn find_topic(id: u64, topics_tree: &Vec<TimeTrackingTopic>) -> Option<&TimeTrackingTopic> {
    for ttt in topics_tree.iter() {
        if ttt.id == id {
            return Some(ttt);
        }
    }
    return None
}

fn find_topic_mutable(id: u64, topics_tree: &mut Vec<TimeTrackingTopic>) -> Option<&mut TimeTrackingTopic> {
    for ttt in topics_tree.iter_mut() {
        if ttt.id == id {
            return Some(ttt);
        }
    }
    return None
}

fn update_duration(state: &mut MutexGuard<TimeTrackingState>) {
    let curr_duration = state.details.current_topic_start_instant.elapsed();
    let curr_topic = find_topic_mutable(state.current_topic_id, &mut state.topics_tree).unwrap();
    curr_topic.duration += curr_duration.as_secs();
}

fn reset_current_topic_start_instant(state: &mut MutexGuard<TimeTrackingState>) {
    state.details.current_topic_start_instant = Instant::now();
}

#[tokio::main]
async fn main(){
    println!("    TimeRacker core starting...");

    let off_topic= TimeTrackingTopic {
        id: 0,
        name: "OFF".to_string(),
        duration: 0,
        parent: Weak::new(), // empty
        dependants: vec![],
    };

    let idle_topic=  TimeTrackingTopic {
        id: 1,
        name: "Idle".to_string(),
        duration: 0,
        parent: Weak::new(), // empty
        dependants: vec![],
    };

    let example_topic= TimeTrackingTopic {
        id: 2,
        name: "Work".to_string(),
        duration: 0,
        parent: Weak::new(), // empty
        dependants: vec![],
    };

    let main_state = Arc::new(Mutex::new(TimeTrackingState  {
        last_assigned_topic_id: 2,
        current_topic_id: 0,
        topics_tree: vec![],
        details: TimeTrackingImplDetails::new()
    }));

    main_state.lock().unwrap().topics_tree.push(off_topic);
    main_state.lock().unwrap().topics_tree.push(idle_topic);
    main_state.lock().unwrap().topics_tree.push(example_topic);

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:45862".to_string());
    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("    Listening on: {}", addr);


    loop {
        match listener.accept().await {
            Ok((socket, _)) => {
                println!("    Accepted connection...");
                let local_state = main_state.clone();

                tokio::spawn(async move {
                    let mut lines = Framed::new(socket, LinesCodec::new());

                    while let Some(result) = lines.next().await {
                        match result {
                            Ok(line) => {
                                let response = handle_request(&line, &local_state);

                                let response_str = serde_json::to_string(&response).unwrap();
                                if let Err(e) = lines.send(response_str.as_str()).await {
                                    println!("[E] Error on sending response; error = {:?}", e);
                                    println!("    Dropping connection");
                                    break;
                                }



                                match response {
                                    ResponseToClient::Bye {} => break,
                                    ResponseToClient::Terminating {} => terminate_server(),
                                    _ => ()
                                }
                            }
                            Err(e) => {
                                println!("[E] Error on decoding from socket; error = {:?}", e);
                                println!("    Dropping connection");
                                break;
                            }
                        }
                    }
                });
            }
            Err(e) => println!("error accepting socket; error = {:?}", e),
        }
    }
}


fn handle_request(line: &str, state: &Arc<Mutex<TimeTrackingState>>) -> ResponseToClient {
    let request = match ClientRequest::parse(&line) {
        Ok(req) => req,
        Err(e) => return ResponseToClient::Error { error_code: 400, msg: e },
    };

    // let mut topics = state.map.lock().unwrap();
    match request {
        ClientRequest::GetState{ } =>  {
            println!("    Processing GET_STATE...");
            let mut local_state_guard = state.lock().unwrap();
            update_duration(&mut local_state_guard);
            reset_current_topic_start_instant(&mut local_state_guard);
            let response_string = serde_json::to_string(&(*local_state_guard)).unwrap();
            ResponseToClient::State {value: response_string}
        },

        ClientRequest::Bye{ } =>  {
            println!("    Processing BYE...");
            ResponseToClient::Bye { }
        },

        ClientRequest::Terminate{ } =>  {
            println!("    Processing TERMINATE...");
            ResponseToClient::Terminating { }
        },


        ClientRequest::SwitchTopic { id } => {
            println!("    Processing SWITCH_TOPIC...");
            let mut local_state_guard = state.lock().unwrap();
            let mut new_topic_id: Option<u64> = None;
            let mut new_topic_name = String::new();
            {
                let topic = find_topic(id, &(local_state_guard.topics_tree));
                if topic.is_some() {
                    new_topic_id = Some(topic.unwrap().id);
                    new_topic_name = topic.unwrap().name.clone();
                } else {
                }
            }

            if new_topic_id.is_some() {
                update_duration(&mut local_state_guard);
                local_state_guard.current_topic_id = new_topic_id.unwrap();
                reset_current_topic_start_instant(&mut local_state_guard);
                println!("Switched topic to {} : {}", new_topic_id.unwrap(),  new_topic_name);
                ResponseToClient::Success {details: format!("Switched topic to {}", new_topic_name)}
            } else {
                ResponseToClient::Error {error_code: 404, msg: "Topic not found".to_string()}
            }

        },

        /*
        ClientRequest::CreateTopic { key, value } => {
            let previous = db.insert(key.clone(), value.clone());
            Response::Set {
                key,
                value,
                previous,
            }
        },
        */

        _ => ResponseToClient::Error { error_code: 500, msg: "Server error".to_string()},
    }
}





