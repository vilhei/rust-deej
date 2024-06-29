#![no_std]
#![no_main]
#![feature(generic_arg_infer)]

#[rtic::app(device=esp32c3, dispatchers = [FROM_CPU_INTR0])]
mod app {

    use esp_backtrace as _; // Panic handling
    use esp_hal::{
        adc::{AdcConfig, Attenuation, ADC},
        clock::ClockControl,
        i2c::I2C,
        peripherals::{Peripherals, ADC1, TIMG0, TIMG1},
        prelude::*,
        timer::{Timer0, TimerGroup},
        Delay, Timer, IO,
    };
    use esp_println::println;

    use rust_deej::{
        globals::{INPUT_COUNT, SERIAL_UPDATE_PERIOD},
        scale_analog_input_to_100, scale_analog_input_to_1023, AnyAnalogPin, DisplayState,
        DisplayStatus, ReadAnalog,
    };
    use ssd1306::{
        prelude::{DisplaySize128x64, *},
        I2CDisplayInterface, Ssd1306,
    };

    #[shared]
    struct Shared {
        raw_input_values: [u16; INPUT_COUNT],
        display: DisplayState<'static>,
        display_on_time: u32,
        timer0: Timer<Timer0<TIMG0>>,
    }

    #[local]
    struct Local {
        adc: ADC<'static, ADC1>,
        pots: [AnyAnalogPin; INPUT_COUNT],
        delay: Delay,
        timer1: Timer<Timer0<TIMG1>>,
    }

    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        let peripherals = Peripherals::take();
        let system = peripherals.SYSTEM.split();
        let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

        let mut adc_config = AdcConfig::new();

        let pot0 = adc_config
            .enable_pin_with_cal(io.pins.gpio0.into_analog(), Attenuation::Attenuation0dB);
        let pot1 = adc_config
            .enable_pin_with_cal(io.pins.gpio1.into_analog(), Attenuation::Attenuation0dB);
        let pot2 = adc_config
            .enable_pin_with_cal(io.pins.gpio2.into_analog(), Attenuation::Attenuation0dB);
        let pot3 = adc_config
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

        let display_on_time: u32 = 10;

        let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
        let mut timer0 = timer_group0.timer0;
        timer0.listen();

        let timer_group1 = TimerGroup::new(peripherals.TIMG1, &clocks);
        let mut timer1 = timer_group1.timer0;
        timer1.listen();
        timer1.start(SERIAL_UPDATE_PERIOD.millis());

        let mut display_state = DisplayState::new(display);
        display_state.set_title("Volumes");
        display_state.ready();

        (
            Shared {
                raw_input_values: Default::default(),
                display: display_state,
                display_on_time,
                timer0,
            },
            Local {
                adc,
                pots,
                delay,
                timer1,
            },
        )
    }

    #[idle (shared = [raw_input_values, display], local=[adc,pots, delay])]
    fn idle(cx: idle::Context) -> ! {
        let idle::LocalResources {
            adc, pots, delay, ..
        } = cx.local;

        let idle::SharedResources {
            mut raw_input_values,
            mut display,
            ..
        } = cx.shared;

        let mut volumes = [0; INPUT_COUNT];
        loop {
            for (idx, input) in pots.iter_mut().enumerate() {
                let new_val = input.read_multi_sample(adc, 128);
                raw_input_values.lock(|r| r[idx] = new_val);
                volumes[idx] = scale_analog_input_to_100(new_val);
            }

            let display_changed = display.lock(|d| d.set_volumes(&volumes));
            match display_changed {
                DisplayStatus::Changed => update_display::spawn().unwrap(),
                DisplayStatus::NotChanged => (),
            };

            delay.delay_ms(50u32);
        }
    }

    #[task(priority=2, shared=[display, timer0, &display_on_time])]
    async fn update_display(cx: update_display::Context) {
        let update_display::SharedResources {
            mut display,
            mut timer0,
            display_on_time,
            ..
        } = cx.shared;

        display.lock(|d| d.draw()).unwrap();
        timer0.lock(|t| t.start(display_on_time.secs()));
    }

    /// Turn the display off after the timer has expired
    #[task(binds=TG0_T0_LEVEL,shared=[display, timer0] )]
    fn turn_display_off(mut cx: turn_display_off::Context) {
        cx.shared.timer0.lock(|t| t.clear_interrupt());
        cx.shared.display.lock(|d| d.turn_off());
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
