mod utils;
use rumqttc::{Client, MqttOptions};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::HashMap, process, thread, time};
use utils::{get_endpoint, to_safe_entity_name};

#[derive(Debug)]
struct Space {
    name: String,
    entity_name: String,
    endpoint: String,
    state: bool,
}

impl Space {
    fn build_discovery_packet(&self) -> (String, Value) {
        let payload = json!({
            "name": format!("{} Spacestate",&self.name), // Friendly name
            "unique_id": &self.entity_name, // Unique identifier
            "state_topic": format!("spacestate/{}/state",&self.entity_name), // Topic for state updates
            "payload_on": "OPEN", // Payload for "on" state
            "payload_off": "CLOSED", // Payload for "off" state
        });
        (
            format!(
                "homeassistant/binary_sensor/spacestate/{}/config",
                &self.entity_name
            ),
            payload,
        )
    }
    fn build_state_packet(&self) -> (String, String) {
        let state: &str = match &self.state {
            true => "OPEN",
            false => "CLOSED",
        };
        (
            format!("spacestate/{}/state", &self.entity_name),
            state.to_string(),
        )
    }
}

fn default_polling_rate() -> u64 {
    600
}

fn default_directory() -> String {
    "https://directory.spaceapi.io".to_string()
}

fn default_mqtt_port() -> u16 {
    1883
}

#[derive(Deserialize, Debug)]
struct Config {
    mqtt_broker: String,
    mqtt_username: Option<String>,
    mqtt_password: Option<String>,
    #[serde(default = "default_mqtt_port")]
    mqtt_port: u16,
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
    let mut mqttoptions = MqttOptions::new("spaceapipoller", config.mqtt_broker, config.mqtt_port);
    if config.mqtt_username.is_some() && config.mqtt_password.is_some() {
        mqttoptions.set_credentials(config.mqtt_username.unwrap(), config.mqtt_password.unwrap());
    }
    let (client, mut connection) = rumqttc::Client::new(mqttoptions, 10);
    thread::spawn(move || {
        for (i, notification) in connection.iter().enumerate() {
            match notification {
                Ok(notif) => {}
                Err(error) => {
                    println!("ERROR: MQTT {error:?}");
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
                Ok(status) => {
                    println!(
                        "INFO: Adding space {} with endpoint {}",
                        entry.key(),
                        entry.get()
                    );
                    spaces.push(Space {
                        name: entry.key().to_string(),
                        entity_name: to_safe_entity_name(&format!(
                            "{} Spacestate",
                            entry.key().to_string()
                        )),
                        endpoint: entry.get().to_string(),
                        state: status.state.and_then(|s| s.open).unwrap_or(false),
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
    //Exit if no eligible spaces are available/set
    if spaces.len() == 0 {
        eprintln!("ERROR: No eligible space available",);
        process::exit(1)
    }
    //Send discovery topic to home assistant
    for space in &spaces {
        let (discovery_topic, payload) = space.build_discovery_packet();
        if let Err(e) = client.publish(
            discovery_topic,
            rumqttc::QoS::AtLeastOnce,
            true,
            payload.to_string(),
        ) {
            println!("ERROR: Failed to send MQTT discovery packet ({})", e)
        };
        let (discovery_topic, payload) = space.build_state_packet();
        if let Err(e) = client.publish(discovery_topic, rumqttc::QoS::AtLeastOnce, true, payload) {
            println!("ERROR: Failed to send MQTT state packet ({})", e)
        };
    }
    //Start polling loop
    loop {
        for space in &mut spaces {
            match get_endpoint(&mut space.endpoint) {
                Ok(status) => {
                    let state = status.state.and_then(|s| s.open).unwrap_or(false);
                    if state != space.state {
                        //Send MQTT update, state changed
                        space.state = state;
                        let (discovery_topic, payload) = space.build_state_packet();
                        if let Err(e) = client.publish(
                            discovery_topic,
                            rumqttc::QoS::AtLeastOnce,
                            true,
                            payload,
                        ) {
                            println!("ERROR: Failed to send MQTT state packet ({})", e)
                        };
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
        thread::sleep(time::Duration::from_secs(config.polling_rate))
    }
}
