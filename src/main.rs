#![no_std]
#![no_main]

use core::sync::atomic::Ordering;

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{AnyPin, Input, Pull},
    pwm::{Config, Pwm, SetDutyCycle},
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use portable_atomic::AtomicU8;
use {defmt_rtt as _, panic_probe as _};

type PwmType = Mutex<ThreadModeRawMutex, Option<Pwm<'static>>>;
static PWM: PwmType = Mutex::new(None);

const DUTY_START: u8 = 0; // percent
const DUTY_STEP: u8 = 10; // percent

static DUTY_CYCLE: AtomicU8 = AtomicU8::new(DUTY_START);

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Note: Pin 18 = Blue wire (PWM Signal)
    let mut pwm = Pwm::new_output_a(p.PWM_SLICE1, p.PIN_18, {
        let mut c = Config::default();
        let pwm_freq = 25_000; // Hz, our desired frequency
        let clock_freq = embassy_rp::clocks::clk_sys_freq();
        c.top = (clock_freq / pwm_freq) as u16 - 1;
        c
    });
    pwm.set_duty_cycle_percent(DUTY_START).unwrap();

    {
        *PWM.lock().await = Some(pwm);
    }

    spawner.spawn(button_duty(p.PIN_4.into(), down)).unwrap();
    spawner.spawn(button_duty(p.PIN_9.into(), up)).unwrap();
}

/// Increases the current duty cycle by a fixed step.
///
/// # Arguments
///
/// * `current_duty` - The current duty cycle percentage (0-100).
///
/// # Returns
///
/// * The new duty cycle percentage, capped at 100.
fn up(current_duty: u8) -> u8 {
    let new_duty = current_duty.saturating_add(DUTY_STEP);
    if new_duty > 100 {
        return 100;
    }
    return new_duty;
}

/// Decreases the current duty cycle by a fixed step, without going below 0.
///
/// # Arguments
///
/// * `current_duty` - The current duty cycle percentage (0-100).
///
/// # Returns
///
/// * The new duty cycle percentage, capped at 0.
fn down(current_duty: u8) -> u8 {
    current_duty.saturating_sub(DUTY_STEP)
}

/// Adjusts the duty cycle of a PWM signal based on button input.
///
/// This asynchronous task listens for button presses on the specified input pin.
/// When a button press is detected, it applies the provided action to modify the
/// current duty cycle percentage. The new duty cycle is then set on the PWM output.
///
/// # Arguments
///
/// * `input_pin` - The GPIO pin configured as an input to detect button presses.
/// * `action` - A function that takes the current duty cycle percentage and returns the new duty cycle.
///
/// The task runs continuously, waiting for the button press (low signal) and release (high signal) events.
#[embassy_executor::task(pool_size = 2)]
async fn button_duty(input_pin: AnyPin, action: fn(u8) -> u8) {
    let mut p = Input::new(input_pin, Pull::Up);
    p.set_schmitt(true);

    loop {
        p.wait_for_falling_edge().await;

        let dc = action(DUTY_CYCLE.load(Ordering::Acquire));
        DUTY_CYCLE.store(dc, Ordering::Release);

        info!("Setting PWM to {}%", dc);
        {
            PWM.lock()
                .await
                .as_mut()
                .unwrap()
                .set_duty_cycle_percent(dc)
                .unwrap();
        }
    }
}
