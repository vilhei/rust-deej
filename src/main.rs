#![no_std]
#![no_main]
#![feature(generic_arg_infer)]

const INPUT_COUNT: usize = 4;

#[rtic::app(device=esp32c3, dispatchers = [FROM_CPU_INTR0])]
mod app {
    use crate::INPUT_COUNT;
    use esp_backtrace as _; // Panic handling
    use esp_hal::{
        adc::{AdcCalCurve, AdcConfig, AdcPin, Attenuation, ADC},
        clock::ClockControl,
        gpio::{Analog, GpioPin},
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

    use rust_deej::{scale_analog_input_to_1024, scale_to_range, AnyAnalogPin, ReadAnalog};
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
        pots: [AnyAnalogPin; INPUT_COUNT],
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

        let pot0: AdcPin<GpioPin<Analog, 0>, ADC1, AdcCalCurve<ADC1>> = adc_config
            .enable_pin_with_cal(io.pins.gpio0.into_analog(), Attenuation::Attenuation0dB);
        let pot1: AdcPin<GpioPin<Analog, 1>, ADC1, AdcCalCurve<ADC1>> = adc_config
            .enable_pin_with_cal(io.pins.gpio1.into_analog(), Attenuation::Attenuation0dB);
        let pot2: AdcPin<GpioPin<Analog, 2>, ADC1, AdcCalCurve<ADC1>> = adc_config
            .enable_pin_with_cal(io.pins.gpio2.into_analog(), Attenuation::Attenuation0dB);
        let pot3: AdcPin<_, ADC1, AdcCalCurve<ADC1>> = adc_config
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

        let pots = [
            AnyAnalogPin::from(pot0),
            AnyAnalogPin::from(pot1),
            AnyAnalogPin::from(pot2),
            AnyAnalogPin::from(pot3),
        ];

        (
            Shared {},
            Local {
                adc,
                pots,
                delay,
                display,
            },
        )
    }

    #[idle ( local=[adc,pots, delay, display])]
    fn idle(cx: idle::Context) -> ! {
        let idle::LocalResources {
            adc,
            pots,
            delay,
            display,
            ..
        } = cx.local;

        const BUF_SIZE: usize = 32;
        let mut s_buf: String<BUF_SIZE> = String::new();

        loop {
            display.clear(BinaryColor::Off).unwrap();

            let v = pots[0].read(adc);
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

            let v = pots[1].read(adc);
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

            let v = pots[2].read(adc);
            let s = scale_analog_input_to_1024(v);
            let s2 = scale_to_range(v, 0, 770, 0, 100);
            println!("pot2: {} - {} - {}\r", v, s, s2);

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

            let v = pots[3].read(adc);
            let s = scale_analog_input_to_1024(v);
            let s2 = scale_to_range(v, 0, 770, 0, 100);
            println!("pot3: {} - {} - {}\r", v, s, s2);

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
