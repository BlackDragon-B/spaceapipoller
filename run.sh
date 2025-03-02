#!/usr/bin/with-contenv bashio
set +u

export mqtt_broker=$(bashio::config 'mqtt_broker')
export mqtt_username=$(bashio::config 'mqtt_username')
export mqtt_password=$(bashio::config 'mqtt_password')
export mqtt_port=$(bashio::config 'mqtt_port')
export directory=$(bashio::config 'directory')
export spaces=$(bashio::config 'spaces')
export polling_rate=$(bashio::config 'polling_rate')

bashio::log.info "mqtt_broker configured as ${mqtt_broker}."
bashio::log.info "mqtt_username configured as ${mqtt_username}."
bashio::log.info "mqtt_password configured as ${mqtt_password}."
bashio::log.info "mqtt_port configured as ${mqtt_port}."
bashio::log.info "directory configured as ${directory}."
bashio::log.info "spaces configured as ${spaces}."
bashio::log.info "polling_rate configured as ${polling_rate}."

./spaceapipoller