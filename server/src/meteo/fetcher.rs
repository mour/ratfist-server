use crate::db::models::Node;
use crate::db::DbConnPool;

use crate::meteo::messages::{transfer, IncomingMessage, OutgoingMessage};
use crate::meteo::models::{Sensor, SensorTypeEnum};
use crate::meteo::MeteoError;

use diesel::insert_into;
use diesel::prelude::*;

use log::warn;

use crate::comm::CommState;

use crate::utils::DateTimeUtc;

pub fn fetcher_iteration(
    db_conn_pool: &DbConnPool,
    comm_state: &CommState,
) -> Result<(), MeteoError> {
    let db = db_conn_pool.get().map_err(|_| MeteoError)?;

    // Get all sensors
    let sensors = {
        use crate::db::schema::*;
        use crate::meteo::schema::*;

        sensors::table
            .inner_join(nodes::table)
            .load::<(Sensor, Node)>(&db)
    }
    .map_err(|_| MeteoError)?;

    let curr_time = DateTimeUtc::now();

    for (ref sensor, ref node) in &sensors {
        // Send message querying each sensor
        let sens_id = sensor.public_id as u32;

        let outgoing_msg = match sensor.sensor_type {
            SensorTypeEnum::Pressure => OutgoingMessage::GetPressure(sens_id),
            SensorTypeEnum::Temperature => OutgoingMessage::GetTemperature(sens_id),
            SensorTypeEnum::Humidity => OutgoingMessage::GetHumidity(sens_id),
            SensorTypeEnum::LightLevel => OutgoingMessage::GetLightLevel(sens_id),
        };

        let channel = comm_state
            .get_comm_channel(node.public_id as u32)
            .map_err(|_| MeteoError)?;

        let measured_val = match transfer(&channel, outgoing_msg) {
            Ok(IncomingMessage::Pressure(id, val))
                if id == sens_id && sensor.sensor_type == SensorTypeEnum::Pressure =>
            {
                val
            }
            Ok(IncomingMessage::Temperature(id, val))
                if id == sens_id && sensor.sensor_type == SensorTypeEnum::Temperature =>
            {
                val
            }
            Ok(IncomingMessage::Humidity(id, val))
                if id == sens_id && sensor.sensor_type == SensorTypeEnum::Humidity =>
            {
                val
            }
            Ok(IncomingMessage::LightLevel(id, val))
                if id == sens_id && sensor.sensor_type == SensorTypeEnum::LightLevel =>
            {
                val
            }
            Ok(msg) => {
                warn!("Unexpected reply message: {:?}", msg);
                continue;
            }
            Err(e) => {
                warn!("Communication error: {:?}", e);
                continue;
            }
        };

        // Push to db (use same timestamp for all values)
        {
            use crate::meteo::schema::measurements::dsl::*;

            if insert_into(measurements)
                .values((
                    sensor_id.eq(sensor.id),
                    value.eq(measured_val),
                    measured_at.eq(&curr_time),
                ))
                .execute(&db)
                .is_err()
            {
                warn!(
                    "Error while inserting measurement: (id {}, value {}, measured_at {:?})",
                    sensor.id, measured_val, curr_time
                );
            }
        }
    }

    Ok(())
}
