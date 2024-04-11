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
        peripherals::{Peripherals, ADC1, I2C0, TIMG0, TIMG1},
        prelude::*,
        timer::{Timer0, TimerGroup},
        Delay, Timer, IO,
    };
    use esp_println::println;

    use embedded_graphics::{
        geometry::AnchorPoint,
        pixelcolor::BinaryColor,
        prelude::*,
        primitives::Rectangle,
        text::{Alignment, Text},
    };

    use rust_deej::{
        scale_analog_input_to_1023, scale_to_range, AnyAnalogPin, ReadAnalog,
        DISPLAY_UPDATE_PERIOD, FILL_RECT_STYLE, MAX_ANALOG_VALUE, OUTER_RECT_STYLE,
        SERIAL_UPDATE_PERIOD, TEXT_STYLE, TEXT_STYLE_BOLD,
    };
    use ssd1306::{
        mode::BufferedGraphicsMode,
        prelude::{DisplaySize128x64, I2CInterface, *},
        I2CDisplayInterface, Ssd1306,
    };

    use heapless::String;

    use core::fmt::Write;

    #[shared]
    struct Shared {
        raw_input_values: [u16; INPUT_COUNT],
    }

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
        timer0: Timer<Timer0<TIMG0>>,
        timer1: Timer<Timer0<TIMG1>>,
    }

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

        let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
        let mut timer0 = timer_group0.timer0;
        timer0.listen();
        timer0.start(DISPLAY_UPDATE_PERIOD.millis());

        let timer_group1 = TimerGroup::new(peripherals.TIMG1, &clocks);
        let mut timer1 = timer_group1.timer0;
        timer1.listen();
        timer1.start(SERIAL_UPDATE_PERIOD.millis());

        (
            Shared {
                raw_input_values: Default::default(),
            },
            Local {
                adc,
                pots,
                delay,
                display,
                timer0,
                timer1,
            },
        )
    }

    #[idle (shared = [raw_input_values], local=[adc,pots, delay])]
    fn idle(cx: idle::Context) -> ! {
        let idle::LocalResources {
            adc, pots, delay, ..
        } = cx.local;
        let mut raw_input_values = cx.shared.raw_input_values;

        loop {
            for (idx, input) in pots.iter_mut().enumerate() {
                let new_val = input.read_multi_sample(adc, 100);
                raw_input_values.lock(|r| r[idx] = new_val);
            }
            delay.delay_ms(50u32);
        }
    }

    #[task(binds=TG0_T0_LEVEL,shared=[raw_input_values], local = [timer0, display])]
    fn update_display(mut cx: update_display::Context) {
        let update_display::LocalResources {
            display, timer0, ..
        } = cx.local;

        timer0.clear_interrupt();

        let mut percentages: [u16; INPUT_COUNT] = Default::default();

        cx.shared.raw_input_values.lock(|r| {
            r.iter().enumerate().for_each(|(idx, val)| {
                percentages[idx] = scale_to_range(*val, 0, MAX_ANALOG_VALUE, 0, 100)
            })
        });

        let line_spacing = 12;
        let mut s_buf: String<32> = String::new();

        let vol_value_y_offset = 22;
        let vol_bar_x_offset = 45;
        let vol_bar_height = 7;
        let vol_bar_width = 80;

        display.clear(BinaryColor::Off).unwrap();

        Text::with_alignment(
            "Volume control",
            display.bounding_box().anchor_point(AnchorPoint::TopCenter) + Point::new(0, 8),
            TEXT_STYLE_BOLD,
            Alignment::Center,
        )
        .draw(display)
        .unwrap();

        for (idx, p_val) in percentages.iter().enumerate() {
            s_buf.clear();
            write!(s_buf, "{}: {}", idx, p_val).expect("Format string failed, check buffer size");

            Text::with_alignment(
                &s_buf,
                display.bounding_box().anchor_point(AnchorPoint::TopLeft)
                    + Point::new(2, vol_value_y_offset + line_spacing * idx as i32),
                TEXT_STYLE,
                Alignment::Left,
            )
            .draw(display)
            .unwrap();

            Rectangle::new(
                display.bounding_box().anchor_point(AnchorPoint::TopLeft)
                    + Point::new(
                        vol_bar_x_offset,
                        vol_value_y_offset - vol_bar_height + line_spacing * idx as i32,
                    ),
                Size::new(vol_bar_width, vol_bar_height as u32),
            )
            .into_styled(OUTER_RECT_STYLE)
            .draw(display)
            .unwrap();

            let fill_val = scale_to_range(*p_val, 0, 100, 0, vol_bar_width as u16);

            Rectangle::new(
                display.bounding_box().anchor_point(AnchorPoint::TopLeft)
                    + Point::new(
                        vol_bar_x_offset,
                        vol_value_y_offset - vol_bar_height + line_spacing * idx as i32,
                    ),
                Size::new(fill_val as u32, vol_bar_height as u32),
            )
            .into_styled(FILL_RECT_STYLE)
            .draw(display)
            .unwrap();
        }

        display.flush().unwrap();

        timer0.start(DISPLAY_UPDATE_PERIOD.millis())
    }

    #[task(binds=TG1_T0_LEVEL,shared =[raw_input_values], local=[timer1])]
    fn send_to_serial(mut cx: send_to_serial::Context) {
        cx.local.timer1.clear_interrupt();

        let mut values: [u16; INPUT_COUNT] = Default::default();

        cx.shared.raw_input_values.lock(|r| {
            r.iter()
                .enumerate()
                .for_each(|(idx, val)| values[idx] = scale_analog_input_to_1023(*val))
        });
        println!("{}|{}|{}|{}\r", values[0], values[1], values[2], values[3]);
        cx.local.timer1.start(SERIAL_UPDATE_PERIOD.millis())
    }
}
