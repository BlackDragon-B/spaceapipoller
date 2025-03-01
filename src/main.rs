mod utils;
use rumqttc::MqttOptions;
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, process, thread, time};
use utils::{get_endpoint, to_safe_entity_name};

#[derive(Debug)]
struct Space {
    name: String,
    endpoint: String,
    state: bool,
}

fn default_polling_rate() -> u64 {
    600
}

fn default_directory() -> String {
    "https://directory.spaceapi.io".to_string()
}

fn default_mqtt_port() -> String {
    "1883".to_string()
}

#[derive(Deserialize, Debug)]
struct Config {
    mqtt_broker: String,
    mqtt_username: Option<String>,
    mqtt_password: Option<String>,
    #[serde(default = "default_mqtt_port")]
    mqtt_port: String,
    #[serde(default = "default_directory")]
    directory: String,
    spaces: String,
    #[serde(default = "default_polling_rate")]
    polling_rate: u64,
}

fn main() {
    //Load config
    let config = match envy::from_env::<Config>() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("ERROR: {}", err.to_string());
            process::exit(1)
        }
    };
    //Connect to MQTT Broker
    let mut mqttoptions = MqttOptions::new("test-1", config.mqtt_broker, 1883);
    if config.mqtt_username.is_some() && config.mqtt_password.is_some() {
        mqttoptions.set_credentials(config.mqtt_username.unwrap(), config.mqtt_password.unwrap());
    }
    let (client, mut connection) = rumqttc::Client::new(mqttoptions, 10);
    thread::spawn(move || {
        for (i, notification) in connection.iter().enumerate() {
            match notification {
                Ok(notif) => {
                    println!("{i}. Notification = {notif:?}");
                }
                Err(error) => {
                    println!("{i}. Notification = {error:?}");
                    return;
                }
            }
        }    
    });
    //Load directory
    let request = reqwest::blocking::Client::new()
        .get(config.directory)
        .send();
    let mut directory = match request {
        Ok(response) => {
            let content = response.text().unwrap();
            let data: HashMap<String, String> = match serde_json::from_str(&content) {
                Ok(d) => d,
                Err(err) => {
                    eprintln!(
                        "ERROR: Failed to parse spaceapi directory: {}",
                        err.to_string()
                    );
                    process::exit(1)
                }
            };
            data
        }
        Err(err) => {
            eprintln!(
                "ERROR: Failed to request spaceapi directory: {}",
                err.to_string()
            );
            process::exit(1)
        }
    };
    //Validate if the spaces in the environment variable exist in the directory
    let mut spaces: Vec<Space> = Vec::new();
    for space in config.spaces.split(";") {
        match directory.entry(space.to_string()) {
            std::collections::hash_map::Entry::Occupied(entry) => match get_endpoint(entry.get()) {
                Ok(_) => {
                    println!(
                        "INFO: Adding space {} with endpoint {}",
                        entry.key(),
                        entry.get()
                    );
                    spaces.push(Space {
                        name: entry.key().to_string(),
                        endpoint: entry.get().to_string(),
                        state: false,
                    })
                }
                Err(err) => {
                    println!(
                        "WARNING: Dropping space {} unavailable with error: {}",
                        space, err
                    )
                }
            },
            std::collections::hash_map::Entry::Vacant(_) => {
                println!("WARNING: Space {} not found in directory", space)
            }
        }
    }
    client.publish("test/state", rumqttc::QoS::AtLeastOnce, true, "test").unwrap();
    //Exit if no eligible spaces are available/set
    if spaces.len() == 0 {
        eprintln!("ERROR: No eligible space available",);
        process::exit(1)
    }
    //Send discovery topic to home assistant
    for space in &spaces {
        println!("yes");
        let discovery_topic = format!("homeassistant/binary_sensor/spacestate/{}/config",to_safe_entity_name(&format!("{} Spacestate",space.name)));
        let payload = json!({
            "name": format!("{} Spacestate",space.name), // Friendly name
            "unique_id": to_safe_entity_name(&format!("{} Spacestate",space.name)), // Unique identifier
            "state_topic": format!("spacestate/{}/state",to_safe_entity_name(&format!("{} Spacestate",space.name))), // Topic for state updates
            "payload_on": "OPEN", // Payload for "on" state
            "payload_off": "CLOSED", // Payload for "off" state
        });
        client.publish(discovery_topic, rumqttc::QoS::AtLeastOnce, true, payload.to_string()).unwrap();
    }
    //Start polling loop
    loop {
        for space in &mut spaces {
            match get_endpoint(&mut space.endpoint) {
                Ok(status) => {
                    let state = status.state.and_then(|s| s.open).unwrap_or(false);
                    if state != space.state {
                        //Send MQTT update, state changed
                        let s = match state {
                            true => "OPEN",
                            false => "CLOSED",
                        };
                        let _ = client.publish(format!("spacestate/{}/state",to_safe_entity_name(&format!("{} Spacestate",space.name))), rumqttc::QoS::AtLeastOnce, true, s);
                        space.state = state;
                    }
                }
                Err(err) => {
                    println!(
                        "WARNING: Space {} unavailable with error: {}",
                        space.name, err
                    )
                }
            }
        }
        println!("{:?}", spaces);
        thread::sleep(time::Duration::from_secs(config.polling_rate))
    }
}
