#![no_std]
#![no_main]

use esp_backtrace as _; // Panic handling

#[rtic::app(device=esp32c3, dispatchers = [FROM_CPU_INTR0])]
mod app {
    use esp_hal::{
        adc::{AdcCalCurve, AdcConfig, AdcPin, Attenuation, ADC},
        clock::ClockControl,
        gpio::{Analog, AnyPin, GpioPin},
        peripherals::{Peripherals, ADC1},
        prelude::*,
        Delay, IO,
    };
    use esp_println::println;

    use crate::{scale_analog_input_to_1024, scale_to_range};
    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        adc: ADC<'static, ADC1>,
        pot0: AdcPin<GpioPin<Analog, 0>, ADC1, AdcCalCurve<ADC1>>,
        delay: Delay,
    }

    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        let peripherals = Peripherals::take();
        let system = peripherals.SYSTEM.split();
        let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

        let pot_pin = io.pins.gpio0.into_analog();

        let mut adc_config = AdcConfig::new();

        // let mut pot_in = adc_config.enable_pin(pot_pin, Attenuation::Attenuation0dB);
        let pot_in: AdcPin<GpioPin<Analog, 0>, ADC1, AdcCalCurve<ADC1>> =
            adc_config.enable_pin_with_cal(pot_pin, Attenuation::Attenuation0dB);
        let adc = ADC::new(peripherals.ADC1, adc_config);

        let clocks = ClockControl::max(system.clock_control).freeze();
        let delay = Delay::new(&clocks);

        println!("Hello world!");
        (
            Shared {},
            Local {
                adc,
                pot0: pot_in,
                delay,
            },
        )
    }

    #[idle ( local=[adc, pot0, delay])]
    fn idle(cx: idle::Context) -> ! {
        let adc = cx.local.adc;
        loop {
            let v = nb::block!(adc.read(cx.local.pot0)).unwrap();
            let s = scale_analog_input_to_1024(v);
            let s2 = scale_to_range(v, 0, 780, 0, 100);

            println!("{} - {} - {}\r", v, s, s2);

            cx.local.delay.delay_ms(200u32);
        }
    }
}

fn scale_analog_input_to_1024(value: u16) -> u16 {
    scale_to_range(value, 0, 780, 0, 1024)
}

fn scale_to_range(value: u16, old_min: u16, old_max: u16, new_min: u16, new_max: u16) -> u16 {
    let old_range = old_max - old_min;
    let new_range = new_max - new_min;
    let value = value.min(old_max); // To ensure that the provided value is not larger than original max to prevent overflow
    ((value as u32 - old_min as u32) * new_range as u32 / old_range as u32 + new_min as u32) as u16
}
