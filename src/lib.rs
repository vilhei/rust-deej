#![no_std]

use embedded_hal::adc::Channel;
use enum_dispatch::enum_dispatch;
use esp_hal::{
    adc::{AdcCalCurve, AdcCalScheme, AdcPin, ADC},
    gpio::{Analog, GpioPin},
    peripherals::{ADC1, ADC2},
    prelude::*,
};

#[enum_dispatch]
pub trait ReadAnalog {
    fn read(&mut self, adc: &mut ADC<ADC1>) -> u16;
}

/// Allows storage for all implemented analog pins. Currently **only** supports ADC1 pins.
///
/// Regardless of the enum variant actually stored the analog input value can be read by using [AnyAnalogPin]
#[enum_dispatch(ReadAnalog)]
pub enum AnyAnalogPin {
    AO(AdcPin<GpioPin<Analog, 0>, ADC1, AdcCalCurve<ADC1>>),
    A1(AdcPin<GpioPin<Analog, 1>, ADC1, AdcCalCurve<ADC1>>),
    A2(AdcPin<GpioPin<Analog, 2>, ADC1, AdcCalCurve<ADC1>>),
    A3(AdcPin<GpioPin<Analog, 3>, ADC1, AdcCalCurve<ADC1>>),
    A4(AdcPin<GpioPin<Analog, 4>, ADC1, AdcCalCurve<ADC1>>),
}

impl<T, Cal> ReadAnalog for AdcPin<T, ADC1, Cal>
where
    T: Channel<ADC1, ID = u8>,
    Cal: AdcCalScheme<ADC1>,
{
    fn read(&mut self, adc: &mut ADC<ADC1>) -> u16 {
        nb::block!(adc.read(self)).expect("Failed to read analog value")
    }
}

pub fn scale_analog_input_to_1024(value: u16) -> u16 {
    scale_to_range(value, 0, 770, 0, 1024)
}

pub fn scale_to_range(value: u16, old_min: u16, old_max: u16, new_min: u16, new_max: u16) -> u16 {
    let old_range = old_max - old_min;
    let new_range = new_max - new_min;
    let value = value.min(old_max); // To ensure that the provided value is not larger than original max to prevent overflow
    ((value as u32 - old_min as u32) * new_range as u32 / old_range as u32 + new_min as u32) as u16
}
