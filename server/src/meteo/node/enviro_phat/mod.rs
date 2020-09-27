use super::SensorNode;

use crate::meteo::models::SensorTypeEnum;
use crate::meteo::MeteoError;

use crate::comm;

mod bmp280;
mod tcs3472;

use bmp280::{Bmp280, IIRCoeficient, Mode, Oversampling, StandbyTime};

pub struct EnviroPHat {
    bmp: Bmp280,
}

impl EnviroPHat {
    pub fn new(i2c_comm_path_id: u32) -> EnviroPHat {
        let comm_path = comm::get_i2c_comm_path(i2c_comm_path_id);

        let bmp = bmp280::Bmp280::new(
            comm_path,
            StandbyTime::Time1000ms,
            IIRCoeficient::Mult4X,
            Oversampling::Mult16X,
            Oversampling::Mult2X,
            Mode::Normal,
        )
        .unwrap();

        EnviroPHat { bmp }
    }
}

impl SensorNode for EnviroPHat {
    fn measure(&self, measurement_type: SensorTypeEnum, sensor_id: u32) -> Result<f32, MeteoError> {
        if sensor_id != 0 {
            return Err(MeteoError);
        }

        match measurement_type {
            SensorTypeEnum::Pressure => Ok(self.bmp.query_press_and_temp()?.0),
            SensorTypeEnum::Temperature => Ok(self.bmp.query_press_and_temp()?.1),
            SensorTypeEnum::Humidity => Err(MeteoError),
            SensorTypeEnum::LightLevel => Err(MeteoError),
        }
    }
}