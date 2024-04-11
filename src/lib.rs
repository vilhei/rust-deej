#![no_std]

use embedded_graphics::{
    mono_font::{
        ascii::{FONT_6X10, FONT_6X12, FONT_8X13},
        MonoTextStyle, MonoTextStyleBuilder,
    },
    pixelcolor::BinaryColor,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder},
};
use embedded_hal::adc::Channel;
use enum_dispatch::enum_dispatch;
use esp_hal::{
    adc::{AdcCalCurve, AdcCalScheme, AdcPin, ADC},
    gpio::{Analog, GpioPin},
    peripherals::ADC1,
    prelude::*,
};

pub const DISPLAY_UPDATE_PERIOD: u32 = 50;
pub const SERIAL_UPDATE_PERIOD: u32 = 50;

pub const MAX_ANALOG_VALUE: u16 = 750;

pub const TEXT_STYLE: MonoTextStyle<'static, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_6X10)
    .text_color(BinaryColor::On)
    .build();

pub const TEXT_STYLE_BOLD: MonoTextStyle<'static, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_8X13)
    .text_color(BinaryColor::On)
    .build();

pub const OUTER_RECT_STYLE: PrimitiveStyle<BinaryColor> = PrimitiveStyleBuilder::new()
    .stroke_color(BinaryColor::On)
    .stroke_width(1)
    .fill_color(BinaryColor::Off)
    .build();

pub const FILL_RECT_STYLE: PrimitiveStyle<BinaryColor> = PrimitiveStyleBuilder::new()
    .fill_color(BinaryColor::On)
    .build();

#[enum_dispatch]
pub trait ReadAnalog {
    fn read(&mut self, adc: &mut ADC<ADC1>) -> u16;
    fn read_multi_sample(&mut self, adc: &mut ADC<ADC1>, sample_size: u32) -> u16;
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

    fn read_multi_sample(&mut self, adc: &mut ADC<ADC1>, sample_size: u32) -> u16 {
        let mut sum = 0u32;
        for _ in 0..sample_size {
            sum += nb::block!(adc.read(self)).expect("Failed to read analog value") as u32;
        }
        (sum / sample_size) as u16 // adc.read returns u16 so the average of u16 should never be larger than u16 --> no overflow
    }
}

pub fn scale_analog_input_to_1023(value: u16) -> u16 {
    scale_to_range(value, 0, MAX_ANALOG_VALUE, 0, 1023)
}

pub fn scale_to_range(value: u16, old_min: u16, old_max: u16, new_min: u16, new_max: u16) -> u16 {
    let old_range = old_max - old_min;
    let new_range = new_max - new_min;
    let value = value.min(old_max); // To ensure that the provided value is not larger than original max to prevent overflow
    ((value as u32 - old_min as u32) * new_range as u32 / old_range as u32 + new_min as u32) as u16
}
