use core::sync::atomic::Ordering;

use defmt::info;
use embassy_rp::gpio::{AnyPin, Input, Pull};
use embassy_rp::pwm::SetDutyCycle;

use crate::{DUTY_CYCLE, PWM};

const DUTY_STEP: u8 = 10; // percent

/// Increase the PWM duty cycle by 10% when the input pin goes low.
#[embassy_executor::task]
pub async fn btn_duty_up(input_pin: AnyPin) {
    button_duty(input_pin, up).await;
}

/// Decrease the PWM duty cycle by 10% when the input pin goes low.
#[embassy_executor::task]
pub async fn btn_duty_down(input_pin: AnyPin) {
    button_duty(input_pin, down).await;
}

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

fn up(current_duty: u8) -> u8 {
    let new_duty = current_duty.saturating_add(DUTY_STEP);
    if new_duty > 100 {
        return 100;
    }
    return new_duty;
}

fn down(current_duty: u8) -> u8 {
    current_duty.saturating_sub(DUTY_STEP)
}
