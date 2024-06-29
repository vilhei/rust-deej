#![no_std]

pub mod globals;
pub mod style;

use core::fmt::Write;
use embedded_graphics::{
    geometry::AnchorPoint,
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::Rectangle,
    text::{Alignment, Text},
};
use embedded_hal_027::adc::Channel;
use enum_dispatch::enum_dispatch;
use esp_hal::{
    adc::{AdcCalCurve, AdcCalScheme, AdcPin, ADC},
    gpio::{Analog, GpioPin},
    i2c::I2C,
    peripherals::{ADC1, I2C0},
    prelude::*,
};
use globals::{INPUT_COUNT, MAX_ANALOG_VALUE, ZERO_CUTOFF};
use heapless::String;
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, Ssd1306};
use style::{FILL_RECT_STYLE, OUTER_RECT_STYLE, TEXT_STYLE, TEXT_STYLE_BOLD};

pub type Ssd1306Display = Ssd1306<
    I2CInterface<I2C<'static, I2C0>>,
    DisplaySize128x64,
    BufferedGraphicsMode<DisplaySize128x64>,
>;

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
        match nb::block!(adc.read(self)).expect("Failed to read analog value") {
            x if x < ZERO_CUTOFF => 0,
            x => x,
        }
    }

    fn read_multi_sample(&mut self, adc: &mut ADC<ADC1>, sample_size: u32) -> u16 {
        let mut sum = 0u32;
        for _ in 0..sample_size {
            sum += match nb::block!(adc.read(self)).expect("Failed to read analog value") {
                x if x < ZERO_CUTOFF => 0u32,
                x => x as u32,
            };
        }
        (sum / sample_size) as u16 // adc.read returns u16 so the average of u16 should never be larger than u16 --> no overflow
    }
}

pub fn scale_analog_input_to_1023(value: u16) -> u16 {
    scale_to_range(value, 0, MAX_ANALOG_VALUE, 0, 1023)
}

pub fn scale_analog_input_to_100(value: u16) -> u16 {
    scale_to_range(value, 0, MAX_ANALOG_VALUE, 0, 100)
}

pub fn scale_to_range(value: u16, old_min: u16, old_max: u16, new_min: u16, new_max: u16) -> u16 {
    let old_range = old_max - old_min;
    let new_range = new_max - new_min;
    let value = value.min(old_max); // To ensure that the provided value is not larger than original max to prevent overflow

    ((value as u32 - old_min as u32) * new_range as u32 / old_range as u32 + new_min as u32) as u16
}

pub enum DisplayStatus {
    Changed,
    NotChanged,
}

pub struct DisplayState<'a> {
    display: Ssd1306Display,
    title: Option<&'a str>,
    title_position: Point,
    volumes: [u16; INPUT_COUNT],
    ready_to_draw: bool,
    vol_value_y_offset: i32,
    line_spacing: i32,
    vol_bar_x_offset: i32,
    vol_bar_height: u32,
    vol_bar_width: u32,
    vol_bar_size: Size,
    top_left_point: Point,
}

impl<'a> DisplayState<'a> {
    pub fn new(display: Ssd1306Display) -> Self {
        let vol_bar_height = 7;
        let vol_bar_width = 80;
        Self {
            top_left_point: display.bounding_box().anchor_point(AnchorPoint::TopLeft),
            title_position: display.bounding_box().anchor_point(AnchorPoint::TopCenter)
                + Point::new(0, 8),
            display,
            volumes: Default::default(),
            ready_to_draw: false,
            title: None,
            vol_value_y_offset: 22,
            line_spacing: 12,
            vol_bar_x_offset: 45,
            vol_bar_height,
            vol_bar_width,
            vol_bar_size: Size::new(vol_bar_width, vol_bar_height),
        }
    }

    pub fn with_volumes(display: Ssd1306Display, volumes: [u16; INPUT_COUNT]) -> Self {
        Self {
            volumes,
            ..Self::new(display)
        }
    }

    /// Needs to be called to actually draw anything on the screen.
    pub fn ready(&mut self) {
        self.ready_to_draw = true;
    }

    pub fn set_title(&mut self, title: &'a str) {
        self.title = Some(title);
    }

    pub fn disable_title(&mut self) {
        self.title = None;
    }

    /// Give volumes in range 0-100
    pub fn set_volumes(&mut self, volumes: &[u16; INPUT_COUNT]) -> DisplayStatus {
        let mut changed = false;

        for (idx, vol) in volumes.iter().enumerate() {
            if vol.abs_diff(self.volumes[idx]) > 1 {
                self.volumes[idx] = *vol;
                changed = true;
            }
        }

        if changed {
            return DisplayStatus::Changed;
        }
        DisplayStatus::NotChanged
    }

    #[allow(clippy::result_unit_err)]
    pub fn draw(&mut self) -> Result<(), ()> {
        if !self.ready_to_draw {
            return Err(());
        }

        // esp_println::println!("Drawing");
        self.turn_on();
        self.display.clear(BinaryColor::Off).unwrap(); // TODO propagate error?

        if let Some(title) = self.title {
            Text::with_alignment(
                title,
                self.title_position,
                TEXT_STYLE_BOLD,
                Alignment::Center,
            )
            .draw(&mut self.display)
            .unwrap();
        }
        let mut s_buf: String<32> = String::new();

        for (idx, p_val) in self.volumes.iter().enumerate() {
            s_buf.clear();
            write!(s_buf, "{}: {}", idx, p_val).expect("Format string failed, check buffer size");

            Text::with_alignment(
                &s_buf,
                self.top_left_point
                    + Point::new(2, self.vol_value_y_offset + self.line_spacing * idx as i32),
                TEXT_STYLE,
                Alignment::Left,
            )
            .draw(&mut self.display)
            .unwrap();

            let mut b = Rectangle::new(
                self.top_left_point
                    + Point::new(
                        self.vol_bar_x_offset,
                        self.vol_value_y_offset - self.vol_bar_height as i32
                            + self.line_spacing * idx as i32,
                    ),
                self.vol_bar_size,
            )
            .into_styled(OUTER_RECT_STYLE);

            b.primitive = Rectangle::new(
                self.top_left_point
                    + Point::new(
                        self.vol_bar_x_offset,
                        self.vol_value_y_offset - self.vol_bar_height as i32
                            + self.line_spacing * idx as i32,
                    ),
                self.vol_bar_size,
            );

            Rectangle::new(
                self.top_left_point
                    + Point::new(
                        self.vol_bar_x_offset,
                        self.vol_value_y_offset - self.vol_bar_height as i32
                            + self.line_spacing * idx as i32,
                    ),
                self.vol_bar_size,
            )
            .into_styled(OUTER_RECT_STYLE)
            .draw(&mut self.display)
            .unwrap();

            let fill_val = scale_to_range(*p_val, 0, 100, 0, self.vol_bar_width as u16);

            Rectangle::new(
                self.top_left_point
                    + Point::new(
                        self.vol_bar_x_offset,
                        self.vol_value_y_offset - self.vol_bar_height as i32
                            + self.line_spacing * idx as i32,
                    ),
                Size::new(fill_val as u32, self.vol_bar_height),
            )
            .into_styled(FILL_RECT_STYLE)
            .draw(&mut self.display)
            .unwrap();
        }
        self.display.flush().unwrap(); // TODO propagate error?
        Ok(())
    }

    pub fn turn_off(&mut self) {
        self.display.set_display_on(false).unwrap(); // TODO propagate error?
    }

    pub fn turn_on(&mut self) {
        self.display.set_display_on(true).unwrap(); // TODO propagate error?
    }
}
