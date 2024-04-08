#![no_std]
#![no_main]

#[rtic::app(device=esp32c3, dispatchers = [FROM_CPU_INTR0])]
mod app {
    use crate::{scale_analog_input_to_1024, scale_to_range};
    use esp_backtrace as _; // Panic handling
    use esp_hal::{
        adc::{AdcCalCurve, AdcConfig, AdcPin, Attenuation, ADC},
        clock::ClockControl,
        gpio::{Analog, AnyPin, GpioPin},
        i2c::I2C,
        peripherals::{Peripherals, ADC1, I2C0},
        prelude::*,
        Delay, IO,
    };
    use esp_println::println;

    use embedded_graphics::{
        geometry::AnchorPoint,
        mono_font::{
            ascii::{FONT_5X8, FONT_6X10, FONT_9X18_BOLD},
            MonoTextStyle, MonoTextStyleBuilder,
        },
        pixelcolor::BinaryColor,
        prelude::*,
        text::{Alignment, Text},
    };

    use ssd1306::{
        mode::BufferedGraphicsMode,
        prelude::{DisplaySize128x64, I2CInterface, *},
        I2CDisplayInterface, Ssd1306,
    };

    use heapless::String;

    use core::fmt::Write;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        adc: ADC<'static, ADC1>,
        pot0: AdcPin<GpioPin<Analog, 0>, ADC1, AdcCalCurve<ADC1>>,
        pot1: AdcPin<GpioPin<Analog, 1>, ADC1, AdcCalCurve<ADC1>>,
        pot2: AdcPin<GpioPin<Analog, 2>, ADC1, AdcCalCurve<ADC1>>,
        pot3: AdcPin<GpioPin<Analog, 3>, ADC1, AdcCalCurve<ADC1>>,
        delay: Delay,
        display: Ssd1306<
            I2CInterface<I2C<'static, I2C0>>,
            DisplaySize128x64,
            BufferedGraphicsMode<DisplaySize128x64>,
        >,
    }

    const TEXT_STYLE: MonoTextStyle<'static, BinaryColor> = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        let peripherals = Peripherals::take();
        let system = peripherals.SYSTEM.split();
        let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

        let mut adc_config = AdcConfig::new();

        // let mut pot_in = adc_config.enable_pin(pot_pin, Attenuation::Attenuation0dB);
        let pot0: AdcPin<GpioPin<Analog, 0>, ADC1, AdcCalCurve<ADC1>> = adc_config
            .enable_pin_with_cal(io.pins.gpio0.into_analog(), Attenuation::Attenuation0dB);

        let pot1: AdcPin<GpioPin<Analog, 1>, ADC1, AdcCalCurve<ADC1>> = adc_config
            .enable_pin_with_cal(io.pins.gpio1.into_analog(), Attenuation::Attenuation0dB);

        let pot2: AdcPin<GpioPin<Analog, 2>, ADC1, AdcCalCurve<ADC1>> = adc_config
            .enable_pin_with_cal(io.pins.gpio2.into_analog(), Attenuation::Attenuation0dB);

        let pot3: AdcPin<GpioPin<Analog, 3>, ADC1, AdcCalCurve<ADC1>> = adc_config
            .enable_pin_with_cal(io.pins.gpio3.into_analog(), Attenuation::Attenuation0dB);

        let adc = ADC::new(peripherals.ADC1, adc_config);

        let clocks = ClockControl::max(system.clock_control).freeze();
        let delay = Delay::new(&clocks);

        let i2c = I2C::new(
            peripherals.I2C0,
            io.pins.gpio6,
            io.pins.gpio7,
            100u32.kHz(),
            &clocks,
        );

        let interface = I2CDisplayInterface::new(i2c);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display.init().unwrap();

        println!("Hello world!");
        (
            Shared {},
            Local {
                adc,
                pot0,
                pot1,
                pot2,
                pot3,
                delay,
                display,
            },
        )
    }

    #[idle ( local=[adc, pot0, pot1, pot2, pot3, delay, display])]
    fn idle(cx: idle::Context) -> ! {
        let idle::LocalResources {
            adc,
            pot0,
            pot1,
            pot2,
            pot3,
            delay,
            display,
            ..
        } = cx.local;

        const BUF_SIZE: usize = 32;
        let mut s_buf: String<BUF_SIZE> = String::new();

        loop {
            display.clear(BinaryColor::Off).unwrap();

            let v = nb::block!(adc.read(pot0)).unwrap();
            let s = scale_analog_input_to_1024(v);
            let s2 = scale_to_range(v, 0, 770, 0, 100);

            s_buf.clear();
            write!(s_buf, "0: {}", s2).expect("Format string failed, probably too small buffer");
            Text::with_alignment(
                &s_buf,
                display.bounding_box().anchor_point(AnchorPoint::TopLeft) + Point::new(2, 10),
                TEXT_STYLE,
                Alignment::Left,
            )
            .draw(display)
            .unwrap();

            println!("pot0: {} - {} - {}\r", v, s, s2);

            let v = nb::block!(adc.read(pot1)).unwrap();
            let s = scale_analog_input_to_1024(v);
            let s2 = scale_to_range(v, 0, 770, 0, 100);
            println!("pot1: {} - {} - {}\r", v, s, s2);

            s_buf.clear();
            write!(s_buf, "1: {}", s2).expect("Format string failed, probably too small buffer");

            Text::with_alignment(
                &s_buf,
                display.bounding_box().anchor_point(AnchorPoint::TopLeft) + Point::new(2, 22),
                TEXT_STYLE,
                Alignment::Left,
            )
            .draw(display)
            .unwrap();

            let v = nb::block!(adc.read(pot2)).unwrap();
            let s = scale_analog_input_to_1024(v);
            let s2 = scale_to_range(v, 0, 770, 0, 100);
            println!("pot1: {} - {} - {}\r", v, s, s2);

            s_buf.clear();
            write!(s_buf, "2: {}", s2).expect("Format string failed, probably too small buffer");

            Text::with_alignment(
                &s_buf,
                display.bounding_box().anchor_point(AnchorPoint::TopLeft) + Point::new(2, 34),
                TEXT_STYLE,
                Alignment::Left,
            )
            .draw(display)
            .unwrap();

            let v = nb::block!(adc.read(pot3)).unwrap();
            let s = scale_analog_input_to_1024(v);
            let s2 = scale_to_range(v, 0, 770, 0, 100);
            println!("pot1: {} - {} - {}\r", v, s, s2);

            s_buf.clear();
            write!(s_buf, "3: {}", s2).expect("Format string failed, probably too small buffer");

            Text::with_alignment(
                &s_buf,
                display.bounding_box().anchor_point(AnchorPoint::TopLeft) + Point::new(2, 46),
                TEXT_STYLE,
                Alignment::Left,
            )
            .draw(display)
            .unwrap();

            display.flush().unwrap();

            delay.delay_ms(500u32);
        }
    }
}

fn scale_analog_input_to_1024(value: u16) -> u16 {
    scale_to_range(value, 0, 770, 0, 1024)
}

fn scale_to_range(value: u16, old_min: u16, old_max: u16, new_min: u16, new_max: u16) -> u16 {
    let old_range = old_max - old_min;
    let new_range = new_max - new_min;
    let value = value.min(old_max); // To ensure that the provided value is not larger than original max to prevent overflow
    ((value as u32 - old_min as u32) * new_range as u32 / old_range as u32 + new_min as u32) as u16
}
